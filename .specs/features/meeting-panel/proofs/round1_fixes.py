"""C39-C41: the defects the verifier found in round 1 stay fixed.

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
