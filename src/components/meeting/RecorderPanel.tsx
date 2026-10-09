import { useTranslation } from "react-i18next";
import type { MeetingMode, MeetingStatus } from "@/bindings";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { clock } from "./meetingModel";

export interface RecorderPanelProps {
  status: MeetingStatus;
  mode: MeetingMode;
  title: string;
  onModeChange: (mode: MeetingMode) => void;
  onTitleChange: (title: string) => void;
  onStart: () => void;
  onNewDraft: () => void;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  onExtend: () => void;
}

function Level({ label, value }: { label: string; value: number }) {
  const width = Math.round(Math.min(1, Math.sqrt(value) * 2) * 100);
  return (
    <div className="flex items-center gap-2 text-xs">
      <span className="w-16">{label}</span>
      <div className="h-2 flex-1 rounded bg-mid-gray/20">
        <div
          className="h-2 rounded bg-logo-primary"
          style={{ width: `${width}%` }}
        />
      </div>
    </div>
  );
}

/** Record, pause, resume and stop, with the time, the levels and the warnings. */
export function RecorderPanel(props: RecorderPanelProps) {
  const { t } = useTranslation();
  const { status } = props;
  const live = status.state === "recording" || status.state === "paused";

  if (!live) {
    return (
      <div className="flex flex-wrap items-center gap-2 p-4">
        <select
          aria-label={t("meeting.recorder.mode")}
          className="rounded-md border border-mid-gray/80 bg-mid-gray/10 px-2 py-1 text-sm"
          value={props.mode}
          onChange={(event) =>
            props.onModeChange(event.target.value as MeetingMode)
          }
        >
          <option value="meeting">{t("meeting.recorder.online")}</option>
          <option value="in_person">{t("meeting.recorder.inPerson")}</option>
        </select>
        <Input
          variant="compact"
          value={props.title}
          placeholder={t("meeting.recorder.titlePlaceholder")}
          onChange={(event) => props.onTitleChange(event.target.value)}
        />
        <Button onClick={props.onStart} disabled={status.state === "stopping"}>
          {t("meeting.recorder.start")}
        </Button>
        <Button variant="secondary" onClick={props.onNewDraft}>
          {t("meeting.recorder.newDraft")}
        </Button>
        {status.state === "processing" && (
          <span className="text-xs">{t("meeting.recorder.processing")}</span>
        )}
      </div>
    );
  }

  const paused = status.state === "paused";
  return (
    <div className="space-y-2 p-4">
      <div className="flex items-center gap-2">
        <span className="font-mono text-lg">{clock(status.recorded_ms)}</span>
        <span className="text-xs">
          {paused ? t("meeting.recorder.paused") : t("meeting.recorder.live")}
        </span>
        {paused ? (
          <Button variant="secondary" onClick={props.onResume}>
            {t("meeting.recorder.resume")}
          </Button>
        ) : (
          <Button variant="secondary" onClick={props.onPause}>
            {t("meeting.recorder.pause")}
          </Button>
        )}
        <Button variant="danger" onClick={props.onStop}>
          {t("meeting.recorder.stop")}
        </Button>
      </div>
      <Level label={t("meeting.recorder.mic")} value={status.mic_level} />
      <Level label={t("meeting.recorder.system")} value={status.system_level} />
      {status.muted_mic && (
        <p role="alert" className="text-xs text-warning">
          {t("meeting.recorder.mutedMic")}
        </p>
      )}
      {status.muted_system && (
        <p role="alert" className="text-xs text-warning">
          {t("meeting.recorder.mutedSystem")}
        </p>
      )}
      {status.cap_remaining_ms !== null && (
        <p role="alert" className="flex items-center gap-2 text-xs">
          {t("meeting.recorder.capWarning", {
            time: clock(status.cap_remaining_ms),
          })}
          <Button size="sm" variant="secondary" onClick={props.onExtend}>
            {t("meeting.recorder.extend")}
          </Button>
        </p>
      )}
      {status.low_disk_bytes !== null && (
        <p role="alert" className="text-xs text-warning">
          {t("meeting.recorder.lowDisk", {
            mb: Math.floor(status.low_disk_bytes / (1024 * 1024)),
          })}
        </p>
      )}
    </div>
  );
}
