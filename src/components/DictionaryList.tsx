import React, { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../hooks/useSettings";
import {
  entryButton,
  neighbourAfterRemoval,
  useFocusWhenIdle,
} from "../hooks/useFocusWhenIdle";
import { Button } from "./ui/Button";
import { Input } from "./ui/Input";
import { Textarea } from "./ui/Textarea";
import {
  countByKind,
  draftOf,
  ENTRY_KINDS,
  entriesOf,
  filterEntries,
  LIMITS,
  SETTING,
  type Entry,
  type EntryKind,
  type KindFilter,
} from "./dictionaryModel";

/** The i18n keys of each kind: their texts stay where the three lists had them. */
const KEYS = {
  term: "settings.advanced.customWords",
  replacement: "dictionary.replacements",
  snippet: "dictionary.snippets",
} as const;

const FILTERS: readonly KindFilter[] = ["all", ...ENTRY_KINDS];

const CloseIcon: React.FC = () => (
  <svg
    className="w-3 h-3"
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
    aria-hidden="true"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M6 18L18 6M6 6l12 12"
    />
  </svg>
);

interface RadioButtonsProps<T extends string> {
  options: readonly T[];
  value: T;
  onChange: (value: T) => void;
  label: string;
  /** The text of each option. */
  text: (option: T) => string;
  selectedVariant: "primary" | "primary-soft";
  idleVariant: "secondary" | "ghost";
}

/** A radio group of buttons: one tab stop, the arrows move the choice (like CleanupLevelCards). */
function RadioButtons<T extends string>(props: RadioButtonsProps<T>) {
  const group = useRef<HTMLDivElement>(null);
  const move = (event: React.KeyboardEvent, index: number) => {
    const step =
      event.key === "ArrowRight" || event.key === "ArrowDown"
        ? 1
        : event.key === "ArrowLeft" || event.key === "ArrowUp"
          ? -1
          : 0;
    if (step === 0) return;
    event.preventDefault();
    const next = (index + step + props.options.length) % props.options.length;
    props.onChange(props.options[next]);
    group.current
      ?.querySelectorAll<HTMLButtonElement>('[role="radio"]')
      [next]?.focus();
  };
  return (
    <div
      ref={group}
      role="radiogroup"
      aria-label={props.label}
      className="flex flex-wrap gap-1"
    >
      {props.options.map((option, index) => {
        const checked = option === props.value;
        return (
          <Button
            key={option}
            role="radio"
            aria-checked={checked}
            tabIndex={checked ? 0 : -1}
            size="sm"
            variant={checked ? props.selectedVariant : props.idleVariant}
            onClick={() => props.onChange(option)}
            onKeyDown={(event) => move(event, index)}
          >
            {props.text(option)}
          </Button>
        );
      })}
    </div>
  );
}

interface DictionaryListProps {
  /** The search box above: only the entries that contain it. */
  filter: string;
}

/**
 * The personal dictionary as one list (Wispr's): terms, spoken → written pairs and snippets
 * together, a filter by kind, and one "Add" that opens the form for the kind picked.
 */
export const DictionaryList: React.FC<DictionaryListProps> = ({ filter }) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const terms = getSetting("custom_words") ?? [];
  const replacements = getSetting("dictionary_replacements") ?? [];
  const snippets = getSetting("dictionary_snippets") ?? [];
  const entries = entriesOf(terms, replacements, snippets);
  const [kindFilter, setKindFilter] = useState<KindFilter>("all");
  const visible = filterEntries(entries, kindFilter, filter);
  const counts = countByKind(entries);

  const [adding, setAdding] = useState(false);
  const [kind, setKind] = useState<EntryKind>("term");
  const [spoken, setSpoken] = useState("");
  const [written, setWritten] = useState("");
  // One write at a time across the three lists: adds and removes wait for it.
  const busy = ENTRY_KINDS.some((k) => isUpdating(SETTING[k]));
  const draft = draftOf(kind, spoken, written, entries);
  const spokenRef = useRef<HTMLInputElement>(null);
  const addRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const focusWhenIdle = useFocusWhenIdle(busy);

  const open = (next: EntryKind) => {
    setKind(next);
    setAdding(true);
    requestAnimationFrame(() => spokenRef.current?.focus());
  };
  const close = () => {
    setAdding(false);
    setSpoken("");
    setWritten("");
    requestAnimationFrame(() => addRef.current?.focus());
  };
  // Another kind starts its second field empty and keeps what fits of the first.
  const pickKind = (next: EntryKind) => {
    setKind(next);
    setWritten("");
    setSpoken((text) => text.slice(0, LIMITS[next].maxSpoken));
  };

  // The fields stay enabled while the setting is written, so focus and the next keys stay in
  // the form; adding waits for the write. After an add, the form stays open for the next one.
  const save = () => {
    if (busy || !draft.ok) return;
    if (kind === "term") {
      updateSetting("custom_words", [...terms, draft.spoken]);
    } else {
      const list = kind === "replacement" ? replacements : snippets;
      updateSetting(SETTING[kind], [
        ...list,
        { spoken: draft.spoken, written: draft.written, uses: 0 },
      ]);
    }
    setSpoken("");
    setWritten("");
    spokenRef.current?.focus();
  };

  // Enter in a field adds (Ctrl+Enter in the snippet text, where Enter is a new line); on a
  // button, Enter is that button's. Escape closes the form.
  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }
    if (event.key !== "Enter") return;
    const target = event.target;
    const field =
      target instanceof HTMLInputElement ||
      (target instanceof HTMLTextAreaElement &&
        (event.ctrlKey || event.metaKey));
    if (field) {
      event.preventDefault();
      save();
    }
  };

  // From the keyboard, focus moves to the entry that takes its place, else to "Add" (or the
  // form's first field while the form is open); a click leaves focus alone.
  const remove = (entry: Entry, fromKeyboard: boolean) => {
    if (busy) return;
    if (fromKeyboard) {
      const next = neighbourAfterRemoval(
        visible.map((e) => e.key),
        entry.key,
      );
      const fallback = () => addRef.current ?? spokenRef.current;
      focusWhenIdle(() =>
        next === null
          ? fallback()
          : (entryButton(listRef.current, next) ?? fallback()),
      );
    }
    if (entry.kind === "term") {
      updateSetting(
        "custom_words",
        terms.filter((word) => word !== entry.text),
      );
    } else {
      const list = entry.kind === "replacement" ? replacements : snippets;
      updateSetting(
        SETTING[entry.kind],
        list.filter((pair) => pair.spoken !== entry.spoken),
      );
    }
  };

  const removeLabel = (entry: Entry) =>
    entry.kind === "term"
      ? t(`${KEYS.term}.remove`, { word: entry.text })
      : t(`${KEYS[entry.kind]}.remove`, {
          spoken: entry.spoken,
          written: entry.written,
        });

  // Why "Add" is off, when the person can fix it.
  const problem =
    !draft.ok && draft.reason === "duplicate"
      ? t(`${KEYS[kind]}.duplicate`, {
          word: draft.spoken,
          spoken: draft.spoken,
        })
      : !draft.ok && draft.reason === "twoWords" && spoken.trim() !== ""
        ? t(`${KEYS.snippet}.twoWords`)
        : null;

  const form = adding && (
    <div
      className="space-y-3 rounded-lg border border-border bg-surface-1 p-4"
      onKeyDown={onKeyDown}
    >
      <RadioButtons
        options={ENTRY_KINDS}
        value={kind}
        onChange={pickKind}
        label={t("dictionary.list.kind")}
        text={(k) => t(`dictionary.list.kinds.${k}`)}
        selectedVariant="primary"
        idleVariant="secondary"
      />
      <p className="text-caption text-text-2">
        {t(`${KEYS[kind]}.description`)}
      </p>
      <div className="flex flex-wrap items-center gap-2">
        <Input
          ref={spokenRef}
          type="text"
          className={kind === "term" ? "w-56" : "w-40"}
          value={spoken}
          onChange={(event) => setSpoken(event.target.value)}
          placeholder={t(
            kind === "term"
              ? `${KEYS.term}.placeholder`
              : `${KEYS[kind]}.spokenPlaceholder`,
          )}
          aria-label={t(
            kind === "term"
              ? `${KEYS.term}.placeholder`
              : `${KEYS[kind]}.spokenLabel`,
          )}
          maxLength={LIMITS[kind].maxSpoken}
          variant="compact"
        />
        {kind === "replacement" && (
          <Input
            type="text"
            className="w-48"
            value={written}
            onChange={(event) => setWritten(event.target.value)}
            placeholder={t(`${KEYS.replacement}.writtenPlaceholder`)}
            aria-label={t(`${KEYS.replacement}.writtenLabel`)}
            maxLength={LIMITS.replacement.maxWritten}
            variant="compact"
          />
        )}
        {kind === "snippet" && (
          <Textarea
            className="w-full"
            value={written}
            onChange={(event) => setWritten(event.target.value)}
            placeholder={t(`${KEYS.snippet}.writtenPlaceholder`)}
            aria-label={t(`${KEYS.snippet}.writtenLabel`)}
            maxLength={LIMITS.snippet.maxWritten}
            variant="compact"
          />
        )}
      </div>
      {problem && (
        <p role="status" className="text-caption text-text-2">
          {problem}
        </p>
      )}
      <div className="flex gap-2">
        <Button onClick={save} disabled={!draft.ok || busy}>
          {t(`${KEYS[kind]}.add`)}
        </Button>
        <Button variant="secondary" onClick={close}>
          {t("dictionary.list.close")}
        </Button>
      </div>
    </div>
  );

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <RadioButtons
          options={FILTERS}
          value={kindFilter}
          onChange={setKindFilter}
          label={t("dictionary.list.filter")}
          text={(f) => t(`dictionary.list.filters.${f}`, { count: counts[f] })}
          selectedVariant="primary-soft"
          idleVariant="ghost"
        />
        {!adding && (
          <Button
            ref={addRef}
            onClick={() => open(kindFilter === "all" ? "term" : kindFilter)}
          >
            {t("dictionary.list.add")}
          </Button>
        )}
      </div>
      {form}
      {visible.length === 0 ? (
        <p className="rounded-lg border border-dashed border-border px-4 py-6 text-center text-caption text-text-2">
          {entries.length === 0
            ? t("dictionary.list.empty")
            : t("dictionary.list.noMatch")}
        </p>
      ) : (
        <ul
          ref={listRef}
          className="divide-y divide-border rounded-lg border border-border bg-surface-1"
        >
          {visible.map((entry) => (
            <li
              key={entry.key}
              className="flex min-h-[40px] items-center gap-2 px-4 py-1.5 text-body"
            >
              <span className="w-24 shrink-0 text-caption text-text-3">
                {t(`dictionary.list.kinds.${entry.kind}`)}
              </span>
              {entry.kind === "term" ? (
                <span className="min-w-0 flex-1 truncate text-text">
                  {entry.text}
                </span>
              ) : (
                <span className="flex min-w-0 flex-1 items-center gap-2">
                  <span className="truncate text-text-2">{entry.spoken}</span>
                  <span aria-hidden="true" className="text-text-3">
                    →
                  </span>
                  <span className="truncate text-text">{entry.written}</span>
                  <span className="ms-auto shrink-0 text-caption text-text-3">
                    {t(`${KEYS[entry.kind]}.uses`, { count: entry.uses })}
                  </span>
                </span>
              )}
              <Button
                data-entry={entry.key}
                variant="ghost"
                size="sm"
                disabled={busy}
                aria-label={removeLabel(entry)}
                title={removeLabel(entry)}
                onClick={(event) => remove(entry, event.detail === 0)}
              >
                <CloseIcon />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
