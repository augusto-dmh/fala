import type { DictionaryReplacement } from "@/bindings";
import {
  LIMITS,
  normalizeSpoken,
  normalizeTerm,
  type Entry,
} from "./dictionaryModel";

/** Entry lines a file may have; above it nothing is imported. */
export const MAX_LINES = 5000;
/** Characters a field may have; above it the file is not a word list and nothing is imported. */
export const MAX_FIELD = 200;

/** Why a line is left out. */
export type InvalidReason = "fields" | "quote" | "empty" | "tooLong";

/** A new entry the file brings, in file order. */
export type ImportEntry =
  | { kind: "term"; text: string }
  | { kind: "replacement"; spoken: string; written: string };

export interface ImportPreview {
  /** Why nothing can be imported: too many entry lines, or a field over `MAX_FIELD`. */
  error:
    | { kind: "lines"; count: number }
    | { kind: "field"; line: number }
    | null;
  /** The entries not in the dictionary yet, nor earlier in the file. */
  added: ImportEntry[];
  /** Entries already in the dictionary or earlier in the file, ignoring case. */
  duplicates: number;
  /** Each line left out, with its number in the file (from 1). */
  invalid: { line: number; reason: InvalidReason }[];
}

const SEPARATORS = ["\t", ";", ","] as const;

/** The headers a list exported elsewhere starts with. */
const HEADERS = new Set([
  "term",
  "terms",
  "termo",
  "termos",
  "spoken,written",
  "falado,escrito",
]);

/**
 * The fields of a line, with simple CSV quotes: a field may be wrapped in `"`, where the
 * separator is text and `""` is a quote. `null` when a quote is left open or text follows it.
 * Trailing empty fields (`term,`) are dropped.
 */
export function splitFields(line: string, separator: string): string[] | null {
  const fields: string[] = [];
  let start = 0;
  for (;;) {
    let at = start;
    while (line[at] === " ") at += 1;
    if (line[at] !== '"') {
      const end = line.indexOf(separator, start);
      fields.push(line.slice(start, end < 0 ? undefined : end));
      if (end < 0) break;
      start = end + separator.length;
      continue;
    }
    let value = "";
    at += 1;
    for (;;) {
      const quote = line.indexOf('"', at);
      if (quote < 0) return null;
      value += line.slice(at, quote);
      at = quote + 1;
      if (line[at] !== '"') break;
      value += '"';
      at += 1;
    }
    while (line[at] === " ") at += 1;
    fields.push(value);
    if (at >= line.length) break;
    if (!line.startsWith(separator, at)) return null;
    start = at + separator.length;
  }
  while (fields.length > 1 && fields[fields.length - 1].trim() === "")
    fields.pop();
  return fields;
}

/** The file's separator: the one that splits the most lines (tab, then `;`, then `,` on a tie). */
export function pickSeparator(lines: readonly string[]): string {
  let best: string = ",";
  let most = 0;
  for (const separator of SEPARATORS) {
    const count = lines.filter(
      (line) => (splitFields(line, separator)?.length ?? 0) > 1,
    ).length;
    if (count > most) {
      best = separator;
      most = count;
    }
  }
  return best;
}

/**
 * What importing `text` adds to the dictionary: one entry per line, a term or a spoken →
 * written pair. Blank lines, `#` comments and a header first line are skipped.
 */
export function previewImport(
  text: string,
  entries: readonly Entry[],
): ImportPreview {
  const preview: ImportPreview = {
    error: null,
    added: [],
    duplicates: 0,
    invalid: [],
  };
  const lines = text
    // A BOM, when the text was decoded without dropping it.
    .replace(/^\uFEFF/, "")
    .split(/\r?\n/)
    .map((line, index) => ({ line, number: index + 1 }))
    .filter(({ line }) => line.trim() !== "" && !line.trim().startsWith("#"));
  if (lines.length > MAX_LINES) {
    preview.error = { kind: "lines", count: lines.length };
    return preview;
  }
  const separator = pickSeparator(lines.map(({ line }) => line));
  const terms = new Set<string>();
  const spokens = new Set<string>();
  for (const entry of entries) {
    if (entry.kind === "term") terms.add(entry.text.toLowerCase());
    if (entry.kind === "replacement") spokens.add(entry.spoken.toLowerCase());
  }

  lines.forEach(({ line, number }, index) => {
    const fields = splitFields(line, separator);
    if (fields === null) {
      preview.invalid.push({ line: number, reason: "quote" });
      return;
    }
    if (fields.some((field) => field.length > MAX_FIELD)) {
      preview.error ??= { kind: "field", line: number };
      return;
    }
    const header = fields.map((field) => field.trim().toLowerCase()).join(",");
    if (index === 0 && HEADERS.has(header)) return;
    const invalid = (reason: InvalidReason) =>
      preview.invalid.push({ line: number, reason });

    if (fields.length > 2) return invalid("fields");
    if (fields.length === 1) {
      const term = normalizeTerm(fields[0]);
      if (!term) return invalid("empty");
      if (term.length > LIMITS.term.maxSpoken) return invalid("tooLong");
      const key = term.toLowerCase();
      if (terms.has(key)) {
        preview.duplicates += 1;
        return;
      }
      terms.add(key);
      preview.added.push({ kind: "term", text: term });
      return;
    }
    const spoken = normalizeSpoken(fields[0]);
    const written = fields[1].trim();
    if (!spoken || !written) return invalid("empty");
    if (
      spoken.length > LIMITS.replacement.maxSpoken ||
      written.length > LIMITS.replacement.maxWritten
    )
      return invalid("tooLong");
    const key = spoken.toLowerCase();
    if (spokens.has(key)) {
      preview.duplicates += 1;
      return;
    }
    spokens.add(key);
    preview.added.push({ kind: "replacement", spoken, written });
  });
  if (preview.error) preview.added = [];
  return preview;
}

/** The lists to store after the import: the stored ones, then the new entries; `null` = unchanged. */
export function importedLists(
  preview: ImportPreview,
  terms: readonly string[],
  replacements: readonly DictionaryReplacement[],
): {
  terms: string[] | null;
  replacements: DictionaryReplacement[] | null;
} {
  const newTerms: string[] = [];
  const newPairs: DictionaryReplacement[] = [];
  for (const entry of preview.added) {
    if (entry.kind === "term") newTerms.push(entry.text);
    else
      newPairs.push({ spoken: entry.spoken, written: entry.written, uses: 0 });
  }
  return {
    terms: newTerms.length > 0 ? [...terms, ...newTerms] : null,
    replacements: newPairs.length > 0 ? [...replacements, ...newPairs] : null,
  };
}
