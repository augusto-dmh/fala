import { useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  MeetingMode,
  MeetingOutputs,
  MeetingStatus,
  MeetingTemplate,
} from "@/bindings";
import { showsConsentChip } from "../../overlay/pillModel";
import { copyToClipboard } from "../settings/history/clipboard";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { clock } from "./meetingModel";

export interface RecorderPanelProps {
  status: MeetingStatus;
  mode: MeetingMode;
  title: string;
  /** The outputs for "Meeting sound"; `null` until listed. */
  outputs: MeetingOutputs | null;
  onModeChange: (mode: MeetingMode) => void;
  onTitleChange: (title: string) => void;
  /** `null`: follow the default output. */
  onOutputChange: (name: string | null) => void;
  /** "Record from this device", while recording. */
  onSwitchOutput: (name: string) => void;
  onStart: () => void;
  onNewDraft: () => void;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  onExtend: () => void;
  /** The template of the session recording now; absent until it loads. */
  templates?: MeetingTemplate[];
  template?: string | null;
  onTemplateChange?: (templateId: string) => void;
}

/** "Copy notice": a ready text for the call's chat, saying the meeting is being recorded. */
export function CopyNoticeButton() {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    // "Copied" only when it was, and only for a moment.
    if (!(await copyToClipboard(t("meeting.notice.text")))) return;
    setCopied(true);
    setTimeout(() => setCopied(false), 3000);
  };
  return (
    <Button variant="secondary" size="sm" onClick={() => void copy()}>
      {copied ? t("meeting.notice.copied") : t("meeting.notice.copy")}
    </Button>
  );
}

/** "Meeting sound": the default output (named) or one of the outputs, remembered. */
function OutputSelect({
  outputs,
  onChange,
}: {
  outputs: MeetingOutputs;
  onChange: (name: string | null) => void;
}) {
  const { t } = useTranslation();
  const fallback = outputs.devices.find((d) => d.is_default)?.name ?? "";
  return (
    <select
      aria-label={t("meeting.recorder.output")}
      title={t("meeting.recorder.output")}
      className="max-w-56 rounded-md border border-mid-gray/80 bg-mid-gray/10 px-2 py-1 text-sm"
      value={outputs.selected ?? ""}
      onChange={(event) => onChange(event.target.value || null)}
    >
      <option value="">
        {t("meeting.recorder.outputDefault", { name: fallback })}
      </option>
      {outputs.devices.map((d) => (
        <option key={d.name} value={d.name}>
          {d.name}
        </option>
      ))}
      {outputs.selected !== null &&
        !outputs.devices.some((d) => d.name === outputs.selected) && (
          <option value={outputs.selected}>{outputs.selected}</option>
        )}
    </select>
  );
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
          <option value="system_only">
            {t("meeting.recorder.systemOnly")}
          </option>
        </select>
        {props.outputs && (
          <OutputSelect
            outputs={props.outputs}
            onChange={props.onOutputChange}
          />
        )}
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
        <CopyNoticeButton />
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
        <CopyNoticeButton />
        {props.templates && props.onTemplateChange && (
          <select
            aria-label={t("meeting.view.template")}
            title={t("meeting.view.template")}
            className="rounded-md border border-mid-gray/80 bg-mid-gray/10 px-2 py-1 text-sm"
            value={props.template ?? props.templates[0]?.id ?? ""}
            onChange={(event) => props.onTemplateChange?.(event.target.value)}
          >
            {props.templates.map((template) => (
              <option key={template.id} value={template.id}>
                {template.name}
              </option>
            ))}
          </select>
        )}
      </div>
      {showsConsentChip(status.recorded_ms) && (
        <p
          role="status"
          className="inline-block rounded-full bg-accent/15 px-3 py-1 text-xs"
        >
          {t("meeting.recorder.consentChip")}
        </p>
      )}
      {status.mode !== "system_only" && (
        <Level label={t("meeting.recorder.mic")} value={status.mic_level} />
      )}
      <Level label={t("meeting.recorder.system")} value={status.system_level} />
      {status.system_output !== null && (
        <p className="text-xs text-mid-gray">
          {t("meeting.recorder.outputFrom", { device: status.system_output })}
        </p>
      )}
      {status.call_output !== null && (
        <p
          role="alert"
          className="flex items-center gap-2 text-xs text-warning"
        >
          {t("meeting.recorder.callElsewhere", { device: status.call_output })}
          <Button
            size="sm"
            variant="secondary"
            onClick={() => props.onSwitchOutput(status.call_output ?? "")}
          >
            {t("meeting.recorder.recordFrom")}
          </Button>
        </p>
      )}
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
