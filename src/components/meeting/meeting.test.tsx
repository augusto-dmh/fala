// Prova dos checks da página Reuniões (`.specs/features/meeting-panel/checks.md`: C12, C25, C27,
// C33, C36; `.specs/features/meeting-source-playback/checks.md`: SP*). Rode com `bun src/components/meeting/meeting.test.tsx`: imprime `<check> ok` e sai
// com erro na primeira falha. Sem instância do i18next, `t` devolve a chave: o que aparece na
// tela é a chave, e a prova confere que toda string visível passa por ela.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type {
  MeetingDetail,
  MeetingStatus,
  MeetingSummary,
  MeetingTemplate,
} from "@/bindings";
import { AnnotationSaver, SAVE_AFTER_MS, type Timers } from "./annotationSaver";
import { ConsentNotice } from "./ConsentNotice";
import { KeyField } from "./KeyField";
import { MeetingList } from "./MeetingList";
import { MeetingView } from "./MeetingView";
import { NotesView } from "./NotesView";
import { parseNotes } from "./notesModel";
import { RecorderPanel } from "./RecorderPanel";
import { SourcePopover } from "./SourcePopover";
import { TranscriptView } from "./TranscriptView";
import { ClipPlayer, type ClipDeps } from "./clipPlayer";
import {
  clock,
  errorKey,
  lineText,
  titleOf,
  type Translate,
} from "./meetingModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const page = read("./MeetingsPage.tsx");

function ok(id: string) {
  console.log(`${id} ok`);
}

/** `t` over the pt resources, with `{{x}}` interpolation. */
const tr: Translate = (key, options = {}) => {
  const value = key.split(".").reduce((node, part) => node?.[part], pt);
  assert.equal(typeof value, "string", `chave sem texto em pt: ${key}`);
  return (value as string).replace(/\{\{(\w+)\}\}/g, (_, name) =>
    String(options[name]),
  );
};

const noop = () => {};

const status = (over: Partial<MeetingStatus> = {}): MeetingStatus => ({
  state: "idle",
  id: null,
  mode: null,
  recorded_ms: 0,
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
    />,
  );

const summary = (over: Partial<MeetingSummary> = {}): MeetingSummary => ({
  id: "01J9Z3K8M4Q7R2S5T6V7W8X9YZ",
  title: "",
  mode: "meeting",
  created_at: "2026-10-02T13:50:00-03:00",
  started_at: "2026-10-02T14:02:30-03:00",
  ended_at: "2026-10-02T14:40:00-03:00",
  recorded_ms: 2_250_000,
  audio_retained: true,
  transcribed: true,
  has_notes: false,
  ...over,
});

const templates: MeetingTemplate[] = [
  { id: "geral", name: "Reunião geral" },
  { id: "um-a-um", name: "1:1" },
];

const view = (detail: MeetingDetail, over: Record<string, unknown> = {}) =>
  renderToStaticMarkup(
    <MeetingView
      detail={detail}
      templates={templates}
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

const detail = (over: Partial<MeetingDetail> = {}): MeetingDetail => ({
  summary: summary(),
  annotations: "decidir data",
  lines: [
    {
      seq: 1,
      channel: "system",
      person: 1,
      t0_ms: 65_000,
      t1_ms: 70_000,
      text: "Proponho 15 de novembro.",
    },
  ],
  notes_md: "## Notas · Reunião geral",
  notes_template: "geral",
  notes_edited: false,
  notes_running: false,
  audio_available: true,
  ...over,
});

// C12: o aviso de terceiros, literal da door 2, com os dois botões.
{
  assert.equal(
    pt.meeting.consent.body,
    "Gravar uma reunião guarda no seu computador o áudio do seu microfone e do sistema, inclusive a voz de outras pessoas. Ao transcrever, esse áudio é enviado à ElevenLabs; ao gerar notas, a transcrição e as suas anotações são enviadas à Anthropic. Transcrições e notas contêm dados pessoais de terceiros: avise quem participa e siga as regras do seu trabalho e a LGPD.",
  );
  assert.equal(pt.meeting.consent.accept, "Entendi, gravar");
  assert.equal(pt.meeting.consent.cancel, "Cancelar");
  assert.ok(en.meeting.consent.body.includes("ElevenLabs"));
  const html = renderToStaticMarkup(
    <ConsentNotice onAccept={noop} onCancel={noop} />,
  );
  for (const key of ["body", "accept", "cancel"]) {
    assert.ok(html.includes(`meeting.consent.${key}`), html);
  }
  assert.ok(
    page.includes("await commands.acceptMeetingConsent();") &&
      page.indexOf("await commands.acceptMeetingConsent();") <
        page.indexOf("await start(pendingDraft);"),
    "o aceite é gravado antes de gravar",
  );
  ok("C12");
}

// C25: canal mudo e teto na página.
{
  const recording = status({ state: "recording", recorded_ms: 725_000 });
  assert.ok(panel(recording).includes("12:05"));
  assert.ok(!panel(recording).includes("meeting.recorder.mutedMic"));
  assert.ok(
    panel({ ...recording, muted_mic: true }).includes(
      "meeting.recorder.mutedMic",
    ),
  );
  assert.ok(
    panel({ ...recording, muted_system: true }).includes(
      "meeting.recorder.mutedSystem",
    ),
  );
  ok("C25-muted");
  const capped = panel({ ...recording, cap_remaining_ms: 600_000 });
  assert.ok(capped.includes("meeting.recorder.capWarning"), capped);
  assert.ok(capped.includes("meeting.recorder.extend"), capped);
  assert.ok(!panel(recording).includes("meeting.recorder.extend"));
  assert.ok(
    page.includes("onExtend={() => void act(commands.extendMeetingCap)}"),
  );
  ok("C25-cap");
}

// C27: as anotações vão ao disco 2 s depois da última tecla, de novo no flush, nunca repetidas.
{
  let now = 0;
  let timers: { at: number; fn: () => void; id: number }[] = [];
  let next = 0;
  const fake: Timers = {
    set: (fn, ms) => {
      timers.push({ at: now + ms, fn, id: ++next });
      return next;
    },
    clear: (handle) => {
      timers = timers.filter((x) => x.id !== handle);
    },
  };
  const advance = (ms: number) => {
    now += ms;
    const due = timers.filter((x) => x.at <= now);
    timers = timers.filter((x) => x.at > now);
    due.forEach((x) => x.fn());
  };
  const saved: string[] = [];
  const saver = new AnnotationSaver((text) => saved.push(text), "pauta", fake);
  assert.equal(SAVE_AFTER_MS, 2000);
  saver.change("pauta\na");
  advance(1500);
  saver.change("pauta\nab");
  advance(1999);
  assert.deepEqual(saved, []);
  advance(1);
  assert.deepEqual(saved, ["pauta\nab"]);
  saver.flush();
  assert.deepEqual(saved, ["pauta\nab"], "flush sem mudança não salva");
  saver.change("pauta\nabc");
  saver.flush();
  assert.deepEqual(saved, ["pauta\nab", "pauta\nabc"]);
  advance(5000);
  assert.equal(saved.length, 2, "o timer cancelado não salva de novo");
  saver.change("pauta\nabc");
  advance(2000);
  assert.equal(saved.length, 2, "texto igual ao salvo não salva");
  ok("C27");
}

// C33: "Copiar Markdown" põe no clipboard exatamente o retorno de meeting_markdown.
{
  const copy = page.slice(page.indexOf("const copy = async"));
  assert.ok(
    copy.includes("const result = await commands.meetingMarkdown(id);") &&
      copy.includes(
        'if (result.status === "ok") await copyToClipboard(result.data);',
      ),
    copy.slice(0, 300),
  );
  assert.ok(view(detail()).includes("meeting.view.copy"));
  ok("C33");
}

// C36: estados da página.
{
  const empty = renderToStaticMarkup(
    <MeetingList sessions={[]} selectedId={null} onOpen={noop} />,
  );
  assert.ok(empty.includes("meeting.list.empty"), empty);
  const idle = panel(status());
  for (const key of ["start", "online", "inPerson", "newDraft"]) {
    assert.ok(idle.includes(`meeting.recorder.${key}`), key);
  }
  ok("C36-empty");

  const list = renderToStaticMarkup(
    <MeetingList
      sessions={[
        summary({ id: "B", title: "Planejamento", has_notes: true }),
        summary({ id: "A", started_at: null, transcribed: false }),
      ]}
      selectedId={null}
      onOpen={noop}
    />,
  );
  assert.ok(
    list.indexOf("Planejamento") < list.indexOf("meeting.untitled"),
    list,
  );
  assert.ok(list.includes("2026-10-02 14:02") && list.includes("37:30"), list);
  assert.ok(
    list.includes("meeting.list.notes") && list.includes("meeting.list.draft"),
  );
  assert.equal(titleOf(summary(), tr), "Reunião de 2026-10-02 13:50");
  ok("C36-list");

  const opened = view(detail());
  for (const key of ["transcribe", "generate", "copy", "transcript", "notes"]) {
    assert.ok(opened.includes(`meeting.view.${key}`), key);
  }
  assert.ok(
    opened.includes("Reunião geral") && opened.includes("1:1"),
    "seletor",
  );
  assert.equal(
    lineText(detail().lines[0], tr),
    "[01:05] Pessoa 1: Proponho 15 de novembro.",
  );
  assert.equal(
    lineText({ ...detail().lines[0], person: null, t0_ms: 3_723_000 }, tr),
    "[1:02:03] Eu: Proponho 15 de novembro.",
  );
  assert.equal(clock(0), "00:00");
  const draft = view(detail({ summary: summary({ started_at: null }) }));
  assert.ok(
    draft.includes("meeting.view.recordDraft") &&
      draft.includes("meeting.view.agenda"),
  );
  assert.ok(!draft.includes("meeting.view.transcribe"));
  ok("C36-detail");

  assert.equal(errorKey({ kind: "missing_key" }), "meeting.errors.missing_key");
  assert.equal(
    errorKey({ kind: "audio_device", detail: "x" }),
    "meeting.errors.audio_device",
  );
  assert.equal(errorKey("texto livre"), "meeting.errors.generic");
  for (const kind of Object.keys(pt.meeting.errors)) {
    assert.ok(en.meeting.errors[kind], `errors.${kind} em en`);
  }
  assert.ok(
    page.includes("{t(errorKey(error), { detail: errorDetail(error) })}"),
    "o erro aparece traduzido",
  );
  ok("C36-error");

  const busy = view(detail(), { transcribing: true, generating: true });
  assert.ok(
    busy.includes("meeting.view.transcribing") &&
      busy.includes("meeting.view.cancel"),
  );
  // Com anotações, "Aprimorando suas notas" (sem elas, "Escrevendo notas").
  assert.ok(busy.includes("meeting.view.enhancing"));
  // Transcrever, gerar e editar a nota.
  assert.equal((busy.match(/disabled=""/g) ?? []).length, 3, busy);
  assert.ok(!view(detail()).includes("meeting.view.cancel"));
  ok("C36-busy");

  const key = renderToStaticMarkup(<KeyField onSave={noop} />);
  assert.ok(key.includes('type="password"') && key.includes('value=""'), key);
  assert.ok(
    /keys && !keys\.elevenlabs && \(?\s*<KeyField/.test(page),
    "campo só sem chave",
  );
  ok("C36-key");
}

// C42: one notes field per session, the open session follows the recording, and the notice
// request is consumed when the page is already open (round 1 of the verification).
{
  const live = view(detail(), { recording: true });
  assert.ok(live.includes("meeting.view.notesAbove"), live);
  assert.ok(
    !live.includes("<textarea"),
    "sem segundo campo de anotações gravando",
  );
  assert.ok(view(detail()).includes("<textarea"));
  assert.ok(
    /\[status\.state, status\.id, openId, open\]/.test(page),
    "a sessão aberta recarrega quando o estado da gravação muda",
  );
  const app = read("../../App.tsx");
  const appListener = app.slice(
    app.indexOf('listen("meeting-consent-required"'),
  );
  const guard = appListener.indexOf('if (sectionRef.current !== "meetings") {');
  assert.ok(
    guard >= 0 && guard < appListener.indexOf("requestConsent();"),
    "o app só deixa o pedido quando a página ainda vai montar",
  );
  assert.ok(app.includes("sectionRef.current = destination;"));
  const pageListener = page.slice(
    page.indexOf('listen("meeting-consent-required"'),
  );
  assert.ok(
    !pageListener.slice(0, 120).includes("takeConsentRequest"),
    "a página aberta não disputa o pedido",
  );
  assert.ok(
    page.includes("useState(takeConsentRequest)"),
    "a montagem consome o pedido",
  );
  ok("C42");
}

// meeting-source-playback: a nota como documento, a lupa, a conversa e o ▶.
{
  // O formato que `crates/notes/src/render.rs` grava.
  const md = [
    "## Anotações",
    "",
    "decidir data ^a1",
    "",
    "Ana: contrato ^a2",
    "",
    "## Notas · Reunião geral",
    "",
    "### Resumo",
    "",
    "- Lançamento em 15/11. [[#^s1|01:05]] [[#^s999|09:59]] <!-- fala:ia -->",
    "- Ana cuida do contrato. [[#^s2|01:10]] [[#^a2|a2]] <!-- fala:ia -->",
    "- Clima bom na reunião. <!-- fala:ia -->",
    "",
  ].join("\n");
  const lines = [
    detail().lines[0],
    {
      seq: 2,
      channel: "mic",
      person: null,
      t0_ms: 70_000,
      t1_ms: 72_000,
      text: "Eu fecho com a Ana.",
    },
    {
      seq: 3,
      channel: "mic",
      person: null,
      t0_ms: 73_000,
      t1_ms: 74_000,
      text: "Até sexta.",
    },
  ];
  const parsed = parseNotes(md, lines);
  const rows = parsed.blocks.flatMap((b) => (b.kind === "line" ? [b] : []));
  assert.deepEqual(
    rows.map((b) => [b.text, b.generated, b.bullet]),
    [
      ["decidir data", false, false],
      ["Ana: contrato", false, false],
      ["Lançamento em 15/11.", true, true],
      ["Ana cuida do contrato.", true, true],
      ["Clima bom na reunião.", true, true],
    ],
  );
  assert.deepEqual(
    rows.map((b) => b.sources),
    [
      [],
      [],
      [{ kind: "segment", seq: 1 }],
      [
        { kind: "segment", seq: 2 },
        { kind: "annotation", n: 2 },
      ],
      [],
    ],
    "s999 não está na transcrição e sai",
  );
  assert.deepEqual(
    parsed.blocks.flatMap((b) =>
      b.kind === "heading" ? [[b.level, b.text]] : [],
    ),
    [
      [2, "Anotações"],
      [2, "Notas · Reunião geral"],
      [3, "Resumo"],
    ],
  );
  assert.equal(parsed.annotations.get(2), "Ana: contrato");
  ok("SP-parse");

  const segments = new Map(lines.map((l) => [l.seq, l]));
  const notesHtml = renderToStaticMarkup(
    <NotesView
      notes={parsed}
      segments={segments}
      audioAvailable={true}
      playing={null}
      onPlay={noop}
      onReveal={noop}
      t={tr}
    />,
  );
  for (const raw of ["[[#^", "fala:ia", "^a1", "^a2"]) {
    assert.ok(!notesHtml.includes(raw), `${raw} visível: ${notesHtml}`);
  }
  const paragraphs = [
    ...notesHtml.matchAll(/<p data-generated="(true|false)" class="([^"]*)"/g),
  ];
  assert.equal(paragraphs.length, 5);
  for (const [, generated, cls] of paragraphs) {
    const color = cls.split(" ").find((c) => c.startsWith("text-text"));
    assert.equal(color, generated === "true" ? "text-text-2" : "text-text");
  }
  assert.equal(
    notesHtml.split(`aria-label="${pt.meeting.source.open}"`).length - 1,
    3,
    "uma lupa por linha gerada",
  );
  assert.ok(notesHtml.includes("group-hover:opacity-100"));
  assert.ok(notesHtml.includes("focus-visible:opacity-100"));
  ok("SP-render");

  const popover = (row: number, audioAvailable = true) =>
    renderToStaticMarkup(
      <SourcePopover
        sources={rows[row].sources}
        segments={segments}
        annotations={parsed.annotations}
        audioAvailable={audioAvailable}
        playing={null}
        onPlay={noop}
        onReveal={noop}
        onClose={noop}
        t={tr}
      />,
    );
  const sourced = popover(3);
  for (const text of [
    "Eu",
    ">01:10<",
    "Eu fecho com a Ana.",
    pt.meeting.source.typed,
    "Ana: contrato",
    `aria-label="${pt.meeting.source.play}"`,
    'aria-label="Ir para 01:10 na transcrição"',
  ]) {
    assert.ok(sourced.includes(text), `${text}: ${sourced}`);
  }
  assert.ok(popover(2).includes("Pessoa 1") && popover(2).includes(">01:05<"));
  assert.ok(popover(2).includes("Proponho 15 de novembro."));
  assert.equal(pt.meeting.source.none, "Esta linha não tem trecho de origem.");
  assert.ok(popover(4).includes(pt.meeting.source.none));
  assert.ok(!popover(3).includes('disabled=""'));
  assert.ok(popover(3, false).includes('disabled=""'));
  ok("SP-popover");

  const chat = (audioAvailable: boolean, playing: number | null = null) =>
    renderToStaticMarkup(
      <TranscriptView
        lines={lines}
        audioAvailable={audioAvailable}
        playing={playing}
        highlighted={2}
        onPlay={noop}
        t={tr}
      />,
    );
  const convo = chat(true);
  assert.deepEqual(
    [
      ...convo.matchAll(
        /data-speaker="(\w+)" class="flex flex-col (items-\w+)/g,
      ),
    ].map((m) => [m[1], m[2]]),
    [
      ["them", "items-start"],
      ["me", "items-end"],
      ["me", "items-end"],
    ],
  );
  assert.equal((convo.match(/Pessoa 1 · 01:05/g) ?? []).length, 1);
  assert.equal(
    (convo.match(/Eu · /g) ?? []).length,
    1,
    "rótulo só na troca de falante",
  );
  assert.ok(convo.includes("Eu · 01:10"));
  assert.ok(convo.includes("bg-me/15") && convo.includes("bg-them/15"));
  assert.equal((convo.match(/ring-accent/g) ?? []).length, 1);
  assert.ok(convo.includes('id="segment-2"'));
  assert.equal(
    convo.split(`aria-label="${pt.meeting.source.play}"`).length - 1,
    3,
  );
  assert.ok(!convo.includes('disabled=""'));
  const gone = chat(false);
  assert.equal((gone.match(/disabled=""/g) ?? []).length, 3);
  assert.ok(gone.includes(`title="${pt.meeting.source.noAudio}"`));
  assert.ok(chat(true, 3).includes(`aria-label="${pt.meeting.source.stop}"`));
  assert.ok(
    view(detail({ audio_available: false })).includes("meeting.source.noAudio"),
  );
  assert.ok(!view(detail()).includes("meeting.source.noAudio"));
  const withNotes = view(detail({ notes_md: md, lines }));
  assert.ok(!withNotes.includes("<pre"), "a nota não sai crua");
  assert.ok(!withNotes.includes("[[#^") && !withNotes.includes("fala:ia"));
  ok("SP-transcript");
}

// O player: um trecho por vez, por Blob URL; o pedido ultrapassado não toca.
{
  const log: string[] = [];
  const pending: ((bytes: ArrayBuffer) => void)[] = [];
  let urls = 0;
  const deps: ClipDeps = {
    fetch: (id, t0, t1) => {
      log.push(`fetch ${id} ${t0}-${t1}`);
      return new Promise((resolve) => pending.push(resolve));
    },
    objectUrl: (blob) => {
      assert.equal(blob.type, "audio/wav");
      return `blob:${++urls}`;
    },
    revoke: (url) => log.push(`revoke ${url}`),
    audio: (url) => ({
      play: async () => void log.push(`play ${url}`),
      pause: () => log.push(`pause ${url}`),
      onEnded: noop,
    }),
  };
  const states: (number | null)[] = [];
  const player = new ClipPlayer((p) => states.push(p), deps);
  const first = player.play("S", 1, 65_000, 70_000);
  const second = player.play("S", 2, 70_000, 72_000);
  pending[1](new ArrayBuffer(44));
  await second;
  pending[0](new ArrayBuffer(44));
  await first;
  assert.deepEqual(log, [
    "fetch S 65000-70000",
    "fetch S 70000-72000",
    "play blob:1",
  ]);
  assert.equal(states[states.length - 1], 2);
  void player.play("S", 3, 73_000, 74_000);
  assert.deepEqual(log.slice(3), [
    "pause blob:1",
    "revoke blob:1",
    "fetch S 73000-74000",
  ]);
  player.stop();
  assert.equal(states[states.length - 1], null);
  const failing = new ClipPlayer((p) => states.push(p), {
    ...deps,
    fetch: () => Promise.reject({ kind: "no_audio" }),
  });
  await assert.rejects(failing.play("S", 1, 0, 1), { kind: "no_audio" });
  assert.equal(states[states.length - 1], null);
  assert.equal(errorKey({ kind: "no_audio" }), "meeting.errors.no_audio");
  const clip = read("./clipPlayer.ts");
  assert.ok(
    clip.includes('invoke<ArrayBuffer>("meeting_clip", { id, t0Ms, t1Ms })'),
  );
  const conf = read("../../../apps/desktop/tauri.conf.json");
  assert.ok(
    conf.includes('"allow": ["$APPDATA/recordings/**"]'),
    "o assetProtocol não muda",
  );
  const lib = read("../../../apps/desktop/src/lib.rs");
  assert.ok(lib.includes('invoke.message.command() == "meeting_clip"'));
  ok("SP-player");
}

console.log("meeting: all assertions passed");
