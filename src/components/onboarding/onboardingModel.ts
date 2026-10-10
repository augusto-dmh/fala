import type { ModelInfo, WindowsMicrophonePermissionStatus } from "@/bindings";

/** The three first-run steps, in order. */
export const ONBOARDING_STEPS = ["microphone", "shortcut", "ai"] as const;
export type OnboardingStepId = (typeof ONBOARDING_STEPS)[number];

/** The dictation model is fixed by ADR-0009; the wizard never asks for one. */
export const DICTATION_MODEL_ID = "parakeet-tdt-0.6b-v3";

export const nextStep = (
  steps: readonly OnboardingStepId[],
  step: OnboardingStepId,
): OnboardingStepId | null => steps[steps.indexOf(step) + 1] ?? null;

export const previousStep = (
  steps: readonly OnboardingStepId[],
  step: OnboardingStepId,
): OnboardingStepId | null => {
  const index = steps.indexOf(step);
  return index > 0 ? steps[index - 1] : null;
};

export type MicAccess = "ok" | "denied";

/** Only an explicit Windows denial blocks the step; anything else lets it go on. */
export const micAccess = (
  status: WindowsMicrophonePermissionStatus | null,
): MicAccess =>
  status?.supported && status.overall_access === "denied" ? "denied" : "ok";

/**
 * RMS of an `AnalyserNode` byte time-domain buffer (silence is 128), scaled so
 * normal speech fills most of the meter, clamped to 0..1.
 */
export const levelFromTimeDomain = (samples: Uint8Array): number => {
  if (samples.length === 0) return 0;
  let sum = 0;
  for (const sample of samples) {
    const centered = (sample - 128) / 128;
    sum += centered * centered;
  }
  return Math.min(1, Math.sqrt(sum / samples.length) * 4);
};

/**
 * The browser device whose label is the microphone picked in Fala, so the
 * meter listens to the same device the dictation uses. `undefined` falls back
 * to the system default.
 */
export const pickInputDeviceId = (
  devices: readonly Pick<MediaDeviceInfo, "deviceId" | "kind" | "label">[],
  selected: string | null | undefined,
): string | undefined => {
  if (!selected || selected === "Default" || selected === "default") {
    return undefined;
  }
  return devices.find(
    (device) => device.kind === "audioinput" && device.label === selected,
  )?.deviceId;
};

export interface ModelProgress {
  models: readonly Pick<ModelInfo, "id" | "is_downloaded">[];
  currentModel: string;
  downloading: Record<string, true>;
  verifying: Record<string, true>;
  extracting: Record<string, true>;
}

const inFlight = (state: ModelProgress, id: string) =>
  id in state.downloading || id in state.verifying || id in state.extracting;

/** What the wizard still has to do so the dictation model is ready. */
export const modelAction = (
  state: ModelProgress,
  id: string = DICTATION_MODEL_ID,
): "download" | "select" | "none" => {
  const model = state.models.find((m) => m.id === id);
  if (!model || inFlight(state, id) || state.currentModel === id) return "none";
  return model.is_downloaded ? "select" : "download";
};

export type ModelFooterState =
  | { kind: "pending" }
  | { kind: "downloading"; percent: number }
  | { kind: "verifying" }
  | { kind: "extracting" }
  | { kind: "ready" }
  | { kind: "failed" };

/**
 * The error the footer shows: the store's, or a generic one when the wizard's
 * own download or selection failed without the store recording any.
 */
export const footerError = (
  storeError: string | null,
  actionFailed: boolean,
): string | null => storeError ?? (actionFailed ? "failed" : null);

/** The model line in the wizard footer. */
export const modelFooterState = (
  state: ModelProgress & {
    progress: Record<string, { percentage: number }>;
    error: string | null;
  },
  id: string = DICTATION_MODEL_ID,
): ModelFooterState => {
  if (id in state.extracting) return { kind: "extracting" };
  if (id in state.verifying) return { kind: "verifying" };
  if (id in state.downloading) {
    return {
      kind: "downloading",
      percent: Math.round(state.progress[id]?.percentage ?? 0),
    };
  }
  const model = state.models.find((m) => m.id === id);
  if (model?.is_downloaded && state.currentModel === id) {
    return { kind: "ready" };
  }
  if (state.error) return { kind: "failed" };
  return { kind: "pending" };
};
