import type { MeetingLine } from "@/bindings";
import { speakerOf, type Translate } from "./meetingModel";

/** Lower case without accents: "Reunião" and "reuniao" match. */
export function fold(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

/** The transcript lines whose text has the query, ignoring case and accents; all when blank. */
export function searchLines(
  lines: MeetingLine[],
  query: string,
): MeetingLine[] {
  const needle = fold(query.trim());
  if (!needle) return lines;
  return lines.filter((line) => fold(line.text).includes(needle));
}

/** A piece of a line: `hit` marks a match of the query. */
export interface Piece {
  text: string;
  hit: boolean;
}

/**
 * The line cut around each match of the query, in the original spelling. Folding keeps one
 * character per character for the Latin letters a transcript has, so the offsets line up; when
 * they do not (a letter that folds to two), the line comes back whole.
 */
export function highlight(text: string, query: string): Piece[] {
  const needle = fold(query.trim());
  if (!needle) return [{ text, hit: false }];
  const folded = fold(text);
  if (folded.length !== text.length) return [{ text, hit: false }];
  const pieces: Piece[] = [];
  let from = 0;
  for (
    let at = folded.indexOf(needle);
    at !== -1;
    at = folded.indexOf(needle, from)
  ) {
    if (at > from) pieces.push({ text: text.slice(from, at), hit: false });
    pieces.push({ text: text.slice(at, at + needle.length), hit: true });
    from = at + needle.length;
  }
  if (from < text.length) pieces.push({ text: text.slice(from), hit: false });
  return pieces;
}

/**
 * "Copy transcript": one paragraph per turn, `Name: text`, without times (as Granola copies it).
 * Consecutive lines of the same speaker join in one turn.
 */
export function transcriptText(lines: MeetingLine[], t: Translate): string {
  const turns: string[] = [];
  let last: MeetingLine | null = null;
  for (const line of lines) {
    // A turn changes when the speaker does, as in the transcript view.
    if (last !== null && last.person === line.person) {
      turns[turns.length - 1] += ` ${line.text}`;
    } else {
      turns.push(`${speakerOf(line, t)}: ${line.text}`);
    }
    last = line;
  }
  return turns.join("\n\n");
}
