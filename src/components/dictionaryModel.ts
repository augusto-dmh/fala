import type { DictionaryReplacement } from "@/bindings";
import { matchesQuery } from "../lib/utils/search";

/** What an entry of the dictionary is: a term to spell, a spoken → written pair or a snippet. */
export type EntryKind = "term" | "replacement" | "snippet";

/** The list's filter: every kind, or one. */
export type KindFilter = "all" | EntryKind;

export const ENTRY_KINDS: readonly EntryKind[] = [
  "term",
  "replacement",
  "snippet",
];

export type Entry =
  | { kind: "term"; key: string; text: string }
  | {
      kind: "replacement" | "snippet";
      key: string;
      spoken: string;
      written: string;
      uses: number;
    };

/** The settings each kind lives in. */
export const SETTING = {
  term: "custom_words",
  replacement: "dictionary_replacements",
  snippet: "dictionary_snippets",
} as const;

/** Limits of each kind, as the backend checks them. */
export const LIMITS = {
  term: { maxSpoken: 50, minWords: 1, maxWritten: 0 },
  replacement: { maxSpoken: 60, minWords: 1, maxWritten: 200 },
  snippet: { maxSpoken: 60, minWords: 2, maxWritten: 4000 },
} as const;

/** A term as the dictionary keeps it: no markup characters, single spaces. */
export const normalizeTerm = (word: string) =>
  word
    .replace(/[<>"']/g, "")
    .replace(/\s+/g, " ")
    .trim();

/** A spoken form as the dictionary keeps it: single spaces. */
export const normalizeSpoken = (spoken: string) =>
  spoken.replace(/\s+/g, " ").trim();

/** Every entry in one list: terms, then pairs, then snippets, each in the order it was added. */
export function entriesOf(
  terms: readonly string[],
  replacements: readonly DictionaryReplacement[],
  snippets: readonly DictionaryReplacement[],
): Entry[] {
  const pairs = (
    kind: "replacement" | "snippet",
    list: readonly DictionaryReplacement[],
  ): Entry[] =>
    list.map((pair) => ({
      kind,
      key: `${kind}:${pair.spoken}`,
      spoken: pair.spoken,
      written: pair.written,
      uses: pair.uses ?? 0,
    }));
  return [
    ...terms.map(
      (text): Entry => ({ kind: "term", key: `term:${text}`, text }),
    ),
    ...pairs("replacement", replacements),
    ...pairs("snippet", snippets),
  ];
}

/** The entries of the filter's kind whose text matches the search (ignoring case and accents). */
export function filterEntries(
  entries: readonly Entry[],
  kind: KindFilter,
  query: string,
): Entry[] {
  return entries.filter(
    (entry) =>
      (kind === "all" || entry.kind === kind) &&
      (entry.kind === "term"
        ? matchesQuery(entry.text, query)
        : matchesQuery(entry.spoken, query) ||
          matchesQuery(entry.written, query)),
  );
}

/** How many entries of each kind, for the filter's labels. */
export function countByKind(
  entries: readonly Entry[],
): Record<KindFilter, number> {
  const counts = { all: entries.length, term: 0, replacement: 0, snippet: 0 };
  for (const entry of entries) counts[entry.kind] += 1;
  return counts;
}

/** A new entry from the form, ready to save, or why it cannot be saved yet. */
export type Draft =
  | { ok: true; kind: EntryKind; spoken: string; written: string }
  | {
      ok: false;
      /** `null` while the form is just incomplete; a message key when it is wrong. */
      reason: "duplicate" | "twoWords" | null;
      spoken: string;
    };

/** Checks a new entry against the kind's limits and the entries already saved. */
export function draftOf(
  kind: EntryKind,
  spokenInput: string,
  writtenInput: string,
  entries: readonly Entry[],
): Draft {
  const limits = LIMITS[kind];
  const spoken =
    kind === "term" ? normalizeTerm(spokenInput) : normalizeSpoken(spokenInput);
  const written = writtenInput.trim();
  const words = spoken ? spoken.split(" ").length : 0;
  if (!spoken || spoken.length > limits.maxSpoken)
    return { ok: false, reason: null, spoken };
  if (words < limits.minWords) return { ok: false, reason: "twoWords", spoken };
  if (kind !== "term" && (!written || written.length > limits.maxWritten))
    return { ok: false, reason: null, spoken };
  const lowered = spoken.toLowerCase();
  const taken = entries.some((entry) =>
    entry.kind !== kind
      ? false
      : (entry.kind === "term" ? entry.text : entry.spoken).toLowerCase() ===
        lowered,
  );
  if (taken) return { ok: false, reason: "duplicate", spoken };
  return { ok: true, kind, spoken, written };
}
