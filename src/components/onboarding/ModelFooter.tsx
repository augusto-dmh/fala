import React from "react";
import { useTranslation } from "react-i18next";
import type { ModelFooterState } from "./onboardingModel";

interface ModelFooterProps {
  state: ModelFooterState;
  onRetry: () => void;
}

const DOT: Record<ModelFooterState["kind"], string> = {
  pending: "bg-text-3",
  downloading: "bg-text-2",
  verifying: "bg-text-2",
  extracting: "bg-text-2",
  ready: "bg-ok",
  failed: "bg-danger",
};

/** The dictation model's download, in the wizard footer. */
export const ModelFooter: React.FC<ModelFooterProps> = ({ state, onRetry }) => {
  const { t } = useTranslation();
  const label =
    state.kind === "downloading"
      ? t("onboarding.footer.downloading", { percent: state.percent })
      : t(`onboarding.footer.${state.kind}`);

  return (
    <div className="flex min-w-0 items-center gap-2 text-caption text-text-2">
      <span
        aria-hidden="true"
        className={`h-2 w-2 shrink-0 rounded-full ${DOT[state.kind]}`}
      />
      <span
        className={`truncate tabular-nums ${state.kind === "failed" ? "text-danger" : ""}`}
      >
        {label}
      </span>
      {state.kind === "downloading" && (
        <div
          role="progressbar"
          aria-label={label}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={state.percent}
          className="h-1 w-24 shrink-0 overflow-hidden rounded-full bg-surface-2"
        >
          <div
            className="h-full rounded-full bg-accent"
            style={{ width: `${state.percent}%` }}
          />
        </div>
      )}
      {state.kind === "failed" && (
        <button
          type="button"
          onClick={onRetry}
          className="rounded-lg px-1 text-text underline cursor-pointer focus-visible:focus-ring"
        >
          {t("onboarding.footer.retry")}
        </button>
      )}
    </div>
  );
};
