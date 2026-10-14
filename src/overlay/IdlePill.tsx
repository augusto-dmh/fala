export interface IdlePillProps {
  /** The cursor is over the pill: the capsule opens on the shortcut. */
  hover: boolean;
  /** The keys of the `transcribe` binding, ready to show (`idleKeys`). */
  keys: readonly string[];
  /** Translated "Ditar". */
  dictateLabel: string;
  /** Translated double-tap hint, only for the default activation; `null` otherwise. */
  doubleTapLabel: string | null;
  /** Translated name for screen readers. */
  label: string;
  onOpen?: () => void;
  /** Translated "Nova reunião" for the hover card's button; no button without it. */
  newMeetingLabel?: string;
  /** Starts a meeting: only this explicit click does, never the hover (ADR-0005). */
  onNewMeeting?: () => void;
}

/**
 * The pill at rest: a thin, translucent capsule with no bars, no colour and no time, so it never
 * looks like a recording (ADR-0005). While the cursor is over the window (`hover`) it opens on
 * the dictation shortcut; a click opens the main window.
 */
export function IdlePill({
  hover,
  keys,
  dictateLabel,
  doubleTapLabel,
  label,
  onOpen,
  newMeetingLabel,
  onNewMeeting,
}: IdlePillProps) {
  const meetingButton = Boolean(hover && newMeetingLabel && onNewMeeting);
  return (
    // With the "New meeting" button inside, the card is a group, not a button: a control inside
    // another would hide it from screen readers.
    <div
      className={`fpill idle${hover ? " hover" : ""}`}
      role={meetingButton ? "group" : "button"}
      aria-label={label}
      onClick={onOpen}
    >
      {hover && (
        <>
          <span className="idle-row">
            <span className="idle-dictate">{dictateLabel}</span>
            {keys.map((key) => (
              <kbd key={key}>{key}</kbd>
            ))}
          </span>
          {doubleTapLabel && (
            <span className="idle-hint">{doubleTapLabel}</span>
          )}
          {meetingButton && (
            <button
              type="button"
              className="idle-meeting"
              // A press here is a click, never the start of a pill drag.
              onPointerDown={(event) => event.stopPropagation()}
              onMouseDown={(event) => event.stopPropagation()}
              onClick={(event) => {
                // The card's own click opens the main window; this one only starts the meeting.
                event.stopPropagation();
                onNewMeeting?.();
              }}
            >
              {newMeetingLabel}
            </button>
          )}
        </>
      )}
    </div>
  );
}
