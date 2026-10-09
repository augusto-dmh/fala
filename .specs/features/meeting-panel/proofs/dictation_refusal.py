"""C18: TranscribeAction::start refuses before loading the model or opening the mic, and the UI
maps `meeting_active` to its own message (ADR-0015)."""
import json

from lib import DESKTOP, ROOT, fail, fn_body, ok, production

actions = production(DESKTOP / "actions.rs")
impl = actions[actions.find("impl ShortcutAction for TranscribeAction") :]
body = fn_body(impl, "fn start(")
refusal = body.find("crate::meeting::dictation_refusal(crate::meeting::indicator())")
if refusal < 0:
    fail("TranscribeAction::start does not consult the meeting indicator")
for later in ["initiate_model_load", "set_tray_state", "try_start_recording"]:
    at = body.find(later)
    if at < 0 or at < refusal:
        fail(f"{later} runs before the meeting refusal (or is missing)")
branch = body[refusal : body.find("initiate_model_load")]
if '"recording-error"' not in branch or "return;" not in branch:
    fail("the refusal does not emit recording-error and return")
ok("dictation is refused before the model load, the tray and the mic")

app = (ROOT / "src/App.tsx").read_text()
if 'error_type === "meeting_active"' not in app or 't("errors.meetingActive")' not in app:
    fail("App.tsx does not map meeting_active to errors.meetingActive")
for lang in ["pt", "en"]:
    data = json.loads((ROOT / f"src/i18n/locales/{lang}/translation.json").read_text())
    if not data["errors"].get("meetingActive"):
        fail(f"errors.meetingActive missing in {lang}")
ok("the UI shows errors.meetingActive in pt and en")
