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
import { DeleteConfirm } from "./DeleteMeeting";
import { ClipPlayer } from "./clipPlayer";
import { MeetingPeoplePanel } from "./MeetingPeoplePanel";
import { NotesEditor } from "./NotesEditor";
import { errorKey, needsConfirmation, titleOf } from "./meetingModel";
import { NotesView } from "./NotesView";
import { metaLine, SessionMenu, TitleField } from "./SessionHeader";
import { parseNotes } from "./notesModel";
import { shownTemplate, TemplateSelect } from "./TemplateSelect";
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
  /** A name or the title changed, or the notes were edited: reload the session. */
  onNamesChanged?: () => void;
  onDelete?: () => void;
  /** Recording, transcribing or writing notes: nothing to delete yet. */
  deleteDisabled?: boolean;
}

export type SessionTab = "annotations" | "notes";

/** One session as a note: the title, "My notes | Notes", the actions and the transcript. */
export function MeetingView(props: MeetingViewProps) {
  const { t, i18n } = useTranslation();
  const { detail } = props;
  const [templateId, setTemplateId] = useState(
    detail.notes_template ?? props.templates[0]?.id ?? "geral",
  );
  // A user template deleted or broken after the pick: show and use the one the select shows.
  const picked = shownTemplate(props.templates, templateId);
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
  // Notes that exist open on "Notes" (unless it records); notes that arrive switch to them, as
  // in Granola. Both panels stay mounted, so switching never loses what is being typed.
  const hasNotes = notes !== null;
  const [tab, setTab] = useState<SessionTab>(
    hasNotes && !props.recording ? "notes" : "annotations",
  );
  const hadNotes = useRef(hasNotes);
  useEffect(() => {
    if (hasNotes && !hadNotes.current) setTab("notes");
    hadNotes.current = hasNotes;
  }, [hasNotes]);
  const [showPeople, setShowPeople] = useState(false);
  const [deleting, setDeleting] = useState(false);
  useEffect(() => {
    if (props.deleteDisabled) setDeleting(false);
  }, [props.deleteDisabled]);

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
    if (needsConfirmation({ ...detail, notes_edited: edited }, picked)) {
      setConfirming(true);
    } else {
      props.onGenerate(picked, false);
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
      value={picked}
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

  const tabButton = (id: SessionTab, label: string) => (
    <button
      type="button"
      role="tab"
      aria-selected={tab === id}
      onClick={() => setTab(id)}
      className={`rounded-md px-3 py-1 text-body transition-colors focus-visible:focus-ring ${
        tab === id
          ? "bg-surface-1 font-semibold text-text"
          : "text-text-2 hover:text-text"
      }`}
    >
      {label}
    </button>
  );
  const annotations = (
    <AnnotationsField
      id={detail.summary.id}
      initial={detail.annotations}
      label={draft ? t("meeting.view.agenda") : t("meeting.view.annotations")}
      labelHidden={!draft}
      slashMenu={(clear) => (
        <div
          role="group"
          aria-label={t("meeting.screen.slash")}
          className="flex flex-wrap items-center gap-1 text-caption text-text-2"
        >
          {t("meeting.screen.slash")}
          {props.templates.map((template) => (
            <Button
              key={template.id}
              size="sm"
              variant={template.id === picked ? "primary" : "secondary"}
              onClick={() => {
                pickTemplate(template.id);
                clear();
              }}
            >
              {template.name}
            </Button>
          ))}
        </div>
      )}
    />
  );

  return (
    <div className="space-y-4 p-4">
      <header className="space-y-1">
        <div className="flex items-start gap-2">
          <TitleField
            id={detail.summary.id}
            title={detail.summary.title}
            fallback={titleOf(detail.summary, t)}
            onSaved={() => props.onNamesChanged?.()}
            t={t}
          />
          <SessionMenu
            transcript={detail.lines.length > 0}
            deleteDisabled={props.deleteDisabled ?? true}
            onCopy={props.onCopy}
            onCopyTranscript={() =>
              void copyToClipboard(transcriptText(detail.lines, t))
            }
            onPeople={() => setShowPeople(true)}
            onDelete={() => setDeleting(true)}
            t={t}
          />
        </div>
        <p className="text-caption text-text-2 tabular-nums">
          {metaLine(
            detail.summary,
            new Date(),
            i18n.language,
            t,
            props.recording,
          )}
        </p>
      </header>
      {deleting && props.onDelete && (
        <DeleteConfirm
          onConfirm={() => {
            setDeleting(false);
            props.onDelete?.();
          }}
          onCancel={() => setDeleting(false)}
          t={t}
        />
      )}
      <MeetingPeoplePanel
        id={detail.summary.id}
        reloadOn={detail}
        expanded={showPeople}
        onNamesChanged={namesChanged}
      />
      {draft ? (
        <>
          {annotations}
          <div className="flex flex-wrap items-center gap-2">
            <Button onClick={props.onRecordDraft}>
              {t("meeting.view.recordDraft")}
            </Button>
            {templateSelect}
          </div>
        </>
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="secondary"
              disabled={
                props.recording ||
                props.transcribing ||
                // Granola sends no audio: there is nothing to transcribe.
                detail.summary.imported_from === "granola"
              }
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
                  props.onGenerate(picked, true);
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
          <div
            role="tablist"
            aria-label={t("meeting.screen.tabs")}
            className="inline-flex gap-1 rounded-lg bg-surface-2 p-1"
          >
            {tabButton("annotations", t("meeting.screen.myNotes"))}
            {tabButton("notes", t("meeting.view.notes"))}
          </div>
          <div role="tabpanel" hidden={tab !== "annotations"}>
            {annotations}
          </div>
          <div role="tabpanel" hidden={tab !== "notes"} className="space-y-1">
            {notes === null ? (
              <p className="text-body text-text-2">
                {props.generating
                  ? generatingLabel
                  : t("meeting.screen.noNotes")}
              </p>
            ) : (
              <>
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
                {detail.summary.imported_from === "granola" && (
                  <p className="text-caption text-text-2">
                    {t("meeting.granola.noAnchors")}
                  </p>
                )}
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
              </>
            )}
          </div>
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
        </>
      )}
    </div>
  );
}
