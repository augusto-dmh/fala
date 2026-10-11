import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { PathDisplay } from "../../ui/PathDisplay";
import { Button } from "../../ui/Button";
import type { PrivacyRow } from "./privacyModel";

interface PrivacyViewProps {
  rows: PrivacyRow[];
  /** The meeting audio folder; empty while the data folder is unknown. */
  audioDir: string;
  onOpenAudio: () => void;
  onOpenDecisions: () => void;
}

/** Read-only: what leaves this computer, where to, and whether it is on now. Never a key. */
export const PrivacyView: React.FC<PrivacyViewProps> = ({
  rows,
  audioDir,
  onOpenAudio,
  onOpenDecisions,
}) => {
  const { t } = useTranslation();
  const base = "settings.about.privacy";

  return (
    <SettingsGroup
      title={t(`${base}.title`)}
      description={t(`${base}.description`)}
    >
      {rows.map((row) => (
        <div
          key={row.channel}
          className="px-4 py-3 space-y-1"
          data-channel={row.channel}
        >
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h3 className="text-body text-text">
              {t(`${base}.channels.${row.channel}.name`)}
            </h3>
            <div className="flex flex-wrap gap-1">
              {row.statuses.map((status) => (
                <span
                  key={status}
                  data-status={status}
                  className="text-caption text-text-2 border border-border rounded px-1.5 py-0.5"
                >
                  {t(`${base}.status.${status}`)}
                </span>
              ))}
            </div>
          </div>
          <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 text-caption">
            <dt className="text-text-3">{t(`${base}.columns.sends`)}</dt>
            <dd className="text-text-2">
              {t(`${base}.channels.${row.channel}.sends`)}
            </dd>
            <dt className="text-text-3">{t(`${base}.columns.destination`)}</dt>
            <dd className="text-text-2">
              {t(`${base}.channels.${row.channel}.destination`, {
                provider: row.provider,
              })}
            </dd>
          </dl>
        </div>
      ))}

      <SettingContainer
        title={t(`${base}.keys.title`)}
        description={t(`${base}.keys.description`)}
        grouped={true}
      >
        {null}
      </SettingContainer>

      <SettingContainer
        title={t(`${base}.audio.title`)}
        description={t(`${base}.audio.description`)}
        grouped={true}
        layout="stacked"
      >
        <PathDisplay
          path={audioDir}
          onOpen={onOpenAudio}
          disabled={!audioDir}
        />
      </SettingContainer>

      <SettingContainer
        title={t(`${base}.decisions.title`)}
        description={t(`${base}.decisions.description`)}
        grouped={true}
      >
        <Button variant="secondary" size="md" onClick={onOpenDecisions}>
          {t(`${base}.decisions.button`)}
        </Button>
      </SettingContainer>
    </SettingsGroup>
  );
};
