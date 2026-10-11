"""C39-C41 and C43: the defects the verifier found in round 1 stay fixed.

C39: one WAV -> Opus conversion at a time (ADR-0014): `retain` holds `retaining` before reading.
C40: no path opens the dictation stream while a meeting records (ADR-0015).
C41: the session row is written only after the capture opened (no orphan draft on failure).
"""
from lib import DESKTOP, fail, fn_body, ok, production

manager = production(DESKTOP / "meeting/mod.rs")
retain = fn_body(manager, "fn retain(")
lock = retain.find("self.retaining.lock()")
if lock < 0 or lock > retain.find("wav.exists()"):
    fail("retain must hold the retaining lock before looking at the WAV")
if "retaining: Mutex<()>" not in manager:
    fail("the manager has no retaining lock")
ok("C39: conversions of the same session never overlap")

audio = production(DESKTOP / "managers/audio.rs")
start_stream = fn_body(audio, "pub fn start_microphone_stream(")
guard = start_stream.find("crate::meeting::indicator() != crate::meeting::MeetingIndicator::None")
if guard < 0 or guard > start_stream.find("self.is_open"):
    fail("start_microphone_stream must refuse first while a meeting records")
ok("C40: the dictation stream never opens during a meeting")

start = fn_body(manager, "pub fn start(\n")
spawn = start.find("recorder::spawn(")
create = start.find("store.create_meeting(")
if spawn < 0 or create < 0 or create < spawn:
    fail("start must write the session row after the capture opened")
ok("C41: a capture that fails leaves no session row")

mode = fn_body(audio, "pub fn update_mode(")
always_on = mode[mode.find("(MicrophoneMode::OnDemand, MicrophoneMode::AlwaysOn)") :]
guarded = always_on.find("crate::meeting::indicator() == crate::meeting::MeetingIndicator::None")
if guarded < 0 or guarded > always_on.find("self.start_microphone_stream()"):
    fail("switching to always-on during a meeting must store the mode without opening the mic")
lib = production(DESKTOP / "lib.rs")
arms = lib[lib.find('"meeting_pause" | "meeting_resume" | "meeting_stop" =>') :][:600]
if "std::thread::spawn" not in arms or arms.find("std::thread::spawn") > arms.find("meetings.stop()"):
    fail("the tray pause/resume/stop must run off the event loop")
failure = start[start.find("recorder::spawn(") :]
if "std::fs::remove_dir(&dir)" not in failure[: failure.find("return Err")]:
    fail("a capture that fails must remove its empty audio directory")
ok("C43: always-on toggle during a meeting, tray actions off the event loop, no empty folder")
