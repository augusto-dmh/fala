import type { MeetingLine } from "@/bindings";

/** End of every line the notes LLM wrote (`crates/notes/src/render.rs`). */
export const GENERATED_MARKER = "<!-- fala:ia -->";

/** Where a generated line came from: a transcript segment or a typed note line. */
export type NoteSource =
  | { kind: "segment"; seq: number }
  | { kind: "annotation"; n: number };

export type NoteBlock =
  | { kind: "heading"; level: number; text: string }
  | {
      kind: "line";
      text: string;
      bullet: boolean;
      /** Written by the LLM; typed lines are `false`. */
      generated: boolean;
      sources: NoteSource[];
      /** A code-block or table line: shown as written, no inline Markdown. */
      literal?: true;
    };

export interface ParsedNotes {
  blocks: NoteBlock[];
  /** Typed note lines by their `^aN` number. */
  annotations: Map<number, string>;
}

/** `[[#^s12|01:05]]` or `[[#^a2|a2]]`, Obsidian block links. */
const POINTER = /\s*\[\[#\^([sa])(\d+)\|[^\]]*\]\]/g;
/** The block anchor at the end of a typed line: `texto ^a2`. */
const ANCHOR = /\s+\^a(\d+)$/;
const HEADING = /^(#{1,6})\s+(.*)$/;

/**
 * Reads the stored notes Markdown into headings and lines, with the pointers taken out of the
 * text. A pointer to a segment that is not in `lines` (or to a typed line that is not in the
 * notes) is dropped, as the notes crate drops an unknown id when it renders.
 */
export function parseNotes(
  markdown: string,
  lines: MeetingLine[],
): ParsedNotes {
  const seqs = new Set(lines.map((line) => line.seq));
  const annotations = new Map<number, string>();
  const blocks: NoteBlock[] = [];
  let fenced = false;

  for (const raw of markdown.split("\n")) {
    const row = raw.trim();
    if (!row) continue;
    // Code blocks and table rows (an imported summary keeps them as written).
    const fence = row.startsWith("```") || row.startsWith("~~~");
    if (fence || fenced || row.startsWith("|")) {
      if (fence) fenced = !fenced;
      blocks.push({
        kind: "line",
        text: row,
        bullet: false,
        generated: false,
        sources: [],
        literal: true,
      });
      continue;
    }
    const heading = HEADING.exec(row);
    if (heading) {
      // An imported summary marks its headings too ("### Decisões <!-- fala:ia -->").
      const text = heading[2].endsWith(GENERATED_MARKER)
        ? heading[2].slice(0, -GENERATED_MARKER.length).trim()
        : heading[2];
      blocks.push({ kind: "heading", level: heading[1].length, text });
      continue;
    }
    const generated = row.endsWith(GENERATED_MARKER);
    let text = generated ? row.slice(0, -GENERATED_MARKER.length) : row;
    const sources: NoteSource[] = [];
    if (generated) {
      for (const match of text.matchAll(POINTER)) {
        const id = Number(match[2]);
        sources.push(
          match[1] === "s"
            ? { kind: "segment", seq: id }
            : { kind: "annotation", n: id },
        );
      }
      text = text.replace(POINTER, "");
    }
    text = text.trim();
    const bullet = text.startsWith("- ");
    if (bullet) text = text.slice(2).trim();
    if (!generated) {
      const anchor = ANCHOR.exec(text);
      if (anchor) {
        text = text.slice(0, anchor.index);
        annotations.set(Number(anchor[1]), text);
      }
    }
    blocks.push({ kind: "line", text, bullet, generated, sources });
  }

  for (const block of blocks) {
    if (block.kind !== "line") continue;
    block.sources = block.sources.filter((source) =>
      source.kind === "segment"
        ? seqs.has(source.seq)
        : annotations.has(source.n),
    );
  }
  return { blocks, annotations };
}

/** A run of a note line: plain, **strong** or _emphasised_. */
export interface InlinePiece {
  text: string;
  strong?: boolean;
  em?: boolean;
}

// `**x**`, `*x*` and `_x_`. The markers hug their text and stand apart from words, so
// `snake_case`, `5*3*2` and `2 ** 3` stay as typed.
const INLINE =
  /(?<![\p{L}\p{N}*])\*\*(?!\s)(.+?)(?<!\s)\*\*(?![\p{L}\p{N}*])|(?<![\p{L}\p{N}*])\*(?![\s*])(.+?)(?<![\s*])\*(?![\p{L}\p{N}*])|(?<![\p{L}\p{N}_])_(?!\s)(.+?)(?<!\s)_(?![\p{L}\p{N}_])/gu;

/** The inline Markdown of a line (what an imported summary uses) as runs to render. */
export function inlinePieces(text: string): InlinePiece[] {
  const pieces: InlinePiece[] = [];
  let at = 0;
  for (const match of text.matchAll(INLINE)) {
    const start = match.index ?? 0;
    if (start > at) pieces.push({ text: text.slice(at, start) });
    if (match[1] !== undefined) pieces.push({ text: match[1], strong: true });
    else pieces.push({ text: match[2] ?? match[3], em: true });
    at = start + match[0].length;
  }
  if (at < text.length) pieces.push({ text: text.slice(at) });
  return pieces;
}

/**
 * The date of the footer the Granola import writes in pt-BR ("_Importado do Granola em
 * 2026-10-12._", `crates/storage`), so the page can say it in the UI language; `null` for any
 * other line.
 */
export function granolaImportDate(text: string): string | null {
  return (
    /^_Importado do Granola em (\d{4}-\d{2}-\d{2})\._$/.exec(text)?.[1] ?? null
  );
}

/** A stored note line as the person reads it: no marker, no pointers, no `^aN` anchor. */
function visibleLine(raw: string): string {
  const row = raw.trim();
  if (row.endsWith(GENERATED_MARKER)) {
    return row.slice(0, -GENERATED_MARKER.length).replace(POINTER, "").trim();
  }
  if (HEADING.test(row)) return row;
  const anchor = ANCHOR.exec(row);
  return anchor ? row.slice(0, anchor.index) : row;
}

/** The stored notes as the text the editor shows, one line per line. */
export function editableText(markdown: string): string {
  return markdown.split("\n").map(visibleLine).join("\n").trim();
}

/**
 * The edited text back as notes Markdown. A line equal to a line of `base` comes back as it was
 * stored (marker, pointers, anchor); a changed or new line is the person's text, without the
 * marker: the AI colour is only for what the AI wrote.
 */
export function mergeEdit(base: string, edited: string): string {
  const pool = new Map<string, string[]>();
  for (const raw of base.split("\n")) {
    const visible = visibleLine(raw);
    if (!visible) continue;
    pool.set(visible, [...(pool.get(visible) ?? []), raw.trim()]);
  }
  const lines = edited.split("\n").map((line) => {
    const text = line.trim();
    if (!text) return "";
    const stored = pool.get(text)?.shift();
    if (stored !== undefined) return stored;
    return text.endsWith(GENERATED_MARKER)
      ? text.slice(0, -GENERATED_MARKER.length).trim()
      : text;
  });
  return `${lines.join("\n").trim()}\n`;
}
