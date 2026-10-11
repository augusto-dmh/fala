import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useSettings } from "../../hooks/useSettings";
import { matchesQuery } from "../../lib/utils/search";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

interface DictionaryReplacementsProps {
  /** Shows only the pairs whose spoken or written form contains it. */
  filter?: string;
}

const MAX_SPOKEN = 60;
const MAX_WRITTEN = 200;

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

/** The spoken -> written pairs of the personal dictionary, with how often each was applied. */
export const DictionaryReplacements: React.FC<DictionaryReplacementsProps> =
  React.memo(({ filter = "" }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const [spoken, setSpoken] = useState("");
    const [written, setWritten] = useState("");
    const pairs = getSetting("dictionary_replacements") || [];
    const busy = isUpdating("dictionary_replacements");
    const cleanSpoken = spoken.replace(/\s+/g, " ").trim();
    const cleanWritten = written.trim();
    const canAdd =
      cleanSpoken.length > 0 &&
      cleanWritten.length > 0 &&
      cleanSpoken.length <= MAX_SPOKEN &&
      cleanWritten.length <= MAX_WRITTEN &&
      !busy;

    const handleAdd = () => {
      if (!canAdd) return;
      const key = cleanSpoken.toLowerCase();
      if (pairs.some((pair) => pair.spoken.toLowerCase() === key)) {
        toast.error(
          t("dictionary.replacements.duplicate", { spoken: cleanSpoken }),
        );
        return;
      }
      updateSetting("dictionary_replacements", [
        ...pairs,
        { spoken: cleanSpoken, written: cleanWritten, uses: 0 },
      ]);
      setSpoken("");
      setWritten("");
    };

    const handleRemove = (target: string) => {
      updateSetting(
        "dictionary_replacements",
        pairs.filter((pair) => pair.spoken !== target),
      );
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
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
          title={t("dictionary.replacements.title")}
          description={t("dictionary.replacements.description")}
          grouped
          layout="stacked"
        >
          <div className="flex flex-wrap items-center gap-2">
            <Input
              type="text"
              className="w-40"
              value={spoken}
              onChange={(e) => setSpoken(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={t("dictionary.replacements.spokenPlaceholder")}
              aria-label={t("dictionary.replacements.spokenLabel")}
              maxLength={MAX_SPOKEN}
              variant="compact"
              disabled={busy}
            />
            <ArrowIcon />
            <Input
              type="text"
              className="w-48"
              value={written}
              onChange={(e) => setWritten(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={t("dictionary.replacements.writtenPlaceholder")}
              aria-label={t("dictionary.replacements.writtenLabel")}
              maxLength={MAX_WRITTEN}
              variant="compact"
              disabled={busy}
            />
            <Button
              onClick={handleAdd}
              disabled={!canAdd}
              variant="primary"
              size="md"
            >
              {t("dictionary.replacements.add")}
            </Button>
          </div>
        </SettingContainer>
        {visible.length > 0 && (
          <ul className="px-4 py-2 flex flex-col gap-1">
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
                  {t("dictionary.replacements.uses", { count: pair.uses ?? 0 })}
                </span>
                <Button
                  onClick={() => handleRemove(pair.spoken)}
                  disabled={busy}
                  variant="secondary"
                  size="sm"
                  aria-label={t("dictionary.replacements.remove", {
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
