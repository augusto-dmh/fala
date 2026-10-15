// The muted-mic warning (meeting-mic, PR 2): the panel's strip names the microphone and offers
// the switch; the pill's card opens the main window with the picker.
// Run with `bun src/components/meeting/micSwitch.test.tsx`: prints `<check> ok` and exits with an
// error on the first failure.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingStatus } from "@/bindings";
import { CallOutputCard } from "../../overlay/MeetingPill";
import { onMicPick, PICK_MIC_EVENT, requestMicPick } from "./micPick";
import { RecorderPanel } from "./RecorderPanel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};

const status = (over: Partial<MeetingStatus> = {}): MeetingStatus => ({
  state: "recording",
  id: "01K",
  mode: "meeting",
  recorded_ms: 20_000,
  mic_level: 0,
  system_level: 0.01,
  muted_mic: false,
  muted_system: false,
  mic_unheard: false,
  cap_remaining_ms: null,
  low_disk_bytes: null,
  mic_input: "Microfone (NVIDIA Broadcast)",
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
    />,
  );

// S1: the strip names the microphone and carries the switch, without stopping anything.
{
  assert.ok(!panel(status()).includes("meeting.recorder.switchMic"));
  const muted = panel(status({ muted_mic: true }));
  assert.ok(muted.includes("meeting.recorder.mutedMicNamed"), muted);
  assert.ok(muted.includes("meeting.recorder.switchMic"), muted);
  const unnamed = panel(status({ muted_mic: true, mic_input: null }));
  assert.ok(unnamed.includes("meeting.recorder.mutedMic"), unnamed);
  assert.ok(!unnamed.includes("meeting.recorder.mutedMicNamed"), unnamed);
  ok("S1");
}

// S2: the pill's card is the call-output card with its own kind and the switch label.
{
  const html = renderToStaticMarkup(
    <CallOutputCard
      kind="muted_mic"
      text="Não estamos ouvindo seu microfone (Mic X)"
      switchLabel="Trocar microfone"
      onSwitch={noop}
    />,
  );
  assert.match(html, /class="fnotice muted_mic"/);
  assert.match(html, /role="alert"/);
  assert.match(html, />Trocar microfone</);
  const overlay = read("../../overlay/RecordingOverlay.tsx");
  assert.ok(overlay.includes("commands.openMeetingMicPicker()"));
  // The card is for a mic never heard, not for two quiet minutes of a working one.
  assert.ok(overlay.includes("event.payload.mic_unheard ?"));
  assert.ok(
    overlay.indexOf('kind="muted_mic"') <
      overlay.indexOf("showsConsentChip(meetingMs) ?"),
    "the mic card wins over the consent chip",
  );
  ok("S2");
}

// S3: the pill's request waits for a picker that mounts later (the main window opens on the
// session first), opens one already on screen, and opens nothing twice.
{
  let opened = 0;
  requestMicPick();
  const stop = onMicPick(() => opened++);
  assert.equal(opened, 1, "a waiting request opens the picker that mounts");
  requestMicPick();
  assert.equal(opened, 2, "a picker on screen opens at once");
  stop();
  requestMicPick();
  assert.equal(opened, 2, "nobody to open: it waits");
  const again = onMicPick(() => opened++);
  assert.equal(opened, 3);
  again();
  const rust = read("../../../apps/desktop/src/overlay.rs");
  assert.ok(rust.includes(`emit_to("main", "${PICK_MIC_EVENT}", ())`));
  ok("S3");
}

// S4: the strings in pt and en, with the device where it is named.
{
  for (const lang of ["pt", "en"]) {
    const json = JSON.parse(
      read(`../../i18n/locales/${lang}/translation.json`),
    );
    assert.match(json.meeting.recorder.mutedMicNamed, /\{\{device\}\}/);
    assert.doesNotMatch(json.meeting.recorder.mutedMic, /2/);
    assert.ok(json.meeting.recorder.switchMic);
    assert.match(json.overlay.mutedMic, /\{\{device\}\}/);
    assert.ok(json.overlay.mutedMicUnnamed && json.overlay.switchMic);
  }
  ok("S4");
}
