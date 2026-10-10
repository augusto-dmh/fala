import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { MeetingDetail, MeetingTemplate } from "@/bindings";
import { Button } from "../ui/Button";
import { AnnotationsField } from "./AnnotationsField";
import { lineText, titleOf } from "./meetingModel";

export interface MeetingViewProps {
  detail: MeetingDetail;
  templates: MeetingTemplate[];
  /** This session is the one recording now. */
  recording: boolean;
  transcribing: boolean;
  generating: boolean;
  onRecordDraft: () => void;
  onTranscribe: () => void;
  onCancelTranscription: () => void;
  onGenerate: (templateId: string) => void;
  onCopy: () => void;
}

/** One session: the draft's agenda, or the transcript, the typed notes and the notes. */
export function MeetingView(props: MeetingViewProps) {
  const { t } = useTranslation();
  const { detail } = props;
  const [templateId, setTemplateId] = useState(
    detail.notes_template ?? props.templates[0]?.id ?? "geral",
  );
  const draft = detail.summary.started_at === null;

  return (
    <div className="space-y-4 p-4">
      <h3 className="text-base font-semibold">{titleOf(detail.summary, t)}</h3>
      {props.recording ? (
        // While it records, its notes are typed in the recorder above: one field per session.
        <p className="text-sm">{t("meeting.view.notesAbove")}</p>
      ) : (
        <AnnotationsField
          id={detail.summary.id}
          initial={detail.annotations}
          label={
            draft ? t("meeting.view.agenda") : t("meeting.view.annotations")
          }
        />
      )}
      {draft ? (
        <Button onClick={props.onRecordDraft}>
          {t("meeting.view.recordDraft")}
        </Button>
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="secondary"
              disabled={props.recording || props.transcribing}
              onClick={props.onTranscribe}
            >
              {props.transcribing
                ? t("meeting.view.transcribing")
                : t("meeting.view.transcribe")}
            </Button>
            {props.transcribing && (
              <Button variant="ghost" onClick={props.onCancelTranscription}>
                {t("meeting.view.cancel")}
              </Button>
            )}
            <select
              aria-label={t("meeting.view.template")}
              className="rounded-md border border-mid-gray/80 bg-mid-gray/10 px-2 py-1 text-sm"
              value={templateId}
              onChange={(event) => setTemplateId(event.target.value)}
            >
              {props.templates.map((template) => (
                <option key={template.id} value={template.id}>
                  {template.name}
                </option>
              ))}
            </select>
            <Button
              variant="secondary"
              disabled={props.recording || props.generating}
              onClick={() => props.onGenerate(templateId)}
            >
              {props.generating
                ? t("meeting.view.generating")
                : t("meeting.view.generate")}
            </Button>
            <Button variant="secondary" onClick={props.onCopy}>
              {t("meeting.view.copy")}
            </Button>
          </div>
          <section className="space-y-1">
            <h4 className="text-sm font-semibold">
              {t("meeting.view.transcript")}
            </h4>
            {detail.lines.length === 0 ? (
              <p className="text-sm">{t("meeting.view.noTranscript")}</p>
            ) : (
              detail.lines.map((line) => (
                <p key={line.seq} className="text-sm">
                  {lineText(line, t)}
                </p>
              ))
            )}
          </section>
          {detail.notes_md !== null && (
            <section className="space-y-1">
              <h4 className="text-sm font-semibold">
                {t("meeting.view.notes")}
              </h4>
              <pre className="whitespace-pre-wrap text-sm">
                {detail.notes_md}
              </pre>
            </section>
          )}
        </>
      )}
    </div>
  );
}
