import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { readFile, stat } from "@tauri-apps/plugin-fs";
import { toast } from "sonner";
import { useSettings } from "../hooks/useSettings";
import { useSettingsStore } from "../stores/settingsStore";
import { Alert } from "./ui/Alert";
import { Button } from "./ui/Button";
import { Dialog } from "./ui/Dialog";
import type { Entry } from "./dictionaryModel";
import {
  importedLists,
  MAX_FIELD,
  MAX_LINES,
  previewImport,
  type ImportPreview,
} from "./dictionaryImport";

/** Lines of the preview: the first invalid lines and the first new entries. */
const SHOWN = 5;
/** 5,000 lines of two 200-character fields fit in about 2 MB. */
const MAX_BYTES = 4 << 20;

/** What the file brings, before anything is stored. */
export const ImportPreviewBody: React.FC<{ preview: ImportPreview }> = ({
  preview,
}) => {
  const { t } = useTranslation();
  if (preview.error) {
    return (
      <Alert variant="error">
        {preview.error.kind === "lines"
          ? t("dictionary.import.tooManyLines", {
              count: preview.error.count,
              max: MAX_LINES,
            })
          : t("dictionary.import.fieldTooLong", {
              line: preview.error.line,
              max: MAX_FIELD,
            })}
      </Alert>
    );
  }
  const terms = preview.added.filter((entry) => entry.kind === "term").length;
  const counts = {
    terms,
    replacements: preview.added.length - terms,
    duplicates: preview.duplicates,
    invalid: preview.invalid.length,
  };
  return (
    <div className="space-y-3 text-body">
      <ul className="space-y-0.5">
        {(Object.keys(counts) as (keyof typeof counts)[]).map((key) => (
          <li key={key}>
            {t(`dictionary.import.counts.${key}`, { count: counts[key] })}
          </li>
        ))}
      </ul>
      {preview.invalid.length > 0 && (
        <ul className="space-y-0.5 text-caption text-text-2">
          {preview.invalid.slice(0, SHOWN).map(({ line, reason }) => (
            <li key={line}>
              {t("dictionary.import.invalidLine", {
                line,
                reason: t(`dictionary.import.reasons.${reason}`),
              })}
            </li>
          ))}
        </ul>
      )}
      {preview.added.length > 0 ? (
        <div className="space-y-1">
          <p className="text-caption text-text-3">
            {t("dictionary.import.first")}
          </p>
          <ul className="divide-y divide-border rounded-lg border border-border bg-surface-1">
            {preview.added.slice(0, SHOWN).map((entry, index) => (
              <li key={index} className="flex gap-2 px-3 py-1.5">
                <span className="w-24 shrink-0 text-caption text-text-3">
                  {t(`dictionary.list.kinds.${entry.kind}`)}
                </span>
                <span className="min-w-0 flex-1 truncate">
                  {entry.kind === "term"
                    ? entry.text
                    : `${entry.spoken} → ${entry.written}`}
                </span>
              </li>
            ))}
          </ul>
        </div>
      ) : (
        <p className="text-caption text-text-2">
          {t("dictionary.import.nothing")}
        </p>
      )}
    </div>
  );
};

interface ImportDictionaryProps {
  /** The dictionary as the page shows it, to tell new entries from repeats. */
  entries: readonly Entry[];
  /** Another write to the dictionary is running. */
  disabled: boolean;
}

/**
 * "Import…": picks a `.txt` or `.csv`, previews what it adds and stores only the new terms and
 * pairs, after the person confirms. Nothing already in the dictionary is changed.
 */
export const ImportDictionary: React.FC<ImportDictionaryProps> = ({
  entries,
  disabled,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const [file, setFile] = useState<{
    name: string;
    preview: ImportPreview;
  } | null>(null);
  const [saving, setSaving] = useState(false);

  const choose = async () => {
    try {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: t("dictionary.import.files"),
            extensions: ["txt", "csv", "tsv"],
          },
        ],
      });
      if (typeof path !== "string") return;
      // A file this big is not a word list: refuse it before reading it into the page.
      if ((await stat(path)).size > MAX_BYTES) {
        toast.error(t("dictionary.import.tooBig", { max: MAX_BYTES >> 20 }));
        return;
      }
      // Anything that is not UTF-8 (cp1252, UTF-16) throws instead of turning into garbage.
      const text = new TextDecoder("utf-8", { fatal: true }).decode(
        await readFile(path),
      );
      const name = path.split(/[\\/]/).pop() ?? path;
      setFile({ name, preview: previewImport(text, entries) });
    } catch (error) {
      console.error("Failed to read the dictionary file:", error);
      toast.error(t("dictionary.import.readError"));
    }
  };

  const cancel = () => setFile(null);

  // The toast counts what the store holds after the writes: `updateSetting` rolls a failed
  // write back without throwing.
  const confirm = async () => {
    if (!file) return;
    const stored = () => {
      const settings = useSettingsStore.getState().settings;
      return (
        (settings?.custom_words?.length ?? 0) +
        (settings?.dictionary_replacements?.length ?? 0)
      );
    };
    const before = stored();
    const total = file.preview.added.length;
    setSaving(true);
    try {
      const lists = importedLists(
        file.preview,
        getSetting("custom_words") ?? [],
        getSetting("dictionary_replacements") ?? [],
      );
      if (lists.terms) await updateSetting("custom_words", lists.terms);
      if (lists.replacements)
        await updateSetting("dictionary_replacements", lists.replacements);
    } finally {
      setSaving(false);
      setFile(null);
    }
    const count = stored() - before;
    if (count === total) toast.success(t("dictionary.import.done", { count }));
    else toast.error(t("dictionary.import.saveError", { count, total }));
  };

  const count = file?.preview.added.length ?? 0;
  return (
    <>
      <Button
        variant="secondary"
        disabled={disabled}
        onClick={() => void choose()}
      >
        {t("dictionary.import.open")}
      </Button>
      {file && (
        <Dialog
          open
          onOpenChange={(next) => !next && !saving && cancel()}
          title={t("dictionary.import.title")}
          description={file.name}
          closeLabel={t("common.close")}
          dismissible={!saving}
          footer={
            <>
              <Button variant="secondary" disabled={saving} onClick={cancel}>
                {t("dictionary.import.cancel")}
              </Button>
              <Button
                disabled={count === 0 || saving || disabled}
                onClick={() => void confirm()}
              >
                {t("dictionary.import.confirm", { count })}
              </Button>
            </>
          }
        >
          <p className="mb-3 text-caption text-text-2">
            {t("dictionary.import.format")}
          </p>
          <ImportPreviewBody preview={file.preview} />
        </Dialog>
      )}
    </>
  );
};
