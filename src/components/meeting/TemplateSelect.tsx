import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { MeetingTemplate } from "@/bindings";
import { Input } from "../ui/Input";
import { fold } from "./transcriptSearch";

/** Above this many templates the picker gets a search field. */
export const TEMPLATE_SEARCH_ABOVE = 5;

/**
 * The templates whose name or purpose has the query, ignoring case and accents; all when
 * blank. The picked one always stays, so the select never shows another as chosen.
 */
export function filterTemplates(
  templates: MeetingTemplate[],
  query: string,
  picked: string,
): MeetingTemplate[] {
  const needle = fold(query.trim());
  if (!needle) return templates;
  return templates.filter(
    (template) =>
      template.id === picked ||
      fold(`${template.name} ${template.purpose}`).includes(needle),
  );
}

/**
 * The id the pickers show and use: `id` while it is in the list, else the first template (a
 * user file deleted or broken after the pick). Before the list loads, `id` as it is.
 */
export function shownTemplate(
  templates: MeetingTemplate[],
  id: string | null,
): string {
  if (templates.length === 0 || templates.some((t) => t.id === id)) {
    return id ?? "";
  }
  return templates[0].id;
}

export interface TemplateSelectProps {
  templates: MeetingTemplate[];
  value: string;
  onChange: (id: string) => void;
}

/** The notes template picker: built-in and the person's own in two groups. */
export function TemplateSelect(props: TemplateSelectProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const shown = filterTemplates(props.templates, query, props.value);
  const option = (template: MeetingTemplate) => (
    <option key={template.id} value={template.id}>
      {template.name}
    </option>
  );
  const builtin = shown.filter((template) => template.source === "builtin");
  const user = shown.filter((template) => template.source === "user");
  return (
    <span className="inline-flex items-center gap-2">
      {props.templates.length > TEMPLATE_SEARCH_ABOVE && (
        <Input
          variant="compact"
          type="search"
          className="w-32"
          aria-label={t("meeting.templates.search")}
          placeholder={t("meeting.templates.search")}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      )}
      <select
        aria-label={t("meeting.view.template")}
        title={t("meeting.view.template")}
        className="rounded-md border border-mid-gray/80 bg-mid-gray/10 px-2 py-1 text-sm"
        value={props.value}
        onChange={(event) => props.onChange(event.target.value)}
      >
        {user.length === 0 ? (
          builtin.map(option)
        ) : (
          <>
            {builtin.length > 0 && (
              <optgroup label={t("meeting.templates.builtin")}>
                {builtin.map(option)}
              </optgroup>
            )}
            <optgroup label={t("meeting.templates.user")}>
              {user.map(option)}
            </optgroup>
          </>
        )}
      </select>
    </span>
  );
}
