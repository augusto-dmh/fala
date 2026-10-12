import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type MeetingDetail,
  type MeetingError,
  type MeetingLine,
  type MeetingTemplate,
} from "@/bindings";
import { copyToClipboard } from "../settings/history/clipboard";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { AnnotationsField } from "./AnnotationsField";
import { ClipPlayer } from "./clipPlayer";
import { MeetingPeoplePanel } from "./MeetingPeoplePanel";
import { NotesEditor } from "./NotesEditor";
import { errorKey, needsConfirmation, titleOf } from "./meetingModel";
import { NotesView } from "./NotesView";
import { parseNotes } from "./notesModel";
import { TemplateSelect } from "./TemplateSelect";
import { segmentAnchor, TranscriptView } from "./TranscriptView";
import { searchLines, transcriptText } from "./transcriptSearch";

export interface MeetingViewProps {
  detail: MeetingDetail;
  templates: MeetingTemplate[];
  /** This session is the one recording now. */
  recording: boolean;
  transcribing: boolean;
  generating: boolean;
  /** The last notes generation of this session failed (a click or after the stop). */
  notesFailed?: boolean;
  onRecordDraft: () => void;
  onTranscribe: () => void;
  onCancelTranscription: () => void;
  /** `replaceEdited`: the person confirmed discarding the notes they edited. */
  onGenerate: (templateId: string, replaceEdited: boolean) => void;
  onCopy: () => void;
  /** A name was applied or typed, or the notes were edited: reload the session. */
  onNamesChanged?: () => void;
}

/** One session: the draft's agenda, or the transcript, the typed notes and the notes. */
export function MeetingView(props: MeetingViewProps) {
  const { t } = useTranslation();
  const { detail } = props;
  const [templateId, setTemplateId] = useState(
    detail.notes_template ?? props.templates[0]?.id ?? "geral",
  );
  const draft = detail.summary.started_at === null;
  const [playing, setPlaying] = useState<number | null>(null);
  const [clipError, setClipError] = useState<MeetingError | null>(null);
  const [highlighted, setHighlighted] = useState<number | null>(null);
  const [nameError, setNameError] = useState<MeetingError | null>(null);
  const [query, setQuery] = useState("");
  const found = useMemo(
    () => searchLines(detail.lines, query),
    [detail.lines, query],
  );
  const [editing, setEditing] = useState(false);
  const [confirming, setConfirming] = useState(false);
  // Edited here: counts before the reload brings `notes_edited`.
  const [editedHere, setEditedHere] = useState(false);
  // A name changed in a session that already has notes: offer to write them again.
  const [offerRegenerate, setOfferRegenerate] = useState(false);
  const player = useMemo(() => new ClipPlayer(setPlaying), []);
  // One clip at a time, and none left playing after leaving the session.
  useEffect(() => () => player.stop(), [player, detail.summary.id]);
  const segments = useMemo(
    () => new Map(detail.lines.map((line) => [line.seq, line])),
    [detail.lines],
  );
  const notes = useMemo(
    () =>
      detail.notes_md === null
        ? null
        : parseNotes(detail.notes_md, detail.lines),
    [detail.notes_md, detail.lines],
  );

  const play = (line: MeetingLine) => {
    setClipError(null);
    if (playing === line.seq) {
      player.stop();
      return;
    }
    player
      .play(detail.summary.id, line.seq, line.t0_ms, line.t1_ms)
      .catch((error: MeetingError) => setClipError(error));
  };
  // A rename that ends after the session was left must not reload it over the next one.
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  // Notes that exist (or are being written) still have the old label: offer to write them again.
  const namesChanged = () => {
    if (detail.notes_md !== null || props.generating) setOfferRegenerate(true);
    props.onNamesChanged?.();
  };
  const rename = async (person: number, name: string) => {
    const result = await commands.setMeetingSpeakerName(
      detail.summary.id,
      person,
      name,
    );
    if (!mounted.current) return;
    setNameError(result.status === "error" ? result.error : null);
    if (result.status === "ok") namesChanged();
  };
  const generate = () => {
    setOfferRegenerate(false);
    const edited = detail.notes_edited || editedHere;
    if (needsConfirmation({ ...detail, notes_edited: edited }, templateId)) {
      setConfirming(true);
    } else {
      props.onGenerate(templateId, false);
    }
  };
  // The template stored with the session (picked in the recorder, or the one that wrote the
  // notes) wins over the local pick when the session reloads.
  useEffect(() => {
    if (detail.notes_template) setTemplateId(detail.notes_template);
  }, [detail.notes_template]);
  // Before the notes exist, the pick goes with the session: the notes after the stop use it.
  const pickTemplate = (id: string) => {
    setTemplateId(id);
    if (detail.notes_md === null) {
      void commands.setMeetingTemplate(detail.summary.id, id);
    }
  };
  const templateSelect = (
    <TemplateSelect
      templates={props.templates}
      value={templateId}
      onChange={pickTemplate}
    />
  );
  const generatingLabel = detail.annotations.trim()
    ? t("meeting.view.enhancing")
    : t("meeting.view.writing");
  const reveal = (seq: number) => {
    setHighlighted(seq);
    const scroll = () =>
      document
        .getElementById(segmentAnchor(seq))
        ?.scrollIntoView({ behavior: "smooth", block: "center" });
    // A source the search hides: show the whole transcript again first.
    if (!found.some((line) => line.seq === seq)) {
      setQuery("");
      requestAnimationFrame(scroll);
    } else scroll();
  };

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
      <MeetingPeoplePanel
        id={detail.summary.id}
        reloadOn={detail}
        onNamesChanged={namesChanged}
      />
      {draft ? (
        <div className="flex flex-wrap items-center gap-2">
          <Button onClick={props.onRecordDraft}>
            {t("meeting.view.recordDraft")}
          </Button>
          {templateSelect}
        </div>
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
            {templateSelect}
            <Button
              variant="secondary"
              disabled={props.recording || props.generating || editing}
              onClick={generate}
            >
              {props.generating ? generatingLabel : t("meeting.view.generate")}
            </Button>
            <Button variant="secondary" onClick={props.onCopy}>
              {t("meeting.view.copy")}
            </Button>
            {detail.lines.length > 0 && (
              <Button
                variant="secondary"
                onClick={() =>
                  void copyToClipboard(transcriptText(detail.lines, t))
                }
              >
                {t("meeting.view.copyTranscript")}
              </Button>
            )}
          </div>
          {props.notesFailed && !props.generating && (
            <div
              role="alert"
              className="flex flex-wrap items-center gap-2 text-sm"
            >
              <span>{t("meeting.view.notesFailed")}</span>
              <Button
                variant="secondary"
                size="sm"
                disabled={props.recording || editing}
                onClick={generate}
              >
                {t("meeting.view.retry")}
              </Button>
            </div>
          )}
          {offerRegenerate && !confirming && (
            <div
              role="status"
              className="flex flex-wrap items-center gap-2 text-sm"
            >
              <span>{t("meeting.rename.regenerate")}</span>
              <Button
                variant="secondary"
                size="sm"
                disabled={props.recording || props.generating || editing}
                onClick={generate}
              >
                {t("meeting.rename.generate")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setOfferRegenerate(false)}
              >
                {t("meeting.rename.dismiss")}
              </Button>
            </div>
          )}
          {confirming && (
            <div
              role="alertdialog"
              className="flex flex-wrap items-center gap-2 text-sm"
            >
              <span>{t("meeting.notesEdit.confirm")}</span>
              <Button
                variant="danger"
                size="sm"
                disabled={props.recording || props.generating || editing}
                onClick={() => {
                  setConfirming(false);
                  setEditedHere(false);
                  props.onGenerate(templateId, true);
                }}
              >
                {t("meeting.notesEdit.regenerate")}
              </Button>
              <Button
                variant="secondary"
                size="sm"
                onClick={() => setConfirming(false)}
              >
                {t("meeting.view.cancel")}
              </Button>
            </div>
          )}
          <section className="space-y-1">
            <h4 className="text-sm font-semibold">
              {t("meeting.view.transcript")}
            </h4>
            {detail.lines.length === 0 ? (
              <p className="text-sm">{t("meeting.view.noTranscript")}</p>
            ) : (
              <>
                {!detail.audio_available && (
                  <p className="text-caption text-text-2">
                    {t("meeting.source.noAudio")}
                  </p>
                )}
                {clipError !== null && (
                  <p role="alert" className="text-caption text-danger">
                    {t(errorKey(clipError))}
                  </p>
                )}
                {nameError !== null && (
                  <p role="alert" className="text-caption text-danger">
                    {t(errorKey(nameError))}
                  </p>
                )}
                <div className="flex items-center gap-2">
                  <Input
                    variant="compact"
                    type="search"
                    aria-label={t("meeting.view.search")}
                    placeholder={t("meeting.view.search")}
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                  />
                  {query.trim() !== "" && (
                    <span className="text-caption text-text-2">
                      {t("meeting.view.found", { count: found.length })}
                    </span>
                  )}
                </div>
                <TranscriptView
                  lines={found}
                  query={query}
                  audioAvailable={detail.audio_available}
                  playing={playing}
                  highlighted={highlighted}
                  onPlay={play}
                  // A new transcript renumbers "Person N" and drops the names.
                  onRename={
                    props.transcribing
                      ? undefined
                      : (person, name) => void rename(person, name)
                  }
                  t={t}
                />
              </>
            )}
          </section>
          {notes !== null && (
            <section className="space-y-1">
              <div className="flex items-center gap-2">
                <h4 className="text-sm font-semibold">
                  {t("meeting.view.notes")}
                </h4>
                {!editing && (
                  <Button
                    variant="ghost"
                    size="sm"
                    disabled={props.generating}
                    onClick={() => {
                      setConfirming(false);
                      setEditing(true);
                    }}
                  >
                    {t("meeting.notesEdit.edit")}
                  </Button>
                )}
              </div>
              {editing && detail.notes_md !== null ? (
                <NotesEditor
                  id={detail.summary.id}
                  markdown={detail.notes_md}
                  onDone={(edited) => {
                    setEditing(false);
                    if (edited) setEditedHere(true);
                    props.onNamesChanged?.();
                  }}
                />
              ) : (
                <NotesView
                  notes={notes}
                  segments={segments}
                  audioAvailable={detail.audio_available}
                  playing={playing}
                  onPlay={play}
                  onReveal={reveal}
                  t={t}
                />
              )}
            </section>
          )}
        </>
      )}
    </div>
  );
}
