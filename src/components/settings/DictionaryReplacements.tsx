import React, { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useSettings } from "../../hooks/useSettings";
import { matchesQuery } from "../../lib/utils/search";
import { Input } from "../ui/Input";
import { Textarea } from "../ui/Textarea";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";
import {
  entryButton,
  neighbourAfterRemoval,
  useFocusWhenIdle,
} from "../../hooks/useFocusWhenIdle";

interface DictionaryReplacementsProps {
  /** Shows only the pairs whose spoken or written form contains it. */
  filter?: string;
  /** Spoken -> written pairs, or snippets: a trigger of two words or more -> fixed text. */
  kind?: "replacements" | "snippets";
}

const KINDS = {
  replacements: {
    setting: "dictionary_replacements",
    maxWritten: 200,
    minWords: 1,
  },
  snippets: { setting: "dictionary_snippets", maxWritten: 4000, minWords: 2 },
} as const;

const MAX_SPOKEN = 60;

const ArrowIcon: React.FC = () => (
  <svg
    className="w-3 h-3 shrink-0 text-text-3"
    fill="none"
    stroke="currentColor"
    viewBox="0 0 24 24"
    aria-hidden="true"
  >
    <path
      strokeLinecap="round"
      strokeLinejoin="round"
      strokeWidth={2}
      d="M5 12h14m-6-6 6 6-6 6"
    />
  </svg>
);

/** The spoken -> written pairs, or the snippets, of the personal dictionary, with how often each
 * was applied. */
export const DictionaryReplacements: React.FC<DictionaryReplacementsProps> =
  React.memo(({ filter = "", kind = "replacements" }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const [spoken, setSpoken] = useState("");
    const [written, setWritten] = useState("");
    const { setting, maxWritten, minWords } = KINDS[kind];
    const keys = `dictionary.${kind}`;
    const pairs = getSetting(setting) || [];
    const busy = isUpdating(setting);
    const spokenRef = useRef<HTMLInputElement>(null);
    const listRef = useRef<HTMLUListElement>(null);
    const focusWhenIdle = useFocusWhenIdle(busy);
    const cleanSpoken = spoken.replace(/\s+/g, " ").trim();
    const cleanWritten = written.trim();
    const words = cleanSpoken ? cleanSpoken.split(" ").length : 0;
    const canAdd =
      words >= minWords &&
      cleanWritten.length > 0 &&
      cleanSpoken.length <= MAX_SPOKEN &&
      cleanWritten.length <= maxWritten &&
      !busy;

    const handleAdd = () => {
      if (!canAdd) return;
      const key = cleanSpoken.toLowerCase();
      if (pairs.some((pair) => pair.spoken.toLowerCase() === key)) {
        toast.error(t(`${keys}.duplicate`, { spoken: cleanSpoken }));
        return;
      }
      updateSetting(setting, [
        ...pairs,
        { spoken: cleanSpoken, written: cleanWritten, uses: 0 },
      ]);
      setSpoken("");
      setWritten("");
      // The fields stay enabled during the write: the next pair starts at its spoken form.
      spokenRef.current?.focus();
    };

    // From the keyboard, focus moves to the pair that takes its place (or the field); a click
    // leaves focus alone.
    const handleRemove = (target: string, fromKeyboard: boolean) => {
      if (fromKeyboard) {
        const next = neighbourAfterRemoval(
          visible.map((pair) => pair.spoken),
          target,
        );
        focusWhenIdle(() =>
          next === null
            ? spokenRef.current
            : (entryButton(listRef.current, next) ?? spokenRef.current),
        );
      }
      updateSetting(
        setting,
        pairs.filter((pair) => pair.spoken !== target),
      );
    };

    // A snippet's text can have line breaks: there Enter is a new line and Ctrl+Enter adds.
    const handleKeyDown = (e: React.KeyboardEvent) => {
      const multiline = e.target instanceof HTMLTextAreaElement;
      if (e.key === "Enter" && (!multiline || e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        handleAdd();
      }
    };

    const visible = pairs.filter(
      (pair) =>
        matchesQuery(pair.spoken, filter) || matchesQuery(pair.written, filter),
    );

    return (
      <>
        <SettingContainer
          title={t(`${keys}.title`)}
          description={t(`${keys}.description`)}
          grouped
          layout="stacked"
        >
          <div className="flex flex-wrap items-center gap-2">
            <Input
              ref={spokenRef}
              type="text"
              className="w-40"
              value={spoken}
              onChange={(e) => setSpoken(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={t(`${keys}.spokenPlaceholder`)}
              aria-label={t(`${keys}.spokenLabel`)}
              maxLength={MAX_SPOKEN}
              variant="compact"
            />
            <ArrowIcon />
            {kind === "snippets" ? (
              <Textarea
                className="w-full"
                value={written}
                onChange={(e) => setWritten(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={t(`${keys}.writtenPlaceholder`)}
                aria-label={t(`${keys}.writtenLabel`)}
                maxLength={maxWritten}
                variant="compact"
              />
            ) : (
              <Input
                type="text"
                className="w-48"
                value={written}
                onChange={(e) => setWritten(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={t(`${keys}.writtenPlaceholder`)}
                aria-label={t(`${keys}.writtenLabel`)}
                maxLength={maxWritten}
                variant="compact"
              />
            )}
            <Button
              onClick={handleAdd}
              disabled={!canAdd}
              variant="primary"
              size="md"
            >
              {t(`${keys}.add`)}
            </Button>
          </div>
          {minWords > 1 && words === 1 && (
            <p className="mt-2 text-caption text-text-3">
              {t(`${keys}.twoWords`)}
            </p>
          )}
        </SettingContainer>
        {visible.length > 0 && (
          <ul ref={listRef} className="px-4 py-2 flex flex-col gap-1">
            {visible.map((pair) => (
              <li
                key={pair.spoken}
                className="flex items-center gap-2 min-h-[32px] text-body"
              >
                <span className="text-text-2 truncate">{pair.spoken}</span>
                <ArrowIcon />
                <span className="text-text font-medium truncate">
                  {pair.written}
                </span>
                <span className="ms-auto text-caption text-text-3 shrink-0">
                  {t(`${keys}.uses`, { count: pair.uses ?? 0 })}
                </span>
                <Button
                  data-entry={pair.spoken}
                  onClick={(event) =>
                    handleRemove(pair.spoken, event.detail === 0)
                  }
                  disabled={busy}
                  variant="secondary"
                  size="sm"
                  aria-label={t(`${keys}.remove`, {
                    spoken: pair.spoken,
                    written: pair.written,
                  })}
                >
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
                </Button>
              </li>
            ))}
          </ul>
        )}
      </>
    );
  });
