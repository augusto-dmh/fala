import { useCallback, useEffect, useRef, useState } from "react";
import { Check, ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useDismiss } from "../../hooks/useDismiss";
import { stepOption } from "../ui/Dropdown";
import type { MeetingTemplate } from "@/bindings";
import { SearchField } from "../ui/SearchField";
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

/** The shown templates split as the pickers group them: built-in, then the person's own. */
export function templateGroups(
  templates: MeetingTemplate[],
  query: string,
  picked: string,
) {
  const shown = filterTemplates(templates, query, picked);
  return {
    builtin: shown.filter((template) => template.source === "builtin"),
    user: shown.filter((template) => template.source === "user"),
  };
}

/** The session's series, for "Usar sempre em “…”" under the templates. */
export interface SeriesPick {
  name: string;
  /** The picked template is the one the series always uses. */
  always: boolean;
  /** The session has no pick of its own: the series' template is in use. */
  inherited: boolean;
  onAlways: () => void;
}

export interface TemplateSelectProps {
  templates: MeetingTemplate[];
  value: string;
  onChange: (id: string) => void;
  series?: SeriesPick;
}

export interface TemplateMenuProps extends TemplateSelectProps {
  /** Called after a pick, to close the menu that holds the list. */
  onPicked?: () => void;
}

/** The templates as menu items: one checked, built-in and the person's own in named groups, a
 *  search field above five. Opened, the focus goes to the checked item; the arrows, Home and
 *  End move it. */
export function TemplateMenu(props: TemplateMenuProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const { builtin, user } = templateGroups(props.templates, query, props.value);
  const list = useRef<HTMLDivElement>(null);
  const items = () =>
    Array.from(
      list.current?.querySelectorAll<HTMLButtonElement>(
        '[role="menuitemradio"], [role="menuitemcheckbox"]',
      ) ?? [],
    );
  useEffect(() => {
    const all = items();
    (
      all.find((item) => item.getAttribute("aria-checked") === "true") ?? all[0]
    )?.focus();
  }, []);
  const onKeyDown = (event: React.KeyboardEvent) => {
    const all = items();
    const at = all.indexOf(document.activeElement as HTMLButtonElement);
    const next = stepOption(
      all.map(() => ({})),
      at,
      event.key,
    );
    if (next === null) return;
    event.preventDefault();
    all[next]?.focus();
  };
  const item = (template: MeetingTemplate) => (
    <button
      key={template.id}
      type="button"
      role="menuitemradio"
      aria-checked={template.id === props.value}
      className="flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-start text-body text-text transition-colors hover:bg-surface-2 focus-visible:focus-ring"
      onClick={() => {
        props.onChange(template.id);
        props.onPicked?.();
      }}
    >
      <Check
        size={16}
        aria-hidden="true"
        className={template.id === props.value ? "" : "invisible"}
      />
      <span className="truncate">{template.name}</span>
    </button>
  );
  const group = (label: string, templates: MeetingTemplate[]) =>
    templates.length > 0 && (
      <div role="group" aria-label={label}>
        <p
          aria-hidden="true"
          className="px-3 pt-2 pb-1 text-caption text-text-2"
        >
          {label}
        </p>
        {templates.map(item)}
      </div>
    );
  const empty = builtin.length === 0 && user.length === 0;
  return (
    <div className="w-64">
      {props.templates.length > TEMPLATE_SEARCH_ABOVE && (
        <SearchField
          variant="compact"
          className="mb-1 w-full"
          aria-label={t("meeting.templates.search")}
          placeholder={t("meeting.templates.search")}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      )}
      <div
        ref={list}
        role="menu"
        aria-label={t("meeting.view.template")}
        onKeyDown={onKeyDown}
        className="max-h-72 overflow-y-auto"
      >
        {empty ? (
          <p className="px-3 py-1.5 text-body text-text-2">
            {t("common.noOptionsFound")}
          </p>
        ) : user.length === 0 ? (
          group(t("meeting.view.template"), builtin)
        ) : (
          <>
            {group(t("meeting.templates.builtin"), builtin)}
            {group(t("meeting.templates.user"), user)}
          </>
        )}
        {props.series && (
          <div className="mt-1 border-t border-border pt-1">
            <button
              type="button"
              role="menuitemcheckbox"
              aria-checked={props.series.always}
              className="flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-start text-body text-text transition-colors hover:bg-surface-2 focus-visible:focus-ring"
              onClick={() => {
                props.series?.onAlways();
                props.onPicked?.();
              }}
            >
              <Check
                size={16}
                aria-hidden="true"
                className={props.series.always ? "" : "invisible"}
              />
              <span className="truncate">
                {t("meeting.series.always", { name: props.series.name })}
              </span>
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

/** A popover menu behind a button; closing it from inside (a pick, Esc) returns the focus to
 *  the button, so the keyboard never falls to the top of the page. */
export function useMenuPopover() {
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  const close = useCallback(() => {
    const inside = box.current?.contains(document.activeElement) ?? false;
    setOpen(false);
    if (inside) {
      box.current
        ?.querySelector<HTMLButtonElement>('[aria-haspopup="menu"]')
        ?.focus();
    }
  }, []);
  useDismiss(open, close, box);
  return { open, setOpen, close, box };
}

export interface TemplateSelectButtonProps extends TemplateSelectProps {
  /** Where the menu opens: above in the bar at the bottom of a meeting. */
  placement?: "up" | "down";
}

/** The notes template picker: the chosen template's name on a button that opens the menu. */
export function TemplateSelect(props: TemplateSelectButtonProps) {
  const { t } = useTranslation();
  const { open, setOpen, close, box } = useMenuPopover();
  const current = props.templates.find(
    (template) => template.id === props.value,
  );
  const name =
    current && props.series?.inherited
      ? `${current.name} ${t("meeting.series.default")}`
      : current?.name;
  return (
    <div ref={box} className="relative">
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={`${t("meeting.view.template")}: ${name ?? ""}`}
        title={name}
        onClick={() => setOpen(!open)}
        className="flex max-w-56 items-center gap-1 rounded-md border border-border bg-surface-2 px-2 py-1 text-body text-text transition-colors hover:bg-text/5 focus-visible:focus-ring"
      >
        <span className="truncate">{name ?? t("meeting.view.template")}</span>
        <ChevronDown size={16} strokeWidth={1.5} aria-hidden="true" />
      </button>
      {open && (
        <div
          className={`absolute end-0 z-20 rounded-xl border border-border bg-surface-1 p-1 shadow-lg ${
            props.placement === "up" ? "bottom-full mb-2" : "top-full mt-1"
          }`}
        >
          <TemplateMenu {...props} onPicked={close} />
        </div>
      )}
    </div>
  );
}
