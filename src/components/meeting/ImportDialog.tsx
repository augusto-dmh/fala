import { open } from "@tauri-apps/plugin-dialog";
import { X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ImportSource, ProgressStage } from "@/bindings";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { importLine, isHttpUrl } from "./meetingModel";

const MEDIA = [
  "mp4",
  "mkv",
  "webm",
  "mov",
  "avi",
  "mp3",
  "m4a",
  "aac",
  "wav",
  "ogg",
  "opus",
  "flac",
];

export interface ImportDialogProps {
  /** An import is running; `stage` is its last progress event. */
  importing: boolean;
  stage: ProgressStage | null;
  onImport: (source: ImportSource) => void;
  onCancel: () => void;
  /** "Granola history": the export picked (a CSV or the `.zip` from the email). */
  onGranola: (path: string) => void;
}

/** "Import": an audio or video file, or the audio of a URL, becomes a session. */
export function ImportDialog(props: ImportDialogProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const [url, setUrl] = useState("");

  if (props.importing) {
    return (
      <div className="flex basis-full items-center gap-2 text-sm">
        <span role="status">{importLine(props.stage, t)}</span>
        <Button size="sm" variant="secondary" onClick={props.onCancel}>
          {t("meeting.import.cancel")}
        </Button>
      </div>
    );
  }
  if (!expanded) {
    return (
      <Button variant="secondary" onClick={() => setExpanded(true)}>
        {t("meeting.import.open")}
      </Button>
    );
  }

  const chooseFile = async () => {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [
        { name: t("meeting.import.media"), extensions: MEDIA },
        { name: t("meeting.import.anyFile"), extensions: ["*"] },
      ],
    });
    if (typeof path === "string") props.onImport({ kind: "file", value: path });
  };

  const chooseGranola = async () => {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [
        { name: t("meeting.import.granolaFiles"), extensions: ["csv", "zip"] },
      ],
    });
    if (typeof path === "string") {
      setExpanded(false);
      props.onGranola(path);
    }
  };

  // The URL gets its own line and Close is the ✕ in the corner: at 900 px the old single row
  // pushed "Close" alone onto a second line and cut the URL placeholder.
  return (
    <div className="relative basis-full space-y-3 rounded-lg border border-border bg-surface-1 p-4 pe-12 text-sm">
      <Button
        variant="ghost"
        size="sm"
        className="absolute end-2 top-2 px-1.5"
        aria-label={t("meeting.import.close")}
        title={t("meeting.import.close")}
        onClick={() => setExpanded(false)}
      >
        <X size={16} strokeWidth={1.5} aria-hidden="true" />
      </Button>
      <div className="flex flex-wrap items-center gap-2">
        <Button onClick={() => void chooseFile()}>
          {t("meeting.import.file")}
        </Button>
        <Button variant="secondary" onClick={() => void chooseGranola()}>
          {t("meeting.import.granola")}
        </Button>
      </div>
      <div className="flex items-center gap-2">
        <Input
          variant="compact"
          type="url"
          className="min-w-0 flex-1"
          value={url}
          placeholder={t("meeting.import.urlPlaceholder")}
          aria-label={t("meeting.import.urlPlaceholder")}
          onChange={(event) => setUrl(event.target.value)}
        />
        <Button
          className="shrink-0"
          disabled={!isHttpUrl(url)}
          onClick={() => {
            props.onImport({ kind: "url", value: url.trim() });
            setUrl("");
          }}
        >
          {t("meeting.import.url")}
        </Button>
      </div>
      <p className="text-caption text-text-2">{t("meeting.import.hint")}</p>
    </div>
  );
}
