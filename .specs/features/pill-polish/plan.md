# pill-polish plan

Profile: light · one PR (`feat/pill-polish`)

## Goal

1. A paste that fails shows in the pill for 5 s (stopped while the pointer is on it): "Não deu
   para colar." and "<paste_last shortcut> cola de novo · clique para copiar" (only "Clique para
   copiar" without a shortcut). A click copies the text and the line becomes "Copiado". The
   clipboard is still given back after the failed paste, and the main window keeps its toast.
2. A click on the meeting pill (outside the mic switch card) opens the main window on Reuniões
   with the recording session open.

## Shape

- Backend: on a failed paste (dictation in `actions.rs`, paste-last in `last_dictation.rs`) the
  overlay goes to a new `paste_failed` state (`overlay::show_paste_failed`) instead of hiding.
  The text that failed is kept in memory (`last_dictation::FAILED_PASTE`) and copied by
  `copy_failed_paste`. `paste-failed` tells the page the show generation; `end_paste_failed`
  (generation) forgets the text and hides only that pill, on the main thread; a 60 s guard ends it
  if the page never asks. Any other state forgets the text. The pill never takes the overlay from
  a live recording or the meeting indicator (ADR-0005): there the old hide runs.
- `open_meeting_from_pill` shows the main window and emits `open-meeting` with the session id;
  `App.tsx` routes to Reuniões and leaves a request for a page that mounts later
  (`requestOpenMeeting`), as the consent notice does; an open page opens it from its listener.
- Frontend: `pillModel.ts` (`pasteLastShortcut`, `pasteFailedTimeout`, `pasteFailedHint`),
  `PasteFailedPill.tsx` (notice card, `warn` dot, ADR-0017), `RecordingOverlay.tsx` branch and
  timer, `MeetingPill` `onOpen`, pt/en strings.

## Decisions

- Copy the text that failed, not `copy_last` from history: history only saves an entry with a
  verified WAV, so `copy_last` could copy an older dictation.
- Show the paste-failed pill even with the overlay off, like the `alone` notices.
- The 5 s restart when the pointer leaves (the timer stops on hover and starts again on leave).
- Paste-last failures use the same pill, and while the pill is up `paste_last` pastes the text
  that failed (it may have no history entry), so "cola de novo" is literal.
- During a meeting the paste failure stays in the main window's toast: the meeting indicator is
  not replaced (ADR-0005).
