import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import {
  commands,
  type CallPromptAnswer,
  type CallPromptPayload,
} from "@/bindings";
import {
  CALL_PROMPT_ANSWERS,
  CALL_PROMPT_TEXT,
  CallPromptCard,
  titleKey,
} from "./CallPromptCard";
import { advance, expired, fraction, startCountdown } from "./countdown";

const TICK_MS = 100;

/** The body key: a short recording and an almost silent long one ask for different reasons;
 *  the wake card says how long the PC slept. */
export function mistakeBody(
  text: string,
  seconds: number | null,
): [string, Record<string, unknown>] {
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

/** The prompt window: shows the card the backend sends and reports the answer. */
export function CallPromptWindow() {
  const { t } = useTranslation();
  const [prompt, setPrompt] = useState<CallPromptPayload | null>(null);
  const [countdown, setCountdown] = useState(() => startCountdown(0));
  const hovered = useRef(false);

  const open = useCallback((payload: CallPromptPayload | null) => {
    hovered.current = false;
    setPrompt(payload);
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
  const [bodyKey, bodyOptions] = mistakeBody(text, prompt.seconds);
  const labels: Record<CallPromptAnswer, string> = {
    record: t("callPrompt.record"),
    not_now: t("callPrompt.notNow"),
    mute_app: t("callPrompt.muteApp", { app }),
    stop: t("callPrompt.stop"),
    keep_recording: t("callPrompt.keepRecording"),
    stop_and_quit: t("callPrompt.stopAndQuit"),
    cancel: t("callPrompt.cancel"),
    record_again: t("callPrompt.recordAgain"),
    dismiss: t("callPrompt.dismiss"),
    keep: t("callPrompt.keep"),
    discard: t("callPrompt.discard"),
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
      actions={CALL_PROMPT_ANSWERS[prompt.kind].map((value, index) => ({
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
