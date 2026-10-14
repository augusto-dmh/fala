import React from "react";
import { useTranslation } from "react-i18next";
import { MicrophoneSelector } from "../MicrophoneSelector";
import { ChannelSelector } from "../ChannelSelector";
import { KeyboardShortcuts } from "../KeyboardShortcuts";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { OutputDeviceSelector } from "../OutputDeviceSelector";
import { AudioFeedback } from "../AudioFeedback";
import { useSettings } from "../../../hooks/useSettings";
import { VolumeSlider } from "../VolumeSlider";
import { MuteWhileRecording } from "../MuteWhileRecording";
import { PauseMediaOnMeeting, PauseMediaWhileRecording } from "../PauseMedia";
import { ModelSettingsCard } from "./ModelSettingsCard";
import { LlmSettings } from "../LlmSettings";
import { CallPromptSettings } from "../CallPromptSettings";
import { MeetingAudioSettings } from "../MeetingAudioSettings";
import { MeetingKeySetting } from "../MeetingKeySetting";
import { NotesContextSettings } from "../../meeting/NotesContextSettings";
import { MeetingTemplatesSettings } from "../../meeting/TemplatesSettings";
import { AppLanguageSelector } from "../AppLanguageSelector";
import { ThemeSelector } from "../ThemeSelector";

export const GeneralSettings: React.FC = () => {
  const { t } = useTranslation();
  const { audioFeedbackEnabled } = useSettings();
  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.general.title")}>
        <KeyboardShortcuts />
      </SettingsGroup>
      <SettingsGroup title={t("settings.general.app")}>
        <AppLanguageSelector descriptionMode="tooltip" grouped={true} />
        <ThemeSelector descriptionMode="tooltip" grouped={true} />
      </SettingsGroup>
      <ModelSettingsCard />
      <SettingsGroup title={t("settings.llm.title")}>
        <LlmSettings />
      </SettingsGroup>
      <SettingsGroup title={t("settings.sound.title")}>
        <MicrophoneSelector descriptionMode="tooltip" grouped={true} />
        <ChannelSelector descriptionMode="tooltip" grouped={true} />
        <MuteWhileRecording descriptionMode="tooltip" grouped={true} />
        <PauseMediaWhileRecording descriptionMode="tooltip" grouped={true} />
        <PauseMediaOnMeeting descriptionMode="tooltip" grouped={true} />
        <AudioFeedback descriptionMode="tooltip" grouped={true} />
        <OutputDeviceSelector
          descriptionMode="tooltip"
          grouped={true}
          disabled={!audioFeedbackEnabled}
        />
        <VolumeSlider disabled={!audioFeedbackEnabled} />
      </SettingsGroup>
      <SettingsGroup title={t("settings.meetings.title")}>
        <CallPromptSettings />
        <MeetingAudioSettings />
        <MeetingKeySetting />
      </SettingsGroup>
      <NotesContextSettings />
      <MeetingTemplatesSettings />
    </div>
  );
};
