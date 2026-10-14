// Prova dos checks de UI de retomar uma reunião (`meeting-session-edit`, R*). Rode com
// `bun src/components/meeting/resume.test.tsx`: imprime `<check> ok` e sai com erro na primeira
// falha. Sem instância do i18next, `t` devolve a chave.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingLine } from "@/bindings";
import type { Translate } from "./meetingModel";
import { SessionBar, type SessionBarProps } from "./SessionBar";
import { TranscriptView } from "./TranscriptView";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const view = read("./MeetingView.tsx");
const page = read("./MeetingsPage.tsx");
const keys: Translate = (key) => key;
const noop = () => {};

function ok(id: string) {
  console.log(`${id} ok`);
}

const bar = (over: Partial<SessionBarProps> = {}) =>
  renderToStaticMarkup(
    <SessionBar
      draft={false}
      live={null}
      transcriptOpen={false}
      onToggleTranscript={noop}
      onRecordDraft={noop}
      transcribing={false}
      transcribeDisabled={false}
      onTranscribe={noop}
      onCancelTranscription={noop}
      generateLabel="meeting.view.generate"
      generateDisabled={false}
      onGenerate={noop}
      templateSelect={null}
      templateMenu={() => null}
      t={keys}
      {...over}
    />,
  );

// R1: "Retomar" sits next to "Transcrever" on a finished session, only where it can take one,
// and is off while the session is busy.
{
  assert.ok(
    !bar().includes("meeting.view.continue"),
    "sem onContinue, sem botão",
  );
  const finished = bar({ onContinue: noop });
  assert.ok(finished.includes("meeting.view.continue"), finished);
  assert.ok(
    finished.indexOf("meeting.view.continue") <
      finished.indexOf("meeting.view.transcribe"),
  );
  assert.match(
    bar({ onContinue: noop, continueDisabled: true }),
    /disabled=""[^>]*><span[^>]*><\/span>meeting\.view\.continue/,
  );
  // Never while recording: the live controls take the bar.
  const recording = bar({
    onContinue: noop,
    live: {
      status: {
        state: "recording",
        id: "s",
        mode: "meeting",
        recorded_ms: 1_000,
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
      },
      onPause: noop,
      onResume: noop,
      onStop: noop,
    },
  });
  assert.ok(!recording.includes("meeting.view.continue"), recording);
  // Only a recorded, finished session that is not an import gets it.
  assert.match(
    view,
    /props\.onContinue &&\s*!draft &&\s*!granola &&\s*detail\.summary\.mode !== "import" &&\s*detail\.summary\.ended_at !== null/,
  );
  assert.equal(pt.meeting.view.continue, "Retomar");
  assert.equal(en.meeting.view.continue, "Resume");
  ok("R1 Retomar button");
}

// R2: a click calls the app, with the third-party notice first when it was never accepted, like
// a start (ADR-0005); nothing else resumes.
{
  assert.equal(page.match(/commands\.continueMeeting\(/g)?.length, 1);
  assert.match(
    page,
    /onContinue=\{\(\) => void continueSession\(selected\.summary\.id\)\}/,
  );
  assert.match(page, /setPendingContinue\(id\);\s*setAskConsent\(true\);/);
  assert.match(page, /await continueSession\(pendingContinue\);/);
  // Cancelling the notice, or starting a new meeting, forgets the pending "Retomar".
  assert.match(page, /setAskConsent\(false\);\s*setPendingContinue\(null\);/);
  assert.match(page, /const start = async[^]*?setPendingContinue\(null\);/);
  // Another meeting recording keeps it off.
  assert.match(page, /recordingElsewhere=\{recordingNow && !recordingThis\}/);
  assert.match(view, /props\.generating \|\|\s*props\.recordingElsewhere/);
  ok("R2 click and consent");
}

// R3: the take's start is a marker line, not a bubble; the next speaker's turn ignores it.
{
  const line = (seq: number, over: Partial<MeetingLine>): MeetingLine => ({
    seq,
    channel: "mic",
    person: null,
    t0_ms: 0,
    t1_ms: 1_000,
    text: "oi",
    ...over,
  });
  const html = renderToStaticMarkup(
    <TranscriptView
      lines={[
        line(1, {}),
        line(2, {
          resumed: true,
          t0_ms: 60_000,
          t1_ms: 60_000,
          text: "[gravação retomada às 15:42]",
        }),
        line(3, { t0_ms: 60_000, t1_ms: 61_000, text: "voltamos" }),
      ]}
      audioAvailable
      playing={null}
      highlighted={null}
      onPlay={noop}
      t={keys}
    />,
  );
  assert.match(html, /data-speaker="resumed"[^>]*>meeting\.resumed · 01:00/);
  // "Eu" spoke before and after the marker: one turn, so one speaker label.
  assert.equal(html.match(/meeting\.speaker\.me/g)?.length, 1, html);
  assert.equal(pt.meeting.resumed, "Gravação retomada");
  ok("R3 resumed marker");
}

// R4: after a take of a session with notes is transcribed, "A transcrição mudou." offers to
// write them again, through the edited-notes confirmation.
{
  assert.match(page, /if \(hadNotes\) takesWithNotes\.current\.add\(id\);/);
  assert.match(
    page,
    /stage\.stage === "done" && takesWithNotes\.current\.delete\(id\)/,
  );
  assert.match(
    view,
    /props\.takeTranscribed && detail\.notes_md !== null && !confirming/,
  );
  assert.match(view, /props\.onTakeSeen\?\.\(\);\s*generate\(\);/);
  ok("R4 transcript changed after a take");
}
