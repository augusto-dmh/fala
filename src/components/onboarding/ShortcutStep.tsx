import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { commands } from "@/bindings";
import { unheardKey } from "./onboardingModel";
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
  // A test dictation that came out empty says why under the field, not only in the overlay
  // at the bottom of the screen; the next text in the field clears it.
  const [notice, setNotice] = useState<string | null>(null);
  useEffect(() => {
    if (preview) return;
    const unlisten = listen<{ muted: boolean }>(
      "dictation-unheard",
      ({ payload }) => setNotice(unheardKey(payload.muted)),
    );
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, [preview]);

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
              className="min-w-8 whitespace-nowrap rounded-lg border border-border border-b-2 bg-surface-2 px-2 py-1 text-center font-sans text-body font-semibold text-text"
            >
              {key}
            </kbd>
          ))}
        </div>
        <textarea
          autoFocus
          rows={3}
          aria-label={t("onboarding.shortcut.testLabel")}
          aria-describedby={notice ? "shortcut-test-notice" : undefined}
          placeholder={t("onboarding.shortcut.testPlaceholder")}
          onInput={() => setNotice(null)}
          className="w-full resize-none rounded-md border border-border bg-surface-1 px-3 py-2 text-body text-text placeholder:text-text-3 focus:outline-none focus:border-b-accent focus:shadow-[inset_0_-1px_0_var(--color-accent)]"
        />
        {/* Always in the page, so a screen reader announces the text when it appears. */}
        <p
          id="shortcut-test-notice"
          role="status"
          className="text-caption text-warn empty:sr-only"
        >
          {notice ? t(notice) : ""}
        </p>
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
