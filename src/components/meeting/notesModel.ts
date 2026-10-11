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

  for (const raw of markdown.split("\n")) {
    const row = raw.trim();
    if (!row) continue;
    const heading = HEADING.exec(row);
    if (heading) {
      blocks.push({
        kind: "heading",
        level: heading[1].length,
        text: heading[2],
      });
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
