import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { readFile } from "@tauri-apps/plugin-fs";
import { toast } from "sonner";
import { useSettings } from "../hooks/useSettings";
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
    try {
      // The decoder drops a UTF-8 BOM.
      const text = new TextDecoder("utf-8").decode(await readFile(path));
      const name = path.split(/[\\/]/).pop() ?? path;
      setFile({ name, preview: previewImport(text, entries) });
    } catch (error) {
      console.error("Failed to read the dictionary file:", error);
      toast.error(t("dictionary.import.readError"));
    }
  };

  const cancel = () => setFile(null);

  const confirm = async () => {
    if (!file) return;
    setSaving(true);
    const lists = importedLists(
      file.preview,
      getSetting("custom_words") ?? [],
      getSetting("dictionary_replacements") ?? [],
    );
    if (lists.terms) await updateSetting("custom_words", lists.terms);
    if (lists.replacements)
      await updateSetting("dictionary_replacements", lists.replacements);
    setSaving(false);
    setFile(null);
    toast.success(
      t("dictionary.import.done", { count: file.preview.added.length }),
    );
  };

  const count = file?.preview.error ? 0 : (file?.preview.added.length ?? 0);
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
