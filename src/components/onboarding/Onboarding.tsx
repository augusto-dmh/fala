import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { useModelStore } from "../../stores/modelStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { Button } from "../ui/Button";
import FalaTextLogo from "../icons/FalaTextLogo";
import { AiStep } from "./AiStep";
import { MicrophoneStep } from "./MicrophoneStep";
import { ModelFooter } from "./ModelFooter";
import { ShortcutStep } from "./ShortcutStep";
import {
  DICTATION_MODEL_ID,
  footerError,
  ONBOARDING_STEPS,
  micAccess,
  modelAction,
  modelFooterState,
  nextStep,
  previousStep,
  type MicAccess,
  type OnboardingStepId,
} from "./onboardingModel";

interface OnboardingProps {
  onFinish: () => void;
  /** Defaults to all three; a returning user with a denied mic sees only the first. */
  steps?: readonly OnboardingStepId[];
  initialStep?: OnboardingStepId;
  /** Download and select the dictation model. Off for a returning user, who
   *  already has a model and only needs the microphone back. */
  prepareModel?: boolean;
  /** Debug preview: never downloads, selects or polls anything. */
  preview?: boolean;
}

const MIC_POLL_MS = 1000;

/** First run in three steps (microphone, shortcut, optional AI) while the
 *  dictation model downloads in the background (ADR-0009: no model choice). */
const Onboarding: React.FC<OnboardingProps> = ({
  onFinish,
  steps = ONBOARDING_STEPS,
  initialStep,
  prepareModel = true,
  preview = false,
}) => {
  const { t } = useTranslation();
  const [step, setStep] = useState<OnboardingStepId>(initialStep ?? steps[0]);
  const [access, setAccess] = useState<MicAccess>("ok");
  const [waitingForAccess, setWaitingForAccess] = useState(false);
  const refreshAudioDevices = useSettingsStore((s) => s.refreshAudioDevices);
  const {
    models,
    currentModel,
    downloadingModels,
    verifyingModels,
    extractingModels,
    downloadProgress,
    error,
    downloadModel,
    selectModel,
  } = useModelStore();
  // Each automatic action runs once; "try again" clears the record.
  const attempted = useRef(new Set<string>());
  const [retries, setRetries] = useState(0);
  // A download or selection that failed without the store recording an error
  // (an IPC exception) still has to offer "try again".
  const [actionFailed, setActionFailed] = useState(false);

  const progress = {
    models,
    currentModel,
    downloading: downloadingModels,
    verifying: verifyingModels,
    extracting: extractingModels,
  };
  const footer = modelFooterState({
    ...progress,
    progress: downloadProgress,
    error: footerError(error, actionFailed),
  });
  const action = modelAction(progress);

  // Start the Parakeet download as soon as the wizard opens, then select it.
  useEffect(() => {
    if (
      preview ||
      !prepareModel ||
      action === "none" ||
      attempted.current.has(action)
    ) {
      return;
    }
    attempted.current.add(action);
    const run =
      action === "download"
        ? downloadModel(DICTATION_MODEL_ID)
        : selectModel(DICTATION_MODEL_ID);
    void run.then((ok) => {
      if (!ok) setActionFailed(true);
    });
  }, [preview, prepareModel, action, retries, downloadModel, selectModel]);

  // Windows can deny the microphone to desktop apps; poll while it does.
  useEffect(() => {
    if (preview) return;
    let cancelled = false;
    const check = async () => {
      try {
        const next = micAccess(
          await commands.getWindowsMicrophonePermissionStatus(),
        );
        if (cancelled) return;
        setAccess(next);
        if (next === "ok" && access === "denied") {
          setWaitingForAccess(false);
          void refreshAudioDevices();
        }
      } catch (e) {
        console.warn("Failed to check microphone access:", e);
      }
    };
    void check();
    const timer =
      access === "denied" ? setInterval(check, MIC_POLL_MS) : undefined;
    return () => {
      cancelled = true;
      if (timer) clearInterval(timer);
    };
  }, [preview, access, refreshAudioDevices]);

  const openPrivacySettings = async () => {
    if (preview) return;
    const result = await commands.openMicrophonePrivacySettings();
    if (result.status === "ok") setWaitingForAccess(true);
  };

  const retry = () => {
    attempted.current.clear();
    setActionFailed(false);
    setRetries((n) => n + 1);
  };

  const next = nextStep(steps, step);
  const previous = previousStep(steps, step);
  const blocked = step === "microphone" && access === "denied";

  return (
    <div className="flex h-screen w-full flex-col bg-surface-0">
      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto flex w-full max-w-[560px] flex-col gap-6 px-6 py-8 max-[840px]:px-4">
          <FalaTextLogo width={96} />
          <header className="flex flex-col gap-1">
            {steps.length > 1 && (
              <p className="text-caption text-text-2 tabular-nums">
                {t("onboarding.progress", {
                  current: steps.indexOf(step) + 1,
                  total: steps.length,
                })}
              </p>
            )}
            <h1 className="font-display text-title font-semibold text-text">
              {t(`onboarding.${step}.title`)}
            </h1>
            <p className="text-body text-text-2">
              {t(`onboarding.${step}.description`)}
            </p>
          </header>
          {step === "microphone" && (
            <MicrophoneStep
              access={access}
              waiting={waitingForAccess}
              onOpenPrivacySettings={openPrivacySettings}
            />
          )}
          {step === "shortcut" && (
            <ShortcutStep
              modelReady={footer.kind === "ready"}
              preview={preview}
            />
          )}
          {step === "ai" && <AiStep onFinish={onFinish} />}
        </div>
      </div>
      <footer className="flex items-center justify-between gap-4 border-t border-border bg-surface-1 px-6 py-3 max-[840px]:px-4">
        {prepareModel ? (
          <ModelFooter state={footer} onRetry={retry} />
        ) : (
          <span />
        )}
        <div className="flex shrink-0 gap-2">
          {previous && (
            <Button variant="secondary" onClick={() => setStep(previous)}>
              {t("onboarding.nav.back")}
            </Button>
          )}
          {next && (
            <Button
              variant="primary"
              disabled={blocked}
              onClick={() => setStep(next)}
            >
              {t("onboarding.nav.next")}
            </Button>
          )}
          {!next && step !== "ai" && (
            <Button variant="primary" disabled={blocked} onClick={onFinish}>
              {t("onboarding.nav.done")}
            </Button>
          )}
        </div>
      </footer>
    </div>
  );
};

export default Onboarding;
