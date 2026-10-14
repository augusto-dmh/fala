import { Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type MeetingTemplates,
  type SeriesTemplate,
} from "@/bindings";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";
import { SettingsGroup } from "../ui/SettingsGroup";
import { errorDetail } from "./meetingModel";
import { useMeetingTemplates } from "./useMeetingTemplates";
import { useSettings } from "../../hooks/useSettings";

export interface TemplatesSettingsProps {
  catalog: MeetingTemplates;
  /** "Por série": the series key to its name and template (`meeting_series_templates`). */
  series?: Partial<Record<string, SeriesTemplate>>;
  /** 🗑 of a series: its meetings go back to choosing. */
  onForgetSeries?: (key: string, name: string) => void;
  /** The list after a duplicate; the page reloads it on focus too. */
  onChange: (catalog: MeetingTemplates) => void;
}

/**
 * The notes templates: the built-in ones and the person's own `*.json` in the templates
 * folder, edited outside the app (no editor here). A broken file is listed, never hidden.
 */
export function TemplatesSettings(props: TemplatesSettingsProps) {
  const { t } = useTranslation();
  const [error, setError] = useState<string | null>(null);

  const openFolder = async () => {
    setError(null);
    const result = await commands.openMeetingTemplatesFolder();
    if (result.status === "error") setError(errorDetail(result.error));
  };
  const duplicate = async (id: string) => {
    setError(null);
    const result = await commands.duplicateMeetingTemplate(id);
    if (result.status === "ok") props.onChange(result.data);
    else setError(errorDetail(result.error));
  };

  return (
    <SettingsGroup
      title={t("meeting.templates.title")}
      description={t("meeting.templates.description")}
    >
      <SettingContainer
        title={t("meeting.templates.folder")}
        description={t("meeting.templates.folderDescription")}
        grouped
      >
        <Button variant="secondary" size="sm" onClick={() => void openFolder()}>
          {t("meeting.templates.open")}
        </Button>
      </SettingContainer>
      {props.catalog.templates.map((template) => (
        <SettingContainer
          key={template.id}
          title={template.name}
          description={
            template.source === "builtin"
              ? t("meeting.templates.builtinOne")
              : t("meeting.templates.userOne")
          }
          grouped
        >
          {template.source === "builtin" && (
            <Button
              variant="secondary"
              size="sm"
              onClick={() => void duplicate(template.id)}
            >
              {t("meeting.templates.duplicate")}
            </Button>
          )}
        </SettingContainer>
      ))}
      {Object.entries(props.series ?? {}).map(
        ([key, series]) =>
          series && (
            <SettingContainer
              key={key}
              title={t("meeting.series.bySeries", { name: series.name })}
              description={
                props.catalog.templates.find((x) => x.id === series.template)
                  ?.name ?? series.template
              }
              grouped
            >
              <Button
                variant="ghost"
                size="sm"
                aria-label={t("meeting.series.forget", { name: series.name })}
                onClick={() => props.onForgetSeries?.(key, series.name)}
              >
                <Trash2 size={14} strokeWidth={1.5} aria-hidden="true" />
              </Button>
            </SettingContainer>
          ),
      )}
      {(props.catalog.errors.length > 0 || error !== null) && (
        <div role="alert" className="space-y-1 p-4 text-sm text-red-500">
          {props.catalog.errors.map((file) => (
            <p key={file.file}>{`${file.file}: ${file.error}`}</p>
          ))}
          {error !== null && (
            <p>{t("meeting.templates.folderError", { detail: error })}</p>
          )}
        </div>
      )}
    </SettingsGroup>
  );
}

/** Settings > Meetings: the templates, with their own copy of the catalog. */
export function MeetingTemplatesSettings() {
  const [catalog, setCatalog] = useMeetingTemplates();
  const { getSetting, refreshSettings } = useSettings();
  return (
    <TemplatesSettings
      catalog={catalog}
      onChange={setCatalog}
      series={getSetting("meeting_series_templates")}
      onForgetSeries={(key, name) =>
        void commands
          .setMeetingSeriesTemplate(key, name, null)
          .then(() => refreshSettings())
      }
    />
  );
}
