import type { CallPromptAnswer, CallPromptKind } from "@/bindings";
import "./CallPrompt.css";

export interface CallPromptAction {
  answer: CallPromptAnswer;
  label: string;
  primary?: boolean;
}

export interface CallPromptCardProps {
  kind: CallPromptKind;
  /** Translated title and body (`callPrompt.*`). */
  title: string;
  body: string;
  actions: CallPromptAction[];
  /** 1 when shown, 0 when the card closes. */
  remaining: number;
  onAnswer: (answer: CallPromptAnswer) => void;
  onHover: (hovered: boolean) => void;
}

/** The answers each card offers, in order: the first is the main one. */
export const CALL_PROMPT_ANSWERS: Record<CallPromptKind, CallPromptAnswer[]> = {
  offer: ["record", "not_now", "mute_app"],
  ask_stop: ["stop", "keep_recording"],
};

/** The card in the 404×104 prompt window: the pill's black, a bar on the left, two lines,
 *  the buttons and the countdown line along the bottom. */
export function CallPromptCard({
  kind,
  title,
  body,
  actions,
  remaining,
  onAnswer,
  onHover,
}: CallPromptCardProps) {
  return (
    <div
      className={`fcall ${kind}`}
      role="alertdialog"
      aria-label={title}
      onMouseEnter={() => onHover(true)}
      onMouseLeave={() => onHover(false)}
    >
      <span className="fcall-bar" />
      <div className="fcall-main">
        <div className="fcall-title">{title}</div>
        <div className="fcall-body">{body}</div>
        <div className="fcall-actions">
          {actions.map((action) => (
            <button
              key={action.answer}
              type="button"
              className={action.primary ? "primary" : undefined}
              onClick={() => onAnswer(action.answer)}
            >
              {action.label}
            </button>
          ))}
        </div>
      </div>
      <span
        className="fcall-countdown"
        style={{ transform: `scaleX(${Math.max(0, Math.min(1, remaining))})` }}
      />
    </div>
  );
}
