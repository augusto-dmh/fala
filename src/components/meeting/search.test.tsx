// Prova da busca na página Reuniões (`.specs/features/meeting-storage-mirror/checks.md`: C13).
// Rode com `bun src/components/meeting/search.test.tsx`. Sem instância do i18next, `t` devolve a
// chave: a prova confere que toda string visível passa por ela.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingSummary } from "@/bindings";
import { MeetingList } from "./MeetingList";
import { searchResults, snippetParts } from "./meetingSearch";

const session = (id: string, title: string): MeetingSummary => ({
  id,
  title,
  mode: "meeting",
  created_at: "2026-10-02T14:00:00-03:00",
  started_at: null,
  ended_at: null,
  recorded_ms: 0,
  audio_retained: false,
  transcribed: false,
  has_notes: false,
});

assert.deepEqual(snippetParts("a reunião de \u0002orçamento\u0003 ficou…"), [
  { text: "a reunião de ", match: false },
  { text: "orçamento", match: true },
  { text: " ficou…", match: false },
]);
assert.deepEqual(snippetParts("\u0002a\u0003\u0002b\u0003"), [
  { text: "a", match: true },
  { text: "b", match: true },
]);
assert.deepEqual(snippetParts("sem termo"), [
  { text: "sem termo", match: false },
]);

// A ordem é a da busca; uma sessão que a lista ainda não tem fica de fora.
const sessions = [session("A", "Daily"), session("B", "Orçamento")];
const found = searchResults(sessions, [
  { id: "B", snippet: "\u0002Orça\u0003mento" },
  { id: "Z", snippet: "sumiu" },
  { id: "A", snippet: "sexta" },
]);
assert.deepEqual(
  found.sessions.map((s) => s.id),
  ["B", "A"],
);

const html = renderToStaticMarkup(
  <MeetingList {...found} selectedId={null} onOpen={() => {}} />,
);
assert.ok(html.includes("<mark>Orça</mark><span>mento</span>"), html);
// O trecho é texto: nada vira HTML.
const evil = renderToStaticMarkup(
  <MeetingList
    sessions={[session("A", "x")]}
    snippets={new Map([["A", "<img src=x>"]])}
    selectedId={null}
    onOpen={() => {}}
  />,
);
assert.ok(!evil.includes("<img"), evil);
assert.ok(
  renderToStaticMarkup(
    <MeetingList
      sessions={[]}
      snippets={new Map()}
      selectedId={null}
      onOpen={() => {}}
    />,
  ).includes("meeting.list.noResults"),
);

const page = readFileSync(
  new URL("./MeetingsPage.tsx", import.meta.url),
  "utf8",
);
assert.ok(page.includes('placeholder={t("meeting.list.search")}'));
assert.ok(page.includes("commands.searchMeetings(query)"));
console.log("search: all assertions passed");
