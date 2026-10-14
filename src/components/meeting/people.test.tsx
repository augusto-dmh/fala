// Prova dos checks de UI de Pessoas (`meeting-people-series`, PRs 1 a 3). Rode com
// `bun src/components/meeting/people.test.tsx`: imprime `<check> ok` e sai com erro na primeira
// falha. O `t` lê as chaves em pt, então toda string visível passa por elas.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingPerson, MeetingSummary } from "@/bindings";
import type { Translate } from "./meetingModel";
import { filterPeople, MeetingsTabs, PeopleView } from "./PeopleView";
import { PersonView } from "./PersonView";
import { PersonEdit } from "./PersonEdit";
import { SuggestedNames, suggestNames } from "./nameSuggest";
import { SeriesLine } from "./SessionHeader";

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

// P5: "⋯" on each row; its menu offers "Juntar com…" and "Renomear".
{
  const html = renderToStaticMarkup(
    <PeopleView
      people={people}
      query=""
      onOpen={noop}
      onRename={noop}
      locale="pt-BR"
      t={tr}
    />,
  );
  assert.ok(html.includes('aria-label="Ações de João"'), html);
  const menu = renderToStaticMarkup(
    <PersonEdit
      person={people[1]}
      people={people}
      onRename={noop}
      onClose={noop}
      t={tr}
    />,
  );
  assert.ok(menu.includes("Juntar com…") && menu.includes("Renomear"), menu);
  ok("P5");
}

// P6: merging names both spellings and how many meetings change before it runs.
{
  const ana = person({ key: "ana", name: "Ana", meetings: 4 });
  const html = renderToStaticMarkup(
    <PersonEdit
      person={ana}
      people={[ana, person()]}
      onRename={noop}
      onClose={noop}
      start={{ kind: "merge", into: person() }}
      t={tr}
    />,
  );
  assert.ok(
    html.includes("Juntar “Ana” com “Ana Souza”? O nome muda em 4 reuniões."),
    html,
  );
  const rename = renderToStaticMarkup(
    <PersonEdit
      person={ana}
      people={[ana]}
      onRename={noop}
      onClose={noop}
      start={{ kind: "rename" }}
      t={tr}
    />,
  );
  assert.ok(rename.includes('aria-label="Novo nome"'), rename);
  assert.ok(rename.includes('value="Ana"'), rename);
  // A new name that is someone else's asks like a merge.
  const old = person({ key: "ana  souza", name: "Ana  Souza", meetings: 2 });
  const clash = renderToStaticMarkup(
    <PersonEdit
      person={old}
      people={[old, person()]}
      onRename={noop}
      onClose={noop}
      start={{ kind: "rename" }}
      t={tr}
    />,
  );
  assert.ok(
    clash.includes("O nome muda em 2 reuniões.") && clash.includes(">Juntar<"),
    clash,
  );
  assert.ok(page.includes("commands.renameMeetingPerson"), "a página renomeia");
  ok("P6");
}

// P7: suggestions start with what was typed, without accents, latest first, at most five,
// without the names already there or the one typed in full.
{
  const many = [
    "Ana Souza",
    "Ana Lima",
    "Anabel",
    "Ána Paula",
    "Ananias",
    "Anita",
  ].map((name) => person({ key: name.toLowerCase(), name }));
  assert.deepEqual(suggestNames(many, "ana"), [
    "Ana Souza",
    "Ana Lima",
    "Anabel",
    "Ána Paula",
    "Ananias",
  ]);
  assert.deepEqual(suggestNames(many, "ÁNA L"), ["Ana Lima"]);
  assert.deepEqual(suggestNames(many, "ana", ["ana souza", "Anabel"]), [
    "Ana Lima",
    "Ána Paula",
    "Ananias",
  ]);
  assert.deepEqual(suggestNames(many, "Anita"), []);
  assert.deepEqual(suggestNames(many, "  "), []);
  assert.deepEqual(suggestNames(many, "lima"), []);
  ok("P7");
}

// P8: the list under the field, the picked one selected; "Quem estava" and "Pessoa N" use it.
{
  const html = renderToStaticMarkup(
    <SuggestedNames
      names={["Ana Souza", "Ana Lima"]}
      picked={1}
      onPick={noop}
    />,
  );
  assert.ok(html.includes('role="listbox"'), html);
  assert.match(html, /aria-selected="true"[^>]*>Ana Lima</);
  assert.ok(read("./MeetingPeoplePanel.tsx").includes("useNameSuggest("));
  assert.ok(read("./TranscriptView.tsx").includes("useNameSuggest("));
  ok("P8");
}

// P9: "Mesma série:" with the earlier meetings by day; "ver todas (N)" only when there are more.
{
  const earlier = [session("B", "Weekly 13/10"), session("C", "Weekly 06/10")];
  const series = { key: "weekly", name: "Weekly", earlier, total: 3 };
  const line = (total: number) =>
    renderToStaticMarkup(
      <SeriesLine
        series={{ ...series, total }}
        onOpen={noop}
        onAll={noop}
        locale="pt-BR"
        t={tr}
        now={now}
      />,
    );
  assert.ok(line(3).includes("Mesma série:"), line(3));
  assert.equal(line(3).match(/title="Weekly/g)?.length, 2, line(3));
  assert.ok(!line(3).includes("ver todas"), line(3));
  assert.ok(line(3).includes("Ontem 14:02"), "o mesmo dia leva a hora");
  assert.ok(line(12).includes("ver todas (12)"), line(12));
  const none = (
    <SeriesLine
      series={null}
      onOpen={noop}
      onAll={noop}
      locale="pt-BR"
      t={tr}
    />
  );
  assert.equal(renderToStaticMarkup(none), "");
  assert.ok(read("./MeetingView.tsx").includes("<SeriesLine"));
  assert.ok(
    page.includes("commands.seriesMeetings"),
    "ver todas filtra a lista",
  );
  assert.ok(page.includes("meeting.series.filter"));
  ok("P9");
}
