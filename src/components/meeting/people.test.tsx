// Prova dos checks de UI de Pessoas (`meeting-people-series`, PR 1). Rode com
// `bun src/components/meeting/people.test.tsx`: imprime `<check> ok` e sai com erro na primeira
// falha. O `t` lê as chaves em pt, então toda string visível passa por elas.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingPerson, MeetingSummary } from "@/bindings";
import type { Translate } from "./meetingModel";
import { filterPeople, MeetingsTabs, PeopleView } from "./PeopleView";
import { PersonView } from "./PersonView";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const page = read("./MeetingsPage.tsx");

const ok = (id: string) => console.log(`${id} ok`);

// With `count`, the pt plural form; `{{name}}` filled from the options.
const tr: Translate = (key, options = {}) => {
  const count = options.count as number | undefined;
  const path =
    count === undefined ? key : `${key}_${count === 1 ? "one" : "other"}`;
  const value = path.split(".").reduce<any>((node, part) => node?.[part], pt);
  assert.equal(typeof value, "string", `chave sem texto em pt: ${path}`);
  return value.replace(/\{\{(\w+)\}\}/g, (_: string, name: string) =>
    String(options[name]),
  );
};

const noop = () => {};
const now = new Date("2026-10-13T12:00:00-03:00");

const person = (over: Partial<MeetingPerson> = {}): MeetingPerson => ({
  key: "ana souza",
  name: "Ana Souza",
  last_meeting_at: "2026-10-13T09:05:00-03:00",
  meetings: 3,
  ...over,
});

const session = (id: string, title: string): MeetingSummary => ({
  id,
  title,
  mode: "meeting",
  created_at: "2026-10-12T14:00:00-03:00",
  started_at: "2026-10-12T14:02:00-03:00",
  ended_at: "2026-10-12T14:40:00-03:00",
  recorded_ms: 2_280_000,
  audio_retained: false,
  transcribed: true,
  has_notes: true,
});

const people = [
  person(),
  person({
    key: "joao",
    name: "João",
    last_meeting_at: "2026-10-06T10:00:00-03:00",
    meetings: 1,
  }),
];

// P1: "Reuniões | Pessoas" beside the search; the pick is pressed.
{
  const html = renderToStaticMarkup(
    <MeetingsTabs tab="people" onTab={noop} t={tr} />,
  );
  assert.ok(html.includes('aria-label="Mostrar"'), html);
  assert.match(html, /aria-pressed="false"[^>]*>Reuniões</);
  assert.match(html, /aria-pressed="true"[^>]*>Pessoas</);
  assert.ok(page.includes("<MeetingsTabs"), "a página mostra o alternador");
  assert.ok(page.includes("meeting.directory.search"), "a busca filtra nomes");
  ok("P1");
}

// P2: the table, latest meeting first, with the day and the count; the box filters names
// without case or accents.
{
  const html = renderToStaticMarkup(
    <PeopleView
      people={people}
      query=""
      onOpen={noop}
      locale="pt-BR"
      t={tr}
      now={now}
    />,
  );
  for (const head of ["Pessoa", "Última reunião", "Reuniões"])
    assert.ok(html.includes(`>${head}</th>`), head);
  assert.ok(html.indexOf("Ana Souza") < html.indexOf("João"), html);
  assert.ok(html.includes(">Hoje<"), html);
  assert.ok(html.includes(">3<"), html);

  assert.deepEqual(
    filterPeople(people, "  JOAO ").map((p) => p.name),
    ["João"],
  );
  assert.deepEqual(
    filterPeople(people, "souz").map((p) => p.name),
    ["Ana Souza"],
  );
  assert.equal(filterPeople(people, "").length, 2);
  ok("P2");
}

// P3: no people yet says where they come from; a search without a match says so.
{
  const empty = renderToStaticMarkup(
    <PeopleView people={[]} query="" onOpen={noop} locale="pt-BR" t={tr} />,
  );
  assert.ok(empty.includes("os nomes de “Quem estava”"), empty);
  const none = renderToStaticMarkup(
    <PeopleView
      people={people}
      query="zé"
      onOpen={noop}
      locale="pt-BR"
      t={tr}
    />,
  );
  assert.ok(none.includes("Nenhuma pessoa encontrada."), none);
  const loading = (
    <PeopleView people={null} query="" onOpen={noop} locale="pt-BR" t={tr} />
  );
  assert.equal(renderToStaticMarkup(loading), "", "nada enquanto carrega");
  ok("P3");
}

// P4: one person: "← Pessoas", the name, the count, and the meetings with them by day.
{
  const html = renderToStaticMarkup(
    <PersonView
      person={person()}
      sessions={[session("A", "1:1 Ana"), session("B", "Weekly produto")]}
      onBack={noop}
      onOpen={noop}
      t={tr}
      now={now}
    />,
  );
  assert.ok(html.includes("Pessoas</button>"), html);
  assert.ok(html.includes(">Ana Souza</h1>"), html);
  assert.ok(html.includes("2 reuniões"), html);
  assert.ok(html.includes("1:1 Ana") && html.includes("Weekly produto"), html);

  const loading = renderToStaticMarkup(
    <PersonView
      person={person()}
      sessions={null}
      onBack={noop}
      onOpen={noop}
      t={tr}
    />,
  );
  assert.ok(loading.includes("3 reuniões"), loading);
  assert.ok(!loading.includes("meeting.list.empty"), loading);
  const gone = renderToStaticMarkup(
    <PersonView
      person={person()}
      sessions={[]}
      onBack={noop}
      onOpen={noop}
      t={tr}
    />,
  );
  assert.ok(gone.includes("Nenhuma reunião com esta pessoa agora."), gone);
  ok("P4");
}
