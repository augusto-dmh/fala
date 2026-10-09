"""C13: UserAction::StartRecording/ResumeRecording are built only in MeetingManager::start/resume,
which only the meeting commands and the tray items call (ADR-0005)."""
from lib import DESKTOP, fail, fn_body, occurrences, ok, production

start = occurrences(r"UserAction::StartRecording")
resume = occurrences(r"UserAction::ResumeRecording")
if [p for p, _ in start] != ["apps/desktop/src/meeting/mod.rs"]:
    fail(f"StartRecording outside meeting/mod.rs or missing: {start}")
if [p for p, _ in resume] != ["apps/desktop/src/meeting/mod.rs"]:
    fail(f"ResumeRecording outside meeting/mod.rs or missing: {resume}")

manager = production(DESKTOP / "meeting/mod.rs")
if "UserAction::StartRecording" not in fn_body(manager, "pub fn start(\n"):
    fail("StartRecording is not inside MeetingManager::start")
if "UserAction::ResumeRecording" not in fn_body(manager, "pub fn resume("):
    fail("ResumeRecording is not inside MeetingManager::resume")
ok("the two actions are built only in start and resume")

callers_start = occurrences(r"\.start\(\s*$|\.start\((mode|MeetingMode)")
expected = {
    ("apps/desktop/src/commands/meeting.rs", "meetings.start(mode, &title, draft_id.as_deref())"),
    ("apps/desktop/src/meeting/mod.rs", 'if let Err(e) = self.start(MeetingMode::Meeting, "", None) {'),
}
if set(callers_start) != expected:
    fail(f"unexpected callers of MeetingManager::start: {callers_start}")
tray = fn_body(manager, "pub fn start_from_tray(")
if "TrayStart::AskConsent" not in tray:
    fail("start_from_tray does not ask for the notice first")
from_tray = occurrences(r"start_from_tray\(\)")
if [p for p, _ in from_tray] != ["apps/desktop/src/lib.rs"]:
    fail(f"start_from_tray called outside the tray item: {from_tray}")
lib = production(DESKTOP / "lib.rs")
arm = lib[lib.find('"meeting_start" =>') :][:200]
if "start_from_tray" not in arm:
    fail("the meeting_start tray item does not call start_from_tray")
ok("start is reached only from start_meeting and the meeting_start tray item")

callers_resume = occurrences(r"\.resume\(\)")
paths = sorted({p for p, _ in callers_resume})
if paths != ["apps/desktop/src/commands/meeting.rs", "apps/desktop/src/lib.rs"]:
    fail(f"unexpected callers of resume: {callers_resume}")
if '"meeting_resume" => meetings.resume()' not in lib:
    fail("resume in lib.rs is not the meeting_resume tray item")
ok("resume is reached only from resume_meeting and the meeting_resume tray item")
if lib.count("start_from_tray") != 1 or lib.count('"meeting_start" =>') != 1:
    fail("start_from_tray must be called once, from the meeting_start tray arm")
ok("so nothing but an explicit click (command or tray item) starts or resumes a meeting")
