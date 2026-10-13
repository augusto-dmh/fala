import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type MeetingError,
  type MeetingKeys,
  type Result,
} from "@/bindings";
import { KeyField } from "../meeting/KeyField";
import { errorDetail, errorKey } from "../meeting/meetingModel";
import { SettingContainer } from "../ui/SettingContainer";

type KeyName = "elevenlabs" | "anthropic";

/** Settings > General > Meetings: the ElevenLabs key (transcription) and the Anthropic key
 *  (notes), stored in the keyring and never shown. */
export const MeetingKeySetting: React.FC = () => {
  const { t } = useTranslation();
  const [keys, setKeys] = useState<MeetingKeys | null>(null);
  // Each key shows its own failure, under its own field.
  const [errors, setErrors] = useState<Partial<Record<KeyName, MeetingError>>>(
    {},
  );
  useEffect(() => {
    void commands
      .meetingKeys()
      .then((r) => r.status === "ok" && setKeys(r.data));
  }, []);

  const save = async (
    name: KeyName,
    call: () => Promise<Result<MeetingKeys, MeetingError>>,
  ) => {
    setErrors((current) => ({ ...current, [name]: undefined }));
    const result = await call();
    if (result.status === "ok") {
      setKeys(result.data);
      return true;
    }
    setErrors((current) => ({ ...current, [name]: result.error }));
    return false;
  };
  const failure = (name: KeyName) => {
    const error = errors[name];
    return (
      error && (
        <p role="alert" className="mt-1 text-caption text-danger">
          {t(errorKey(error), { detail: errorDetail(error) })}
        </p>
      )
    );
  };

  return (
    <>
      <SettingContainer
        title={t("meeting.key.label")}
        description={
          keys?.elevenlabs ? t("meeting.key.saved") : t("meeting.key.missing")
        }
        layout="stacked"
        grouped={true}
        anchor="meetings-key"
      >
        <KeyField
          saved={keys?.elevenlabs}
          onSave={(key) =>
            save("elevenlabs", () => commands.setMeetingTranscriptionKey(key))
          }
        />
        {failure("elevenlabs")}
      </SettingContainer>
      <SettingContainer
        title={t("meeting.notesKey.label")}
        description={
          keys?.anthropic
            ? t("meeting.notesKey.saved")
            : t("meeting.notesKey.missing")
        }
        layout="stacked"
        grouped={true}
      >
        <KeyField
          label={t("meeting.notesKey.label")}
          saved={keys?.anthropic}
          onSave={(key) =>
            save("anthropic", () => commands.setMeetingNotesKey(key))
          }
        />
        {failure("anthropic")}
      </SettingContainer>
    </>
  );
};
