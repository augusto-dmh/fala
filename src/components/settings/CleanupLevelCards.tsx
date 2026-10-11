import React, { useId, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { CleanupLevel } from "@/bindings";
import {
  CLEANUP_LEVELS,
  DEFAULT_CLEANUP_LEVEL,
  levelForKey,
} from "./cleanupLevelModel";

interface CleanupLevelCardsProps {
  value: CleanupLevel;
  onChange: (level: CleanupLevel) => void;
  /** The AI is off or has no key: the level is kept but does not apply now. */
  dimmed: boolean;
}

/** How much the dictation AI rewrites: four radio cards, each with the same example
 *  dictation transformed. The examples are static i18n text, never generated. */
export const CleanupLevelCards: React.FC<CleanupLevelCardsProps> = ({
  value,
  onChange,
  dimmed,
}) => {
  const { t } = useTranslation();
  const titleId = useId();
  const cards = useRef<Partial<Record<CleanupLevel, HTMLButtonElement>>>({});

  const handleKeyDown = (event: React.KeyboardEvent) => {
    const next = levelForKey(value, event.key);
    if (!next) return;
    event.preventDefault();
    onChange(next);
    cards.current[next]?.focus();
  };

  return (
    <div className="px-4 py-3 space-y-3">
      <div>
        <h3 id={titleId} className="text-body text-text">
          {t("settings.llm.cleanup.title")}
        </h3>
        <p className="text-caption text-text-2">
          {t("settings.llm.cleanup.description")}
        </p>
      </div>
      <p className="text-caption text-text-2">
        <span className="font-semibold">
          {t("settings.llm.cleanup.rawLabel")}
        </span>{" "}
        <q>{t("settings.llm.cleanup.raw")}</q>
      </p>
      {dimmed && (
        <p className="text-caption text-text-2" data-cleanup-inactive>
          {t("settings.llm.cleanup.inactive")}
        </p>
      )}
      <div
        role="radiogroup"
        aria-labelledby={titleId}
        className={`grid grid-cols-1 sm:grid-cols-2 gap-2 ${dimmed ? "opacity-60" : ""}`}
      >
        {CLEANUP_LEVELS.map((level) => {
          const checked = level === value;
          return (
            <button
              key={level}
              ref={(node) => {
                if (node) cards.current[level] = node;
              }}
              type="button"
              role="radio"
              aria-checked={checked}
              tabIndex={checked ? 0 : -1}
              data-level={level}
              onClick={() => onChange(level)}
              onKeyDown={handleKeyDown}
              className={`flex flex-col gap-1 text-left rounded-lg border p-3 bg-surface-2 transition-colors focus-visible:focus-ring cursor-pointer ${
                checked
                  ? "border-accent ring-1 ring-accent"
                  : "border-border hover:bg-text/5"
              }`}
            >
              <span className="flex items-center gap-2">
                <span
                  aria-hidden="true"
                  className={`w-3 h-3 shrink-0 rounded-full border ${
                    checked ? "border-accent bg-accent" : "border-text-3"
                  }`}
                />
                <span className="text-body font-semibold text-text">
                  {t(`settings.llm.cleanup.levels.${level}.label`)}
                </span>
                {level === DEFAULT_CLEANUP_LEVEL && (
                  <span className="text-caption text-text-3">
                    {t("settings.llm.cleanup.default")}
                  </span>
                )}
              </span>
              <span className="text-caption text-text-2">
                {t(`settings.llm.cleanup.levels.${level}.description`)}
              </span>
              <span className="mt-1 text-body text-text">
                {t(`settings.llm.cleanup.levels.${level}.example`)}
              </span>
            </button>
          );
        })}
      </div>
      <p className="text-caption text-text-2">
        {t("settings.llm.cleanup.footer")}
      </p>
    </div>
  );
};
