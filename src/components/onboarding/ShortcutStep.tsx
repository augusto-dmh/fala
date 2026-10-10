import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { formatKeyCombination } from "../../lib/utils/keyboard";
import { ShortcutInput } from "../settings/ShortcutInput";
import { ShortcutActivationSetting } from "../settings/ShortcutActivation";
import { useSettings } from "../../hooks/useSettings";

interface ShortcutStepProps {
  /** The dictation model is downloaded and selected. */
  modelReady: boolean;
  preview?: boolean;
}

/** Step 2: a real dictation into a test field, with the shortcut and the
 *  activation mode changeable right there. */
export const ShortcutStep: React.FC<ShortcutStepProps> = ({
  modelReady,
  preview = false,
}) => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const binding = getSetting("bindings")?.transcribe?.current_binding ?? "";
  const keys = binding
    ? formatKeyCombination(binding, "unknown").split(" + ")
    : [];
  const toggle = getSetting("shortcut_activation") === "toggle";

  // The shortcut has to work before the main window: register it (and the
  // paste backend) now. Both commands are no-ops once done.
  useEffect(() => {
    if (preview) return;
    Promise.all([
      commands.initializeEnigo(),
      commands.initializeShortcuts(),
    ]).catch((e) => console.warn("Failed to initialize shortcuts:", e));
  }, [preview]);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-3 rounded-lg border border-border bg-surface-1 p-4">
        <p className="text-body text-text">
          {toggle
            ? t("onboarding.shortcut.instructionToggle")
            : t("onboarding.shortcut.instruction")}
        </p>
        <div className="flex flex-wrap items-center gap-1.5">
          {keys.map((key) => (
            <kbd
              key={key}
              className="min-w-8 rounded-lg border border-border border-b-2 bg-surface-2 px-2 py-1 text-center font-sans text-body font-semibold text-text"
            >
              {key}
            </kbd>
          ))}
        </div>
        <textarea
          autoFocus
          rows={3}
          aria-label={t("onboarding.shortcut.testLabel")}
          placeholder={t("onboarding.shortcut.testPlaceholder")}
          className="w-full resize-none rounded-md border border-border bg-surface-1 px-3 py-2 text-body text-text placeholder:text-text-3 focus:outline-none focus:border-b-accent focus:shadow-[inset_0_-1px_0_var(--color-accent)]"
        />
        {!modelReady && (
          <p className="text-caption text-text-2">
            {t("onboarding.shortcut.waitModel")}
          </p>
        )}
      </div>
      <div className="flex flex-col gap-1">
        <ShortcutInput shortcutId="transcribe" descriptionMode="inline" />
        <ShortcutActivationSetting descriptionMode="inline" />
      </div>
    </div>
  );
};
