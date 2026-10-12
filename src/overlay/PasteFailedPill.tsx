export interface PasteFailedPillProps {
  /** Translated "Não deu para colar." */
  title: string;
  /** Translated second line (`pasteFailedHint`). */
  hint: string;
  /** Translated name for screen readers. */
  label: string;
  onCopy: () => void;
  /** The pointer entered or left the card: its timer stops while the pointer is on it. */
  onHover: (on: boolean) => void;
}

/**
 * The pill after a paste that failed: the warning tone of the notices (ADR-0017), what happened
 * and how to get the text; a click copies it. The clipboard was already given back by the paste.
 */
export function PasteFailedPill({
  title,
  hint,
  label,
  onCopy,
  onHover,
}: PasteFailedPillProps) {
  return (
    <div
      className="fnotice paste_failed"
      role="button"
      aria-label={label}
      onClick={onCopy}
      onMouseEnter={() => onHover(true)}
      onMouseLeave={() => onHover(false)}
    >
      <div className="fnotice-line">
        <span className="fnotice-dot warn" />
        <span className="fnotice-text">{title}</span>
      </div>
      <span className="fnotice-hint">{hint}</span>
    </div>
  );
}
