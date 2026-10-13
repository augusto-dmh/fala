// Prova do início de Reuniões (`.specs/features/meetings-ui-redesign/checks.md`: C2–C5).
// Rode com `bun src/components/meeting/home.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha. Sem instância do i18next, `t` devolve a chave.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingStatus, MeetingSummary } from "@/bindings";
import { MeetingList } from "./MeetingList";
import { NewMeetingButton, RecordingStrip } from "./MeetingHome";
import {
  dayLabel,
  groupMeetingsByDay,
  rowState,
  type LiveSessions,
} from "./meetingDays";
import type { Translate } from "./meetingModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const page = read("./MeetingsPage.tsx");

function ok(id: string) {
  console.log(`${id} ok`);
}

const keyOnly: Translate = (key, options) =>
  options?.title === undefined ? key : `${key}(${String(options.title)})`;
const noop = () => {};

/** A session that started at this local time (the list shows local days). */
const at = (
  id: string,
  local: Date,
  over: Partial<MeetingSummary> = {},
): MeetingSummary => ({
  id,
  title: id,
  mode: "meeting",
  created_at: local.toISOString(),
  started_at: local.toISOString(),
  ended_at: local.toISOString(),
  recorded_ms: 1_830_000,
  audio_retained: true,
  transcribed: true,
  has_notes: false,
  ...over,
});

const now = new Date(2026, 9, 12, 18, 0);
const OPTIONS = "opções";

// C2: por dia local, o início mais recente primeiro; "Hoje", "Ontem" e a data.
{
  // Na ordem de criação, como `list_meetings`: o rascunho de segunda gravado hoje vem depois.
  const sessions = [
    at("ontem", new Date(2026, 9, 11, 23, 55)),
    at("hoje-cedo", new Date(2026, 9, 12, 0, 5)),
    at("segunda", new Date(2026, 9, 5, 9, 30)),
    // Um rascunho conta pelo dia em que foi criado.
    at("rascunho", new Date(2026, 9, 5, 8, 0), { started_at: null }),
    at("ano-passado", new Date(2025, 9, 6, 9, 0)),
    at("pauta-gravada-hoje", new Date(2026, 9, 12, 15, 0), {
      created_at: new Date(2026, 9, 5, 7, 0).toISOString(),
    }),
  ];
  const days = groupMeetingsByDay(sessions);
  const labels = days.map((d) => dayLabel(d.date, now, "pt-BR", keyOnly));
  assert.deepEqual(
    days.map((d, i) => [labels[i], d.sessions.map((s) => s.id)]),
    [
      ["meeting.home.today", ["pauta-gravada-hoje", "hoje-cedo"]],
      ["meeting.home.yesterday", ["ontem"]],
      ["seg., 5 de out.", ["segunda", "rascunho"]],
      [labels[3], ["ano-passado"]],
    ],
  );
  assert.ok(labels[3].includes("2025"), labels[3]);
  assert.equal(dayLabel(days[2].date, now, "en", keyOnly), "Mon, Oct 5");

  // Virada de mês: em 1º de novembro, ontem é 31 de outubro.
  assert.equal(
    dayLabel(
      new Date(2026, 9, 31, 22, 0),
      new Date(2026, 10, 1, 9, 0),
      "pt-BR",
      keyOnly,
    ),
    "meeting.home.yesterday",
  );
  ok("C2");
}

// C3: cada linha com hora, título, duração (ou rascunho) e o ponto de estado.
{
  const live: LiveSessions = {
    recording: "rec",
    working: new Set(["work"]),
    failed: new Set(["fail", "work"]),
  };
  const s = (id: string, over: Partial<MeetingSummary> = {}) =>
    at(id, new Date(2026, 9, 12, 10, 0), over);
  assert.equal(rowState(s("rec", { has_notes: true }), live), "recording");
  assert.equal(rowState(s("work"), live), "working");
  assert.equal(rowState(s("fail", { has_notes: true }), live), "failed");
  assert.equal(rowState(s("done", { has_notes: true }), live), "ready");
  assert.equal(rowState(s("raw"), live), "idle");

  const html = renderToStaticMarkup(
    <MeetingList
      sessions={[
        s("rec"),
        s("done", { has_notes: true }),
        at("draft", new Date(2026, 9, 11, 9, 0), { started_at: null }),
      ]}
      live={live}
      now={now}
      onOpen={noop}
    />,
  );
  assert.ok(html.indexOf("meeting.home.today") < html.indexOf(">rec<"), html);
  assert.ok(html.indexOf(">done<") < html.indexOf("meeting.home.yesterday"));
  assert.ok(html.includes("30:30"), "duração");
  assert.ok(html.includes("meeting.list.draft"));
  assert.ok(/bg-rec[^>]*title="meeting\.home\.state\.recording"/.test(html));
  assert.ok(html.includes('class="sr-only">meeting.home.state.ready'));
  assert.ok(/\d\d:\d\d<\/span>/.test(html), "hora local");
  ok("C3");
}

// C4: sem reuniões, o convite a gravar ou arrastar; buscando, sem cabeçalhos de dia.
{
  const empty = renderToStaticMarkup(
    <MeetingList sessions={[]} onOpen={noop} />,
  );
  assert.ok(empty.includes("meeting.list.empty"));
  assert.ok(empty.includes("meeting.list.emptyHint"));
  const found = renderToStaticMarkup(
    <MeetingList
      sessions={[at("a", new Date(2026, 9, 12, 10, 0))]}
      snippets={new Map([["a", "x \u0002y\u0003"]])}
      now={now}
      onOpen={noop}
    />,
  );
  assert.ok(!found.includes("<h2"), found);
  assert.ok(found.includes("meeting.home.today") && found.includes("<mark>y"));
  ok("C4");
}

// C5: a página é uma coluna sem SettingsGroup; "Nova reunião" grava e abre a sessão; a faixa
// de gravação; o órfão em âmbar; a sessão no lugar da lista, com a volta.
{
  assert.ok(!page.includes("SettingsGroup"));
  assert.ok(page.includes('className="mx-auto w-full max-w-2xl space-y-4"'));
  assert.match(
    page,
    /<NewMeetingButton[\s\S]*onStart=\{\(\) => void start\(null\)\}/,
  );
  assert.match(
    page,
    /setTitle\(""\);\s*\/\/[^\n]*\n\s*if \(result\.data\.id !== null\) await open\(result\.data\.id\);/,
  );
  assert.match(
    page,
    /if \(selected\) \{[\s\S]*onClick=\{\(\) => setSelected\(null\)\}/,
  );
  // A run that ends reloads only the session still open: never back over the list.
  assert.match(
    page,
    /current\?\.summary\.id === id \|\| \(orHome && current === null\)/,
  );
  assert.equal(
    page.match(/await open\(/g)?.length,
    3,
    "só ações da pessoa abrem",
  );
  // Gravando, o gravador; parando ou processando, só a linha de processamento.
  assert.match(page, /\{recordingNow \? \(\s*recorder\s*\) : \(/);
  assert.ok(read("./RecoverBanner.tsx").includes("border-warn bg-warn/10"));

  const button = renderToStaticMarkup(
    <NewMeetingButton
      disabled={false}
      onStart={noop}
      options={<p>{OPTIONS}</p>}
      t={keyOnly}
    />,
  );
  assert.ok(button.includes("meeting.home.new"));
  assert.ok(button.includes('aria-expanded="false"'));
  assert.ok(!button.includes("opções"), "as opções só abrem pela seta");

  const status = (over: Partial<MeetingStatus>): MeetingStatus => ({
    state: "recording",
    id: "rec",
    mode: "meeting",
    recorded_ms: 754_000,
    mic_level: 0,
    system_level: 0,
    muted_mic: false,
    muted_system: false,
    cap_remaining_ms: null,
    low_disk_bytes: null,
    system_output: null,
    call_output: null,
    zero_mic: false,
    zero_system: false,
    dictating: false,
    ...over,
  });
  const strip = (s: MeetingStatus) =>
    renderToStaticMarkup(
      <RecordingStrip status={s} title="Daily" onOpen={noop} t={keyOnly} />,
    );
  assert.equal(strip(status({ state: "idle", id: null })), "");
  assert.equal(strip(status({ state: "processing" })), "");
  const recording = strip(status({}));
  assert.ok(recording.includes("meeting.home.recording(Daily)"));
  assert.ok(
    recording.includes("12:34") && recording.includes("meeting.home.open"),
  );
  assert.ok(!recording.includes("meeting.home.attention"));
  assert.ok(strip(status({ state: "paused" })).includes("meeting.home.paused"));
  assert.ok(
    strip(status({ muted_system: true })).includes("meeting.home.attention"),
  );
  ok("C5");
}
