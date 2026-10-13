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
  quit: ["stop_and_quit", "cancel"],
  still_recording: ["keep_recording", "stop"],
  silence_stopped: ["record_again", "dismiss"],
  mistake: ["keep", "discard"],
  nothing_recorded: ["dismiss"],
  woke: ["dismiss"],
};

/** The `callPrompt.*` group with the title and body of each card. */
export const CALL_PROMPT_TEXT: Record<CallPromptKind, string> = {
  offer: "offer",
  ask_stop: "askStop",
  quit: "quit",
  still_recording: "stillRecording",
  silence_stopped: "silenceStopped",
  mistake: "mistake",
  nothing_recorded: "nothingRecorded",
  woke: "woke",
};

/** The title key: a meeting in a browser names both ("Google Meet no Edge"); the cards about
 *  the meeting itself name no app. */
export function titleKey(kind: CallPromptKind, webName: string | null): string {
  if (kind === "offer" || kind === "ask_stop") {
    const card = CALL_PROMPT_TEXT[kind];
    return webName ? `callPrompt.${card}.titleWeb` : `callPrompt.${card}.title`;
  }
  return `callPrompt.${CALL_PROMPT_TEXT[kind]}.title`;
}

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
