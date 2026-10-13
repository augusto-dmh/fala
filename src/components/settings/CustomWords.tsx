import React, { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useSettings } from "../../hooks/useSettings";
import { matchesQuery } from "../../lib/utils/search";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";
import {
  entryButton,
  neighbourAfterRemoval,
  useFocusWhenIdle,
} from "../../hooks/useFocusWhenIdle";

interface CustomWordsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  /** Shows only the terms that contain it. */
  filter?: string;
}

const normalizeCustomWord = (word: string) =>
  word
    .replace(/[<>"']/g, "")
    .replace(/\s+/g, " ")
    .trim();

export const CustomWords: React.FC<CustomWordsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false, filter = "" }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const [newWord, setNewWord] = useState("");
    const customWords = getSetting("custom_words") || [];
    const visibleWords = customWords.filter((word) =>
      matchesQuery(word, filter),
    );
    const normalizedWord = normalizeCustomWord(newWord);
    const inputRef = useRef<HTMLInputElement>(null);
    const listRef = useRef<HTMLDivElement>(null);
    const busy = isUpdating("custom_words");
    const focusWhenIdle = useFocusWhenIdle(busy);

    // The field stays enabled during a write, so focus and the next keystrokes stay in it; only
    // adding waits for the write.
    const handleAddWord = () => {
      if (busy) return;
      if (normalizedWord && normalizedWord.length <= 50) {
        const lowered = normalizedWord.toLowerCase();
        if (customWords.some((word) => word.toLowerCase() === lowered)) {
          toast.error(
            t("settings.advanced.customWords.duplicate", {
              word: normalizedWord,
            }),
          );
          return;
        }
        updateSetting("custom_words", [...customWords, normalizedWord]);
        setNewWord("");
      }
    };

    // From the keyboard, focus moves to the entry that takes its place (or the field), so the
    // next removal or term needs no mouse; a click leaves focus alone.
    const handleRemoveWord = (wordToRemove: string, fromKeyboard: boolean) => {
      if (fromKeyboard) {
        const next = neighbourAfterRemoval(visibleWords, wordToRemove);
        focusWhenIdle(() =>
          next === null
            ? inputRef.current
            : (entryButton(listRef.current, next) ?? inputRef.current),
        );
      }
      updateSetting(
        "custom_words",
        customWords.filter((word) => word !== wordToRemove),
      );
    };

    const handleKeyPress = (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAddWord();
      }
    };

    return (
      <>
        <SettingContainer
          title={t("settings.advanced.customWords.title")}
          description={t("settings.advanced.customWords.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <div className="flex items-center gap-2">
            <Input
              ref={inputRef}
              type="text"
              className="max-w-40"
              value={newWord}
              onChange={(e) => setNewWord(e.target.value)}
              onKeyDown={handleKeyPress}
              placeholder={t("settings.advanced.customWords.placeholder")}
              variant="compact"
            />
            <Button
              onClick={handleAddWord}
              disabled={!normalizedWord || normalizedWord.length > 50 || busy}
              variant="primary"
              size="md"
            >
              {t("settings.advanced.customWords.add")}
            </Button>
          </div>
        </SettingContainer>
        {visibleWords.length > 0 && (
          <div
            ref={listRef}
            className={`px-4 p-2 ${grouped ? "" : "rounded-lg border border-mid-gray/20"} flex flex-wrap gap-1`}
          >
            {visibleWords.map((word) => (
              <Button
                key={word}
                data-entry={word}
                onClick={(event) => handleRemoveWord(word, event.detail === 0)}
                disabled={busy}
                variant="secondary"
                size="sm"
                className="inline-flex items-center gap-1 cursor-pointer"
                aria-label={t("settings.advanced.customWords.remove", { word })}
              >
                <span>{word}</span>
                <svg
                  className="w-3 h-3"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </Button>
            ))}
          </div>
        )}
      </>
    );
  },
);
