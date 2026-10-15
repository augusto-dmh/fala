// Prova da barra inferior e da folha da transcrição (`.specs/features/meetings-ui-redesign/
// checks.md`: C10–C11). Rode com `bun src/components/meeting/bar.test.tsx`: imprime
// `<check> ok` e sai com erro na primeira falha. Sem instância do i18next, `t` devolve a chave.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingStatus } from "@/bindings";
import type { Translate } from "./meetingModel";
import { RecorderPanel } from "./RecorderPanel";
import { SessionBar, type SessionBarProps } from "./SessionBar";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const keys: Translate = (key) => key;
const noop = () => {};
const PICKER = "picker";

function ok(id: string) {
  console.log(`${id} ok`);
}

const status = (over: Partial<MeetingStatus> = {}): MeetingStatus => ({
  state: "recording",
  id: "s",
  mode: "meeting",
  recorded_ms: 754_000,
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
const live = (over: Partial<MeetingStatus> = {}) => ({
  status: status(over),
  onPause: noop,
  onResume: noop,
  onStop: noop,
});
const bar = (over: Partial<SessionBarProps> = {}) =>
  renderToStaticMarkup(
    <SessionBar
      draft={false}
      live={null}
      transcriptOpen={false}
      onToggleTranscript={noop}
      onRecordDraft={noop}
      transcribing={false}
      transcribeLabel="meeting.view.transcribe"
      transcribeDisabled={false}
      onTranscribe={noop}
      onCancelTranscription={noop}
      generateLabel="meeting.view.generate"
      generateDisabled={false}
      onGenerate={noop}
      templateSelect={<span>{PICKER}</span>}
      templateMenu={() => <span>{PICKER}</span>}
      t={keys}
      {...over}
    />,
  );

// C10: a barra mostra o que a sessão pode fazer agora.
{
  const done = bar();
  assert.ok(done.includes('role="toolbar"'));
  assert.ok(done.includes('aria-pressed="false"'), "onda da transcrição");
  assert.ok(done.includes("meeting.view.transcribe"));
  assert.ok(done.includes(">meeting.view.generate</button>"));
  assert.ok(done.includes('aria-label="meeting.view.template"'));
  assert.ok(!done.includes(PICKER), "o template fica atrás da seta");
  assert.ok(!done.includes("meeting.view.cancel"));
  assert.ok(bar({ transcriptOpen: true }).includes('aria-pressed="true"'));
  const running = bar({
    transcribing: true,
    transcribeLabel: "meeting.view.transcribing",
    transcribeDisabled: true,
  });
  assert.ok(running.includes("meeting.view.transcribing"));
  assert.ok(running.includes("meeting.view.cancel"));

  const recording = bar({ live: live() });
  assert.ok(recording.includes("12:34"));
  assert.ok(recording.includes("meeting.recorder.pause"));
  assert.ok(recording.includes("meeting.recorder.stop"));
  assert.ok(!recording.includes("meeting.view.transcribe"));
  assert.ok(
    bar({ live: live({ state: "paused" }) }).includes(
      "meeting.recorder.resume",
    ),
  );
  const processing = bar({ live: live({ state: "processing" }) });
  assert.ok(processing.includes("meeting.recorder.processing"));
  assert.ok(!processing.includes("meeting.recorder.stop"));

  const draft = bar({ draft: true });
  assert.ok(
    draft.includes("meeting.view.recordDraft") && draft.includes(PICKER),
  );
  assert.ok(!draft.includes("aria-pressed"), "rascunho não tem transcrição");
  ok("C10");
}

// C11: a transcrição numa folha lateral que a onda abre e a fonte de uma nota também; os
// avisos do gravador em faixas âmbar, e os controles só na barra da reunião.
{
  const view = read("./MeetingView.tsx");
  assert.match(view, /<aside\s+hidden=\{!transcriptOpen\}/);
  assert.match(
    view,
    /setTranscriptOpen\(true\);[\s\S]*requestAnimationFrame\(scroll\);/,
  );
  assert.match(view, /<SessionBar[\s\S]*live=\{props\.live \?\? null\}/);
  const page = read("./MeetingsPage.tsx");
  assert.match(page, /onStop: \(\) => void act\(commands\.stopMeeting\)/);

  const panel = (controls: boolean, over: Partial<MeetingStatus> = {}) =>
    renderToStaticMarkup(
      <RecorderPanel
        status={status(over)}
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
        controls={controls}
      />,
    );
  assert.ok(panel(true).includes("meeting.recorder.stop"));
  assert.ok(!panel(false).includes("meeting.recorder.stop"));
  assert.ok(panel(false).includes("meeting.notice.copy"));
  const muted = panel(false, { muted_mic: true, low_disk_bytes: 1 << 20 });
  assert.equal(
    muted.match(/role="alert" class="[^"]*border-warn bg-warn\/10/g)?.length,
    2,
    muted,
  );
  // Escape fecha a folha, que fica sobre a barra (Q23 do QA: a barra cortava a folha).
  assert.match(
    view,
    /if \(event\.key === "Escape"\) setTranscriptOpen\(false\);/,
  );
  assert.match(view, /fixed inset-y-0 end-0 z-\[45\]/);
  assert.match(read("./SessionBar.tsx"), /sticky bottom-4 z-40/);
  ok("C11");
}
