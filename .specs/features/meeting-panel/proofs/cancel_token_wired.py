"""C35: the transcription's progress becomes MeetingProgress and the cancel button cancels the
same CancelToken the Scribe request polls."""
from lib import DESKTOP, fail, fn_body, ok, production

pipeline = production(DESKTOP / "meeting/pipeline.rs")
transcribe = fn_body(pipeline, "pub fn transcribe(")
if "work.transcribing = Some((id, cancel.clone()))" not in transcribe:
    fail("transcribe does not register its CancelToken")
if "self.run_transcription(id, &record, &cancel)" not in transcribe:
    fail("transcribe does not pass that token on")
run = fn_body(pipeline, "fn run_transcription(")
if "scribe.transcribe_session(&recording, cancel," not in run:
    fail("the Scribe request does not receive the token")
if "self.progress(id, progress_stage(progress))" not in run:
    fail("Scribe progress is not emitted")
cancel = fn_body(pipeline, "pub fn cancel_transcription(")
if "cancel.cancel()" not in cancel:
    fail("cancel_transcription does not cancel the token")
emit = fn_body(pipeline, "pub(crate) fn progress(")
if "MeetingProgress" not in emit or ".emit(&self.app)" not in emit:
    fail("progress does not emit MeetingProgress")
ok("progress is emitted and cancel reaches the token Scribe polls")
