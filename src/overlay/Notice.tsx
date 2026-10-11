import { noticeHasActions, noticeTone, type NoticeKind } from "./noticeModel";

export interface NoticeProps {
  kind: NoticeKind;
  /** The translated text (`overlay.notice.*`). */
  text: string;
  chooseLabel: string;
  resolveLabel: string;
  onChoose: () => void;
  onResolve: () => void;
}

/** A short card in the overlay window: a failed gesture, a muted or a new microphone. */
export function Notice({
  kind,
  text,
  chooseLabel,
  resolveLabel,
  onChoose,
  onResolve,
}: NoticeProps) {
  const tone = noticeTone(kind);
  return (
    <div className={`fnotice ${kind}`} role="status">
      <div className="fnotice-line">
        {tone && <span className={`fnotice-dot ${tone}`} />}
        <span className="fnotice-text">{text}</span>
      </div>
      {noticeHasActions(kind) && (
        <div className="fnotice-actions">
          <button type="button" onClick={onChoose}>
            {chooseLabel}
          </button>
          <button type="button" onClick={onResolve}>
            {resolveLabel}
          </button>
        </div>
      )}
    </div>
  );
}
