// Prova da busca na transcrição, do "Copiar transcrição", do aviso e do chip de consentimento, do
// template escolhido antes e das notas depois do parar (`meeting-notes-polish`, PR 3). Rode com
// `bun src/components/meeting/meetingFlow.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingDetail, MeetingLine, MeetingStatus } from "@/bindings";
import { ConsentCard } from "../../overlay/MeetingPill";
import { CONSENT_CHIP_MS, showsConsentChip } from "../../overlay/pillModel";
import { MeetingView } from "./MeetingView";
import type { Translate } from "./meetingModel";
import { RecorderPanel } from "./RecorderPanel";
import { TranscriptView } from "./TranscriptView";
import {
  fold,
  highlight,
  searchLines,
  transcriptText,
} from "./transcriptSearch";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const page = read("./MeetingsPage.tsx");
const viewSource = read("./MeetingView.tsx");
const overlay = read("../../overlay/RecordingOverlay.tsx");
const rust = read("../../../apps/desktop/src/meeting/mod.rs");

function ok(id: string) {
  console.log(`${id} ok`);
}

const tr: Translate = (key, options = {}) => {
  const value = key.split(".").reduce((node, part) => node?.[part], pt);
  assert.equal(typeof value, "string", `chave sem texto em pt: ${key}`);
  return (value as string).replace(/\{\{(\w+)\}\}/g, (_, name) =>
    String(options[name]),
  );
};

const noop = () => {};

const line = (
  seq: number,
  person: number | null,
  text: string,
  name?: string,
): MeetingLine => ({
  seq,
  channel: person === null ? "mic" : "system",
  person,
  ...(name ? { name } : {}),
  t0_ms: seq * 1000,
  t1_ms: seq * 1000 + 900,
  text,
});

const lines = [
  line(1, null, "Bom dia, vamos à Reunião."),
  line(2, 1, "A reuniao de ontem foi longa."),
  line(3, 1, "Concordo."),
  line(4, 2, "Fechamos a data.", "Ana"),
];

// Busca sem acento e sem maiúsculas; o trecho achado vem na grafia original.
{
  assert.equal(fold("Reunião ÀS"), "reuniao as");
  assert.deepEqual(
    searchLines(lines, "REUNIAO").map((l) => l.seq),
    [1, 2],
  );
  assert.deepEqual(searchLines(lines, "  ").length, lines.length);
  assert.deepEqual(highlight("vamos à Reunião.", "reuniao"), [
    { text: "vamos à ", hit: false },
    { text: "Reunião", hit: true },
    { text: ".", hit: false },
  ]);
  const html = renderToStaticMarkup(
    <TranscriptView
      lines={searchLines(lines, "reuniao")}
      query="reuniao"
      audioAvailable
      playing={null}
      highlighted={null}
      onPlay={noop}
      t={tr}
    />,
  );
  assert.equal(html.match(/<mark/g)?.length, 2, html);
  assert.ok(html.includes(">Reunião</mark>"), html);
  assert.ok(!html.includes("Concordo"), html);
  // Cada trecho achado mantém quem fala e a hora, mesmo vizinho de outro da mesma pessoa.
  const spread = renderToStaticMarkup(
    <TranscriptView
      lines={searchLines(lines, "o")}
      query="o"
      audioAvailable
      playing={null}
      highlighted={null}
      onPlay={noop}
      t={tr}
    />,
  );
  assert.equal(spread.match(/Pessoa 1 · /g)?.length, 2, spread);
  // A fonte de uma nota que a busca esconde limpa a busca antes de rolar.
  assert.match(
    viewSource,
    /setQuery\(""\);\s*requestAnimationFrame\(scroll\);/,
  );
  ok("FL-search");
}

// "Copiar transcrição": "Nome: texto" por turno, sem horário.
{
  assert.equal(
    transcriptText(lines, tr),
    [
      "Eu: Bom dia, vamos à Reunião.",
      "Pessoa 1: A reuniao de ontem foi longa. Concordo.",
      "Ana: Fechamos a data.",
    ].join("\n\n"),
  );
  ok("FL-copy");
}

const status = (over: Partial<MeetingStatus>): MeetingStatus => ({
  state: "recording",
  id: "01J9Z3K8M4Q7R2S5T6V7W8X9YZ",
  mode: "meeting",
  recorded_ms: 0,
  mic_level: 0,
  system_level: 0,
  muted_mic: false,
  muted_system: false,
  cap_remaining_ms: null,
  low_disk_bytes: null,
  mic_input: null,
  system_output: null,
  call_output: null,
  zero_mic: false,
  zero_system: false,
  dictating: false,
  ...over,
});

const panel = (s: MeetingStatus) =>
  renderToStaticMarkup(
    <RecorderPanel
      status={s}
      mode="meeting"
      title=""
      outputs={null}
      onModeChange={noop}
      onTitleChange={noop}
      onOutputChange={noop}
      onSwitchOutput={noop}
      onStart={noop}
      onNewDraft={noop}
      onPause={noop}
      onResume={noop}
      onStop={noop}
      onExtend={noop}
      templates={[
        { id: "geral", name: "Reunião geral", purpose: "", source: "builtin" },
        { id: "um-a-um", name: "1:1", purpose: "", source: "builtin" },
      ]}
      template="um-a-um"
      onTemplateChange={noop}
    />,
  );

// Chip de consentimento nos primeiros 5 s de cada início, na página e na pílula; "Copiar aviso"
// antes e durante; o template da gravação no painel.
{
  assert.equal(CONSENT_CHIP_MS, 5000);
  assert.match(rust, /pub\(crate\) const CONSENT_CHIP_MS: u64 = 5_000;/);
  const first = panel(status({ recorded_ms: 4999 }));
  assert.ok(first.includes("meeting.recorder.consentChip"), first);
  assert.ok(!panel(status({ recorded_ms: 5000 })).includes("consentChip"));
  assert.ok(first.includes("meeting.notice.copy"), first);
  assert.match(first, /aria-haspopup="menu"[^>]*title="1:1"/, first);
  assert.ok(
    panel(status({ state: "idle", id: null })).includes("meeting.notice.copy"),
  );
  assert.ok(showsConsentChip(0) && !showsConsentChip(5000));
  assert.ok(
    renderToStaticMarkup(<ConsentCard text="x" />).includes('role="status"'),
  );
  assert.match(overlay, /showsConsentChip\(meetingMs\)/);
  // O aviso não promete que o áudio não sai do computador: a transcrição vai à nuvem (ADR-0005).
  for (const resources of [pt, en]) {
    assert.ok(!/computador|computer/i.test(resources.meeting.notice.text));
  }
  ok("FL-consent");
}

const detail = (over: Partial<MeetingDetail> = {}): MeetingDetail => ({
  summary: {
    id: "01J9Z3K8M4Q7R2S5T6V7W8X9YZ",
    title: "Planejamento",
    mode: "meeting",
    created_at: "2026-10-02T13:50:00-03:00",
    started_at: "2026-10-02T14:02:30-03:00",
    ended_at: "2026-10-02T14:40:00-03:00",
    recorded_ms: 2_250_000,
    audio_retained: true,
    transcribed: true,
    has_notes: false,
  },
  annotations: "",
  lines,
  notes_md: null,
  notes_template: "um-a-um",
  notes_edited: false,
  notes_running: false,
  audio_available: true,
  ...over,
});

const view = (d: MeetingDetail, over: Record<string, unknown> = {}) =>
  renderToStaticMarkup(
    <MeetingView
      detail={d}
      templates={[
        { id: "geral", name: "Reunião geral", purpose: "", source: "builtin" },
        { id: "um-a-um", name: "1:1", purpose: "", source: "builtin" },
      ]}
      recording={false}
      transcribing={false}
      generating={false}
      onRecordDraft={noop}
      onTranscribe={noop}
      onCancelTranscription={noop}
      onGenerate={noop}
      onCopy={noop}
      {...over}
    />,
  );

// O template escolhido vai com a sessão (rascunho inclusive) enquanto ela não tem notas; os
// estados da geração; o erro diz que nada se perdeu e oferece tentar de novo.
{
  const draft = view(
    detail({ summary: { ...detail().summary, started_at: null } }),
  );
  assert.match(draft, /aria-haspopup="menu"[^>]*title="1:1"/, draft);
  assert.match(
    viewSource,
    /if \(detail\.notes_md === null\) \{\s*void commands\.setMeetingTemplate\(detail\.summary\.id, id\);/,
  );
  assert.ok(
    view(detail(), { generating: true }).includes("meeting.view.writing"),
  );
  assert.ok(
    view(detail({ annotations: "decidir data" }), {
      generating: true,
    }).includes("meeting.view.enhancing"),
  );
  const failed = view(detail(), { notesFailed: true });
  assert.ok(failed.includes("meeting.view.notesFailed"), failed);
  assert.ok(failed.includes("meeting.view.retry"), failed);
  assert.ok(!view(detail()).includes("meeting.view.notesFailed"));
  assert.match(viewSource, /transcript=\{detail\.lines\.length > 0\}/);
  assert.ok(view(detail()).includes("meeting.view.search"));
  // A página segue a geração que o parar dispara: "notes" liga o estado, "failed" logo depois
  // de "notes" é falha das notas.
  assert.match(
    page,
    /if \(stage\.stage === "notes"\) \{\s*setGenerating\(id\);/,
  );
  assert.match(page, /stage\.stage === "failed" && before === "notes"/);
  // Uma geração em curso aparece mesmo numa página aberta depois (o backend diz).
  assert.match(
    page,
    /generating === selected\.summary\.id \|\| selected\.notes_running/,
  );
  ok("FL-flow");
}

// Toda string nova existe em pt e en.
{
  const keys = [
    "meeting.notice.copy",
    "meeting.notice.copied",
    "meeting.notice.text",
    "meeting.recorder.consentChip",
    "meeting.view.search",
    "meeting.view.found_one",
    "meeting.view.found_other",
    "meeting.view.copyTranscript",
    "meeting.view.writing",
    "meeting.view.enhancing",
    "meeting.view.notesFailed",
    "meeting.view.retry",
    "meeting.context.autoNotes.title",
    "meeting.context.autoNotes.description",
    "overlay.meetingConsent",
  ];
  for (const key of keys) {
    for (const resources of [pt, en]) {
      const value = key
        .split(".")
        .reduce((node, part) => node?.[part], resources);
      assert.equal(typeof value, "string", key);
    }
  }
  ok("FL-i18n");
}
