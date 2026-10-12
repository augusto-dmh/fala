import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import type { WritingStyle } from "@/bindings";
import { useSettings } from "../../hooks/useSettings";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { Select } from "../ui/Select";
import { SettingContainer } from "../ui/SettingContainer";
import { Textarea } from "../ui/Textarea";
import { newStyleId, setAppStyle } from "./appStylesModel";

/** One style, edited in place and saved when a field loses focus. */
const StyleRow: React.FC<{
  style: WritingStyle;
  disabled: boolean;
  onSave: (style: WritingStyle) => void;
  onRemove: () => void;
}> = ({ style, disabled, onSave, onRemove }) => {
  const { t } = useTranslation();
  const [name, setName] = useState(style.name);
  const [instruction, setInstruction] = useState(style.instruction);
  // A blank name keeps the stored one; the instruction is saved either way.
  const save = () => {
    const next = {
      ...style,
      name: name.trim() ? name : style.name,
      instruction,
    };
    setName(next.name);
    if (next.name !== style.name || next.instruction !== style.instruction) {
      onSave(next);
    }
  };
  return (
    <li className="flex flex-col gap-1" data-style={style.id}>
      <div className="flex items-center gap-2">
        <Input
          type="text"
          variant="compact"
          className="max-w-48"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onBlur={save}
          aria-label={t("settings.llm.styles.nameLabel")}
        />
        <Button
          variant="secondary"
          size="sm"
          onClick={onRemove}
          disabled={disabled}
          aria-label={t("settings.llm.styles.remove", { name: style.name })}
        >
          {t("settings.llm.styles.removeShort")}
        </Button>
      </div>
      <Textarea
        variant="compact"
        className="min-h-[56px]"
        value={instruction}
        onChange={(e) => setInstruction(e.target.value)}
        onBlur={save}
        placeholder={t("settings.llm.styles.instructionPlaceholder")}
        aria-label={t("settings.llm.styles.instructionLabel")}
      />
    </li>
  );
};

/** The user's writing styles and the app -> style mapping. The instruction of the app's style
 *  goes at the end of the prompt; an app without one keeps the prompt as it was. */
export const AppStyles: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const styles = getSetting("writing_styles") ?? [];
  const mappings = getSetting("app_styles") ?? [];
  const [newApp, setNewApp] = useState("");
  const [newStyle, setNewStyle] = useState<string | null>(null);
  const busy = isUpdating("writing_styles") || isUpdating("app_styles");
  const options = styles.map((style) => ({
    value: style.id,
    label: style.name,
  }));
  const nameOf = (id: string) => styles.find((s) => s.id === id)?.name ?? id;

  const handleMap = () => {
    if (!newApp.trim() || !newStyle) return;
    // The backend reduces the entry to the app name.
    updateSetting("app_styles", setAppStyle(mappings, newApp, newStyle));
    setNewApp("");
  };

  return (
    <>
      <SettingContainer
        title={t("settings.llm.appStyles.title")}
        description={t("settings.llm.appStyles.description")}
        descriptionMode="tooltip"
        grouped={true}
      >
        <div className="flex items-center gap-2">
          <Input
            type="text"
            className="max-w-32"
            value={newApp}
            onChange={(e) => setNewApp(e.target.value)}
            placeholder={t("settings.llm.disabledApps.placeholder")}
            variant="compact"
            disabled={busy}
          />
          <Select
            className="min-w-36"
            value={newStyle}
            options={options}
            onChange={setNewStyle}
            placeholder={t("settings.llm.appStyles.stylePlaceholder")}
            disabled={busy}
          />
          <Button
            onClick={handleMap}
            disabled={!newApp.trim() || !newStyle || busy}
            variant="primary"
            size="md"
          >
            {t("settings.llm.disabledApps.add")}
          </Button>
        </div>
      </SettingContainer>
      {mappings.length > 0 && (
        <div className="px-4 p-2 flex flex-wrap gap-1">
          {mappings.map((m) => (
            <Button
              key={m.app}
              onClick={() =>
                updateSetting(
                  "app_styles",
                  mappings.filter((other) => other.app !== m.app),
                )
              }
              disabled={busy}
              variant="secondary"
              size="sm"
              aria-label={t("settings.llm.disabledApps.remove", { app: m.app })}
            >
              {t("settings.llm.appStyles.mapping", {
                app: m.app,
                style: nameOf(m.style),
              })}
            </Button>
          ))}
        </div>
      )}
      <SettingContainer
        title={t("settings.llm.styles.title")}
        description={t("settings.llm.styles.description")}
        descriptionMode="tooltip"
        layout="stacked"
        grouped={true}
      >
        <ul className="flex flex-col gap-3">
          {styles.map((style) => (
            <StyleRow
              key={style.id}
              style={style}
              disabled={busy}
              onSave={(edited) =>
                updateSetting(
                  "writing_styles",
                  styles.map((s) => (s.id === edited.id ? edited : s)),
                )
              }
              onRemove={() =>
                updateSetting(
                  "writing_styles",
                  styles.filter((s) => s.id !== style.id),
                )
              }
            />
          ))}
        </ul>
        <Button
          className="mt-2 self-start"
          variant="secondary"
          size="md"
          disabled={busy}
          onClick={() =>
            updateSetting("writing_styles", [
              ...styles,
              {
                id: newStyleId(styles),
                name: t("settings.llm.styles.newName"),
                instruction: "",
              },
            ])
          }
        >
          {t("settings.llm.styles.add")}
        </Button>
      </SettingContainer>
    </>
  );
};
