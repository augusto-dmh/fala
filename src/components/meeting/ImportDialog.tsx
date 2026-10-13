import { open } from "@tauri-apps/plugin-dialog";
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

  return (
    <div className="flex basis-full flex-wrap items-center gap-2 rounded-lg border border-border bg-surface-1 p-4 text-sm">
      <Button onClick={() => void chooseFile()}>
        {t("meeting.import.file")}
      </Button>
      <Input
        variant="compact"
        type="url"
        value={url}
        placeholder={t("meeting.import.urlPlaceholder")}
        onChange={(event) => setUrl(event.target.value)}
      />
      <Button
        disabled={!isHttpUrl(url)}
        onClick={() => {
          props.onImport({ kind: "url", value: url.trim() });
          setUrl("");
        }}
      >
        {t("meeting.import.url")}
      </Button>
      <Button variant="secondary" onClick={() => setExpanded(false)}>
        {t("meeting.import.close")}
      </Button>
      <p className="w-full text-xs text-mid-gray">{t("meeting.import.hint")}</p>
    </div>
  );
}
