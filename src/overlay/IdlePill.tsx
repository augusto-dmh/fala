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
}: IdlePillProps) {
  return (
    <div
      className={`fpill idle${hover ? " hover" : ""}`}
      role="button"
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
        </>
      )}
    </div>
  );
}
