import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { NotesLanguage } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";
import { SettingsGroup } from "../ui/SettingsGroup";
import { Textarea } from "../ui/Textarea";
import { ToggleSwitch } from "../ui/ToggleSwitch";

/** The profile cap of `fala-notes` (`PROFILE_MAX_CHARS`). */
export const PROFILE_MAX_CHARS = 300;

/**
 * What the notes know about the user and what the transcript sends with the audio: the profile
 * and the notes language go to the notes LLM (ADR-0016), the dictionary to Scribe as
 * `keyterms` (ADR-0005).
 */
export function NotesContextSettings() {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const saved = getSetting("notes_profile") ?? "";
  const [profile, setProfile] = useState(saved);
  useEffect(() => setProfile(saved), [saved]);

  const languages: { value: NotesLanguage; label: string }[] = [
    { value: "pt-BR", label: t("meeting.context.language.ptBR") },
    { value: "en", label: t("meeting.context.language.en") },
    { value: "app", label: t("meeting.context.language.app") },
  ];

  return (
    <SettingsGroup
      title={t("meeting.context.title")}
      description={t("meeting.context.description")}
    >
      <SettingContainer
        title={t("meeting.context.profile.title")}
        description={t("meeting.context.profile.description")}
        layout="stacked"
        grouped
      >
        <Textarea
          aria-label={t("meeting.context.profile.title")}
          className="w-full"
          value={profile}
          maxLength={PROFILE_MAX_CHARS}
          placeholder={t("meeting.context.profile.placeholder")}
          onChange={(event) => setProfile(event.target.value)}
          onBlur={() => {
            if (profile !== saved) void updateSetting("notes_profile", profile);
          }}
        />
      </SettingContainer>
      <SettingContainer
        title={t("meeting.context.language.title")}
        description={t("meeting.context.language.description")}
        grouped
      >
        <Dropdown
          options={languages}
          selectedValue={getSetting("notes_language") ?? "pt-BR"}
          onSelect={(value) =>
            void updateSetting("notes_language", value as NotesLanguage)
          }
        />
      </SettingContainer>
      <ToggleSwitch
        checked={getSetting("meeting_keyterms") ?? true}
        onChange={(enabled) => void updateSetting("meeting_keyterms", enabled)}
        isUpdating={isUpdating("meeting_keyterms")}
        label={t("meeting.context.keyterms.title")}
        description={t("meeting.context.keyterms.description")}
        descriptionMode="inline"
        grouped
      />
    </SettingsGroup>
  );
}
