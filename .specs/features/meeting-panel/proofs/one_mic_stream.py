"""C19: the dictation stream is closed before the meeting opens the mic, and reopened at the end
only when the microphone is always on (ADR-0015, one stream per device)."""
from lib import DESKTOP, fail, fn_body, ok, production

manager = production(DESKTOP / "meeting/mod.rs")
start = fn_body(manager, "pub fn start(\n")
release = start.find("self.release_dictation_mic()")
spawn = start.find("recorder::spawn(")
if release < 0 or spawn < 0 or release > spawn:
    fail("start must release the dictation mic before spawning the recorder")
if "self.restore_dictation_mic()" not in fn_body(manager, "fn finish("):
    fail("finish does not give the mic back")
failure = start[spawn:]
if "self.restore_dictation_mic()" not in failure[: failure.find("return Err")]:
    fail("a capture that does not open must give the mic back")
ok("the meeting takes the mic from dictation and gives it back")

audio = production(DESKTOP / "managers/audio.rs")
release_fn = fn_body(audio, "pub fn release_microphone_for_meeting(")
if "self.stop_microphone_stream()" not in release_fn:
    fail("release_microphone_for_meeting does not stop the dictation stream")
restore_fn = fn_body(audio, "pub fn restore_microphone_after_meeting(")
if restore_fn.find("MicrophoneMode::AlwaysOn") < 0 or restore_fn.find(
    "MicrophoneMode::AlwaysOn"
) > restore_fn.find("self.start_microphone_stream()"):
    fail("restore must reopen only in always-on mode")
ok("the stream reopens only when the microphone is always on")
