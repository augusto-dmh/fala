import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/Button";
import { createMicLevelSession } from "./micLevelSession";

interface MicLevelMeterProps {
  /** The microphone picked in Fala (cpal device name, or "Default"). */
  microphone: string | null | undefined;
}

type MeterState = "idle" | "starting" | "listening";

/**
 * A live level meter for the first-run microphone check. It listens through
 * the webview (getUserMedia + AnalyserNode) only while the person tests; the
 * audio never leaves this window. The backend's own levels go to the pill
 * window only, during a dictation.
 */
export const MicLevelMeter: React.FC<MicLevelMeterProps> = ({ microphone }) => {
  const { t } = useTranslation();
  const [state, setState] = useState<MeterState>("idle");
  const [level, setLevel] = useState(0);
  const [unavailable, setUnavailable] = useState<string | null>(null);
  const sessionRef = useRef<ReturnType<typeof createMicLevelSession> | null>(
    null,
  );
  const session = () =>
    (sessionRef.current ??= createMicLevelSession(
      {
        mediaDevices: navigator.mediaDevices,
        createContext: () => new AudioContext(),
        requestFrame: (tick) => requestAnimationFrame(tick),
        cancelFrame: (handle) => cancelAnimationFrame(handle),
      },
      setLevel,
    ));

  const stop = () => {
    session().stop();
    setState("idle");
  };

  const start = async () => {
    setUnavailable(null);
    setState("starting");
    try {
      if ((await session().start(microphone)) === "listening") {
        setState("listening");
      }
    } catch (error) {
      session().stop();
      setState("idle");
      setUnavailable(error instanceof Error ? error.name : String(error));
    }
  };

  // Restart on a device change while testing.
  useEffect(() => {
    if (state === "listening") void start();
  }, [microphone]);

  // Release the microphone on unmount, including a start still pending.
  useEffect(() => () => sessionRef.current?.stop(), []);

  const percent = Math.round(level * 100);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-3">
        <div
          role="meter"
          aria-label={t("onboarding.microphone.meterLabel")}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={percent}
          className="h-2 flex-1 overflow-hidden rounded-full bg-surface-2"
        >
          <div
            className="h-full rounded-full bg-accent transition-[width] duration-75"
            style={{ width: `${percent}%` }}
          />
        </div>
        <Button
          variant="secondary"
          size="md"
          disabled={state === "starting"}
          onClick={() => (state === "listening" ? stop() : void start())}
        >
          {state === "listening"
            ? t("onboarding.microphone.stop")
            : t("onboarding.microphone.test")}
        </Button>
      </div>
      {unavailable && (
        <p className="text-caption text-text-2">
          {t("onboarding.microphone.unavailable", { reason: unavailable })}
        </p>
      )}
    </div>
  );
};
