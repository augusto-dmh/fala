import { noticeTone, type NoticeKind } from "./noticeModel";

export interface NoticeProps {
  kind: NoticeKind;
  /** The translated text (`overlay.notice.*`). */
  text: string;
}

/** A short card in the overlay window: why the dictation did not start. */
export function Notice({ kind, text }: NoticeProps) {
  const tone = noticeTone(kind);
  return (
    <div className={`fnotice ${kind}`} role="status">
      <div className="fnotice-line">
        {tone && <span className={`fnotice-dot ${tone}`} />}
        <span className="fnotice-text">{text}</span>
      </div>
    </div>
  );
}
