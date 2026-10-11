import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { commands, events } from "../../bindings";
import type { UpdateStatus } from "../../bindings";
import { useSettings } from "../../hooks/useSettings";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";
import { PORTABLE_RELEASES_URL, RELEASE_NOTES_URL } from "./portableInstaller";

/** The row in About: the update status and what can be done about it (ADR-0018). The checks,
 * the download and the install all run in Rust; this only asks. */
const UpdateChecker: React.FC = () => {
  const { t } = useTranslation();
  const { settings, updateChecksLocked } = useSettings();
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [manual, setManual] = useState(false);
  const [showPortable, setShowPortable] = useState(false);

  const enabled =
    (settings?.update_checks_enabled ?? false) && updateChecksLocked === false;

  useEffect(() => {
    let cancelled = false;
    void commands.updateStatus().then((s) => !cancelled && setStatus(s));
    const unlisten = events.updateStatus.listen((e) => setStatus(e.payload));
    return () => {
      cancelled = true;
      void unlisten.then((fn) => fn());
    };
  }, []);

  const phase = status?.phase ?? "idle";
  const version = status?.version ?? "";
  const percent =
    status?.total && status.total > 0
      ? Math.min(100, Math.round((status.downloaded / status.total) * 100))
      : 0;

  const checkNow = async () => {
    setManual(true);
    setStatus(await commands.checkForUpdatesNow());
  };

  const install = async () => {
    if (phase === "available") {
      if (await commands.isPortable()) setShowPortable(true);
      else await openUrl(status?.installer_url ?? PORTABLE_RELEASES_URL);
      return;
    }
    // A refusal comes back in the status event, which names what is running.
    await commands.installUpdate();
  };

  const statusText = () => {
    if (status?.refusal)
      return t(`settings.about.updates.refusal.${status.refusal}`);
    if (!enabled && phase !== "ready")
      return t("settings.about.updates.disabled");
    switch (phase) {
      case "checking":
        return t("settings.about.updates.checking");
      case "downloading":
        return t("settings.about.updates.downloading", { version, percent });
      case "ready":
        return t("settings.about.updates.ready", { version });
      case "available":
        return t("settings.about.updates.available", { version });
      case "installing":
        return t("settings.about.updates.installing");
      case "failed":
        return t("settings.about.updates.failed");
      case "up_to_date":
        return t("settings.about.updates.upToDate");
      default:
        return manual
          ? t("settings.about.updates.upToDate")
          : t("settings.about.updates.idle");
    }
  };

  const offered = phase === "ready" || phase === "available";

  return (
    <>
      <SettingContainer
        title={t("settings.about.updates.title")}
        description={statusText()}
        grouped={true}
      >
        <div className="flex gap-2">
          {offered && (
            <>
              <Button
                variant="ghost"
                size="md"
                onClick={() => openUrl(`${RELEASE_NOTES_URL}${version}`)}
              >
                {t("settings.about.updates.notes")}
              </Button>
              <Button
                variant="secondary"
                size="md"
                onClick={() => commands.dismissUpdate()}
              >
                {t("settings.about.updates.later")}
              </Button>
              <Button variant="primary" size="md" onClick={install}>
                {phase === "ready"
                  ? t("settings.about.updates.restart")
                  : t("settings.about.updates.download")}
              </Button>
            </>
          )}
          {!offered && enabled && (
            <Button
              variant="secondary"
              size="md"
              disabled={phase === "checking" || phase === "downloading"}
              onClick={checkNow}
            >
              {t("settings.about.updates.checkNow")}
            </Button>
          )}
        </div>
      </SettingContainer>
      {showPortable && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="bg-background border border-mid-gray/20 rounded-lg p-6 max-w-md w-full mx-4 space-y-4">
            <h2 className="text-base font-semibold">
              {t("settings.about.updates.portable.title")}
            </h2>
            <p className="text-sm text-text/70">
              {t("settings.about.updates.portable.message")}
            </p>
            <div className="flex gap-2 justify-end">
              <Button
                variant="secondary"
                size="md"
                onClick={() => setShowPortable(false)}
              >
                {t("common.close")}
              </Button>
              <Button
                variant="primary"
                size="md"
                onClick={() => {
                  void openUrl(status?.installer_url ?? PORTABLE_RELEASES_URL);
                  setShowPortable(false);
                }}
              >
                {t("settings.about.updates.portable.button")}
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};

export default UpdateChecker;
