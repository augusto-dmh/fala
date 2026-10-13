import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  events,
  type MeetingError,
  type MeetingKeys,
  type Result,
} from "@/bindings";
import { useSettings } from "../../hooks/useSettings";
import { KeyField } from "../meeting/KeyField";
import { errorDetail, errorKey, levelPercent } from "../meeting/meetingModel";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";
import { MicLevelMeter } from "./MicLevelMeter";

/** The level of the sound a meeting records, while the person tests it (nothing is saved). */
const SystemLevelMeter: React.FC<{ preview: boolean }> = ({ preview }) => {
  const { t } = useTranslation();
  const [listening, setListening] = useState(false);
  // One start at a time: a second click before the output opens does nothing.
  const [starting, setStarting] = useState(false);
  const [level, setLevel] = useState(0);
  const [error, setError] = useState<MeetingError | null>(null);
  useEffect(() => {
    if (preview) return;
    const unlisten = events.meetingSoundLevel.listen(({ payload }) => {
      setLevel(payload.level);
      // A minute later the test stops by itself: the button offers it again.
      if (payload.ended) setListening(false);
    });
    return () => {
      void unlisten.then((stop) => stop());
      void commands.stopMeetingSoundCheck();
    };
  }, [preview]);

  const toggle = async () => {
    setError(null);
    if (listening) {
      await commands.stopMeetingSoundCheck();
      setListening(false);
      setLevel(0);
      return;
    }
    if (preview) return;
    setStarting(true);
    const result = await commands.startMeetingSoundCheck();
    setStarting(false);
    if (result.status === "ok") setListening(true);
    else setError(result.error);
  };

  // The same curve as the recorder's meters.
  const percent = levelPercent(level);
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-3">
        <div
          role="meter"
          aria-label={t("onboarding.meetings.systemLabel")}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={percent}
          className="h-2 flex-1 overflow-hidden rounded-full bg-surface-2"
        >
          <div
            className="h-full rounded-full bg-accent transition-[width] duration-75"
            style={{ width: `${percent}%` }}
          />
        </div>
        <Button
          variant="secondary"
          disabled={starting}
          onClick={() => void toggle()}
        >
          {listening
            ? t("onboarding.meetings.stopSound")
            : t("onboarding.meetings.testSound")}
        </Button>
      </div>
      {error !== null && (
        <p role="alert" className="text-caption text-danger">
          {t(errorKey(error), { detail: errorDetail(error) })}
        </p>
      )}
    </div>
  );
};

interface MeetingsStepProps {
  /** Debug preview: the sound check stays off and nothing is stored or accepted. */
  preview?: boolean;
}

/**
 * Step 4 (optional): the two meters of a meeting, the third-party notice (ADR-0005) and the
 * keys of the transcription and the notes (keyring, ADR-0008); the footer skips or finishes.
 */
export const MeetingsStep: React.FC<MeetingsStepProps> = ({
  preview = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, refreshSettings } = useSettings();
  const [keys, setKeys] = useState<MeetingKeys | null>(null);
  // Accepted before (a recording, or this step on an earlier run): nothing to accept again.
  const accepted = getSetting("meeting_consent_accepted_at") != null;
  const [error, setError] = useState<MeetingError | null>(null);
  useEffect(() => {
    if (preview) return;
    void commands
      .meetingKeys()
      .then((r) => r.status === "ok" && setKeys(r.data));
  }, [preview]);

  // A command that throws (IPC) shows the generic error instead of failing silently.
  const run = async <T,>(
    call: () => Promise<Result<T, MeetingError>>,
    then: (data: T) => void,
  ) => {
    if (preview) return;
    setError(null);
    try {
      const result = await call();
      if (result.status === "ok") then(result.data);
      else setError(result.error);
    } catch (e) {
      setError({ kind: "storage", detail: String(e) } as MeetingError);
    }
  };
  const save = (call: () => Promise<Result<MeetingKeys, MeetingError>>) =>
    run(call, setKeys);
  const accept = () =>
    run(commands.acceptMeetingConsent, () => void refreshSettings());

  return (
    <div className="flex flex-col gap-4">
      <SettingContainer
        title={t("onboarding.meetings.micLabel")}
        description={t("onboarding.meetings.micDescription")}
        layout="stacked"
      >
        <MicLevelMeter microphone={getSetting("selected_microphone")} />
      </SettingContainer>
      <SettingContainer
        title={t("onboarding.meetings.systemLabel")}
        description={t("onboarding.meetings.systemDescription")}
        layout="stacked"
      >
        <SystemLevelMeter preview={preview} />
      </SettingContainer>
      <div
        role="note"
        className="flex flex-col gap-3 rounded-lg border border-border bg-surface-1 p-4"
      >
        <p className="text-body font-semibold">{t("meeting.consent.title")}</p>
        <p className="text-body text-text-2">{t("meeting.consent.body")}</p>
        <Button
          variant="secondary"
          className="self-start"
          disabled={accepted}
          onClick={() => void accept()}
        >
          {accepted
            ? t("onboarding.meetings.accepted")
            : t("onboarding.meetings.accept")}
        </Button>
      </div>
      <SettingContainer
        title={t("meeting.key.label")}
        description={
          keys?.elevenlabs ? t("meeting.key.saved") : t("meeting.key.missing")
        }
        layout="stacked"
      >
        <KeyField
          saved={keys?.elevenlabs}
          onSave={(key) =>
            void save(() => commands.setMeetingTranscriptionKey(key))
          }
        />
      </SettingContainer>
      <SettingContainer
        title={t("onboarding.meetings.anthropicLabel")}
        description={
          keys?.anthropic
            ? t("meeting.notesKey.saved")
            : t("onboarding.meetings.anthropicDescription")
        }
        layout="stacked"
      >
        <KeyField
          label={t("onboarding.meetings.anthropicLabel")}
          saved={keys?.anthropic}
          onSave={(key) => void save(() => commands.setMeetingNotesKey(key))}
        />
      </SettingContainer>
      {error !== null && (
        <p role="alert" className="text-caption text-danger">
          {t(errorKey(error), { detail: errorDetail(error) })}
        </p>
      )}
    </div>
  );
};
