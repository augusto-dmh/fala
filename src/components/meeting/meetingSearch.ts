import type { MeetingSearchHit, MeetingSummary } from "@/bindings";

/** Around each found term in a search snippet (`fala-storage`'s `MATCH_START`/`MATCH_END`). */
const MATCH_START = "\u0002";
const MATCH_END = "\u0003";

/** How long after the last keystroke the search runs. */
export const SEARCH_AFTER_MS = 200;

export interface SnippetPart {
  text: string;
  match: boolean;
}

/** The snippet as plain text parts, the found terms marked; never parsed as HTML. */
export function snippetParts(snippet: string): SnippetPart[] {
  const parts: SnippetPart[] = [];
  for (const piece of snippet.split(MATCH_START)) {
    const end = piece.indexOf(MATCH_END);
    if (end < 0) {
      if (piece) parts.push({ text: piece, match: false });
      continue;
    }
    parts.push({ text: piece.slice(0, end), match: true });
    const rest = piece.slice(end + 1);
    if (rest) parts.push({ text: rest, match: false });
  }
  return parts;
}

/** The sessions the search found, in its order, each with its snippet. */
export function searchResults(
  sessions: MeetingSummary[],
  hits: MeetingSearchHit[],
): { sessions: MeetingSummary[]; snippets: Map<string, string> } {
  const byId = new Map(sessions.map((s) => [s.id, s]));
  const found = hits.flatMap((hit) => {
    const session = byId.get(hit.id);
    return session ? [session] : [];
  });
  return {
    sessions: found,
    snippets: new Map(hits.map((hit) => [hit.id, hit.snippet])),
  };
}
