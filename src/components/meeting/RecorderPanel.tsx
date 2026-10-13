import { useId, useState, type ReactNode } from "react";
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
import { Dropdown, type DropdownOption } from "../ui/Dropdown";
import { SettingTitleContext } from "../ui/SettingContainer";
import { Input } from "../ui/Input";
import { clock, levelPercent } from "./meetingModel";
import { shownTemplate, TemplateSelect } from "./TemplateSelect";

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
  /** `false` on the meeting itself: its bar has the time, pause and stop. */
  controls?: boolean;
}

/** A recorder warning: an amber strip, as in Granola. */
const STRIP =
  "flex flex-wrap items-center gap-2 rounded-lg border border-warn bg-warn/10 px-3 py-2 text-caption";

/** "Copy notice": a ready text for the call's chat, saying the meeting is being recorded. */
export function CopyNoticeButton({ size = "sm" }: { size?: "sm" | "md" }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    // "Copied" only when it was, and only for a moment.
    if (!(await copyToClipboard(t("meeting.notice.text")))) return;
    setCopied(true);
    setTimeout(() => setCopied(false), 3000);
  };
  return (
    <Button variant="secondary" size={size} onClick={() => void copy()}>
      {copied ? t("meeting.notice.copied") : t("meeting.notice.copy")}
    </Button>
  );
}

/** The options of "Meeting sound": the default output (named), each output, and a remembered
 *  one that is not plugged in now. The default is the empty value. */
export function outputOptions(
  outputs: MeetingOutputs,
  defaultLabel: (name: string) => string,
): DropdownOption[] {
  const fallback = outputs.devices.find((d) => d.is_default)?.name ?? "";
  const options = [
    { value: "", label: defaultLabel(fallback) },
    ...outputs.devices.map((d) => ({ value: d.name, label: d.name })),
  ];
  if (
    outputs.selected !== null &&
    !outputs.devices.some((d) => d.name === outputs.selected)
  ) {
    options.push({ value: outputs.selected, label: outputs.selected });
  }
  return options;
}

/** A recorder choice: its name above the control, as in the rest of the app. The name labels
 *  the control: a Dropdown through the settings title context, an input through `labelledBy`. */
function Field({
  label,
  children,
}: {
  label: string;
  children: (labelledBy: string) => ReactNode;
}) {
  const id = useId();
  return (
    <div className="flex flex-col gap-1">
      <span id={id} className="text-caption text-text-2">
        {label}
      </span>
      <SettingTitleContext.Provider value={id}>
        {children(id)}
      </SettingTitleContext.Provider>
    </div>
  );
}

function Level({ label, value }: { label: string; value: number }) {
  const width = levelPercent(value);
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
    const outputs = props.outputs;
    const modes: DropdownOption[] = [
      { value: "meeting", label: t("meeting.recorder.online") },
      { value: "in_person", label: t("meeting.recorder.inPerson") },
      { value: "system_only", label: t("meeting.recorder.systemOnly") },
    ];
    return (
      <div className="flex flex-col gap-3 p-4">
        <div className="grid gap-3 sm:grid-cols-2">
          <Field label={t("meeting.recorder.mode")}>
            {() => (
              <Dropdown
                options={modes}
                selectedValue={props.mode}
                onSelect={(mode) => props.onModeChange(mode as MeetingMode)}
              />
            )}
          </Field>
          {outputs && (
            <Field label={t("meeting.recorder.output")}>
              {() => (
                <Dropdown
                  options={outputOptions(outputs, (name) =>
                    t("meeting.recorder.outputDefault", { name }),
                  )}
                  selectedValue={outputs.selected ?? ""}
                  onSelect={(name) => props.onOutputChange(name || null)}
                />
              )}
            </Field>
          )}
        </div>
        <Field label={t("meeting.recorder.title")}>
          {(labelledBy) => (
            <Input
              variant="compact"
              aria-labelledby={labelledBy}
              value={props.title}
              placeholder={t("meeting.recorder.titlePlaceholder")}
              onChange={(event) => props.onTitleChange(event.target.value)}
            />
          )}
        </Field>
        <div className="flex flex-wrap items-center gap-2">
          <Button
            onClick={props.onStart}
            disabled={status.state === "stopping"}
          >
            {t("meeting.recorder.start")}
          </Button>
          <Button variant="secondary" onClick={props.onNewDraft}>
            {t("meeting.recorder.newDraft")}
          </Button>
          <CopyNoticeButton size="md" />
          {status.state === "processing" && (
            <span className="text-xs">{t("meeting.recorder.processing")}</span>
          )}
        </div>
      </div>
    );
  }

  const paused = status.state === "paused";
  return (
    <div className="space-y-2 p-4">
      <div className="flex flex-wrap items-center gap-2">
        {props.controls !== false && (
          <>
            <span className="font-mono text-lg">
              {clock(status.recorded_ms)}
            </span>
            <span className="text-xs">
              {paused
                ? t("meeting.recorder.paused")
                : t("meeting.recorder.live")}
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
          </>
        )}
        <CopyNoticeButton />
        {/* On the meeting itself the template is in its "/" menu and its bar. */}
        {props.controls !== false &&
          props.templates &&
          props.onTemplateChange && (
            <TemplateSelect
              templates={props.templates}
              value={shownTemplate(props.templates, props.template ?? null)}
              onChange={props.onTemplateChange}
            />
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
        <p role="alert" className={STRIP}>
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
        <p role="alert" className={STRIP}>
          {t("meeting.recorder.mutedMic")}
        </p>
      )}
      {status.muted_system && (
        <p role="alert" className={STRIP}>
          {t("meeting.recorder.mutedSystem")}
        </p>
      )}
      {status.cap_remaining_ms !== null && (
        <p role="alert" className={STRIP}>
          {t("meeting.recorder.capWarning", {
            time: clock(status.cap_remaining_ms),
          })}
          <Button size="sm" variant="secondary" onClick={props.onExtend}>
            {t("meeting.recorder.extend")}
          </Button>
        </p>
      )}
      {status.low_disk_bytes !== null && (
        <p role="alert" className={STRIP}>
          {t("meeting.recorder.lowDisk", {
            mb: Math.floor(status.low_disk_bytes / (1024 * 1024)),
          })}
        </p>
      )}
    </div>
  );
}
