import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import {
  commands,
  type CallPromptAnswer,
  type CallPromptPayload,
} from "@/bindings";
import {
  CALL_PROMPT_TEXT,
  CallPromptCard,
  cardAnswers,
  titleKey,
} from "./CallPromptCard";
import { advance, expired, fraction, startCountdown } from "./countdown";

const TICK_MS = 100;

/** The body key and its values: a short recording and an almost silent long one ask for
 *  different reasons; the wake card says how long the PC slept; a "stop?" that stops when it
 *  runs out counts the seconds left (`stopInMs`). */
export function cardBody(
  text: string,
  seconds: number | null,
  stopInMs: number | null = null,
): [string, Record<string, unknown>] {
  if (text === "askStop" && stopInMs !== null)
    return [
      "callPrompt.askStop.autoStop",
      { seconds: Math.max(1, Math.ceil(stopInMs / 1000)) },
    ];
  if (text === "woke" && seconds !== null)
    return [
      "callPrompt.woke.body",
      { minutes: Math.max(1, Math.round(seconds / 60)) },
    ];
  if (text !== "mistake" || seconds === null)
    return [`callPrompt.${text}.body`, {}];
  return seconds < 30
    ? ["callPrompt.mistake.short", { seconds }]
    : [
        "callPrompt.mistake.quiet",
        { minutes: Math.max(1, Math.round(seconds / 60)) },
      ];
}

/** The backend sent the card on screen again with new facts (a "stop?" that heard sound): the
 *  countdown and the hover go on. */
export function isUpdate(
  shown: CallPromptPayload | null,
  next: CallPromptPayload | null,
): boolean {
  return (
    shown !== null &&
    next !== null &&
    shown.kind === next.kind &&
    shown.app_id === next.app_id &&
    shown.web_name === next.web_name &&
    shown.session_id === next.session_id
  );
}

/** The prompt window: shows the card the backend sends and reports the answer. */
export function CallPromptWindow() {
  const { t } = useTranslation();
  const [prompt, setPrompt] = useState<CallPromptPayload | null>(null);
  const [countdown, setCountdown] = useState(() => startCountdown(0));
  const hovered = useRef(false);
  const shown = useRef<CallPromptPayload | null>(null);

  const open = useCallback((payload: CallPromptPayload | null) => {
    const update = isUpdate(shown.current, payload);
    shown.current = payload;
    setPrompt(payload);
    if (update) return;
    hovered.current = false;
    if (payload) setCountdown(startCountdown(payload.countdown_ms));
  }, []);

  useEffect(() => {
    void commands.callPromptCurrent().then(open);
    const unlisten = listen<CallPromptPayload>("call-prompt", (event) =>
      open(event.payload),
    );
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, [open]);

  const answer = useCallback((value: CallPromptAnswer) => {
    shown.current = null;
    setPrompt(null);
    void commands.answerCallPrompt(value);
  }, []);

  useEffect(() => {
    if (!prompt) return;
    const timer = window.setInterval(() => {
      setCountdown((current) => advance(current, TICK_MS, hovered.current));
    }, TICK_MS);
    return () => window.clearInterval(timer);
  }, [prompt]);

  useEffect(() => {
    if (prompt && expired(countdown)) answer("expired");
  }, [prompt, countdown, answer]);

  if (!prompt) return null;
  const app = prompt.app_name;
  const text = CALL_PROMPT_TEXT[prompt.kind];
  const autoStop = prompt.kind === "ask_stop" && prompt.auto_stop;
  const [bodyKey, bodyOptions] = cardBody(
    text,
    prompt.seconds,
    autoStop ? countdown.remainingMs : null,
  );
  const labels: Record<CallPromptAnswer, string> = {
    record: t("callPrompt.record"),
    not_now: t("callPrompt.notNow"),
    mute_app: t("callPrompt.muteApp", { app }),
    stop: autoStop ? t("callPrompt.askStop.stopNow") : t("callPrompt.stop"),
    keep_recording: autoStop
      ? t("callPrompt.askStop.keep")
      : t("callPrompt.keepRecording"),
    stop_and_quit: t("callPrompt.stopAndQuit"),
    cancel: t("callPrompt.cancel"),
    record_again: t("callPrompt.recordAgain"),
    dismiss: t("callPrompt.dismiss"),
    keep: t("callPrompt.keep"),
    discard: t("callPrompt.discard"),
    resume: t("callPrompt.resume"),
    expired: "",
  };
  return (
    <CallPromptCard
      kind={prompt.kind}
      title={t(titleKey(prompt.kind, prompt.web_name), {
        app,
        web: prompt.web_name,
      })}
      body={t(bodyKey, bodyOptions)}
      actions={cardAnswers(prompt).map((value, index) => ({
        answer: value,
        label: labels[value],
        primary: index === 0,
      }))}
      remaining={fraction(countdown)}
      onAnswer={answer}
      onHover={(value) => {
        hovered.current = value;
      }}
    />
  );
}
