import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type MeetingError, type MeetingKeys } from "@/bindings";
import { KeyField } from "../meeting/KeyField";
import { errorDetail, errorKey } from "../meeting/meetingModel";
import { SettingContainer } from "../ui/SettingContainer";

/** Settings > General > Meetings: the ElevenLabs key, stored in the keyring and never shown. */
export const MeetingKeySetting: React.FC = () => {
  const { t } = useTranslation();
  const [keys, setKeys] = useState<MeetingKeys | null>(null);
  const [error, setError] = useState<MeetingError | null>(null);
  useEffect(() => {
    void commands
      .meetingKeys()
      .then((r) => r.status === "ok" && setKeys(r.data));
  }, []);

  const save = async (key: string) => {
    setError(null);
    const result = await commands.setMeetingTranscriptionKey(key);
    if (result.status === "ok") setKeys(result.data);
    else setError(result.error);
  };

  return (
    <SettingContainer
      title={t("meeting.key.label")}
      description={
        keys?.elevenlabs ? t("meeting.key.saved") : t("meeting.key.missing")
      }
      layout="stacked"
      grouped={true}
    >
      <KeyField onSave={(key) => void save(key)} />
      {error !== null && (
        <p role="alert" className="mt-1 text-caption text-danger">
          {t(errorKey(error), { detail: errorDetail(error) })}
        </p>
      )}
    </SettingContainer>
  );
};
