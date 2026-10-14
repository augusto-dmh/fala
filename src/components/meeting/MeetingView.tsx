import { useEffect, useMemo, useRef, useState } from "react";
import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type MeetingDetail,
  type MeetingError,
  type MeetingErased,
  type MeetingKeys,
  type MeetingLine,
  type MeetingTemplate,
} from "@/bindings";
import { copyToClipboard } from "../settings/history/clipboard";
import { Button } from "../ui/Button";
import { SearchField } from "../ui/SearchField";
import { AnnotationsField } from "./AnnotationsField";
import { DeleteConfirm, trashNeedsConfirm } from "./DeleteMeeting";
import {
  EraseBar,
  EraseConfirm,
  stretchOf,
  type Stretch,
} from "./EraseStretch";
import { ClipPlayer } from "./clipPlayer";
import { MeetingPeoplePanel } from "./MeetingPeoplePanel";
import { NotesEditor } from "./NotesEditor";
import {
  errorKey,
  needsConfirmation,
  sessionHints,
  titleOf,
} from "./meetingModel";
import { NotesView } from "./NotesView";
import { metaLine, SessionMenu, TitleField } from "./SessionHeader";
import { parseNotes } from "./notesModel";
import { shownTemplate, TemplateMenu, TemplateSelect } from "./TemplateSelect";
import { segmentAnchor, TranscriptView } from "./TranscriptView";
import { SessionBar, type LiveControls } from "./SessionBar";
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
  /** A name or the title changed, the notes were edited or a stretch erased: reload the session. */
  onNamesChanged?: () => void;
  /** "Move to trash", after the confirmation when there is audio to lose. */
  onDelete?: () => void;
  /** Recording, transcribing or writing notes: nothing to move yet. */
  deleteDisabled?: boolean;
  /** This session's recording: the bar's pause and stop, or the processing line. */
  live?: LiveControls | null;
  /** Which meeting keys are saved; `null` until known (nothing is turned off meanwhile). */
  keys?: MeetingKeys | null;
  /** "Retomar": record a new take of this finished session (absent: it cannot take one). */
  onContinue?: () => void;
  /** A take was just transcribed after notes existed: offer to write them again. */
  takeTranscribed?: boolean;
  onTakeSeen?: () => void;
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
  // Something is typed in "My notes" now, ahead of the save: notes can be written from it alone.
  // A boolean, so typing does not re-render the session on every key.
  const [typedHere, setTypedHere] = useState(false);
  const hints = sessionHints(props.keys ?? null, {
    transcript: detail.lines.length > 0,
    typed: typedHere || detail.annotations.trim() !== "",
    noAudio: detail.summary.imported_from === "granola",
  });
  // "Gerar notas" and the buttons that write them again are off together.
  const notesOff =
    props.recording || props.generating || editing || hints.generate !== null;
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
  const [transcriptOpen, setTranscriptOpen] = useState(false);
  const granola = detail.summary.imported_from === "granola";
  // Escape closes the sheet, like the menus.
  useEffect(() => {
    if (!transcriptOpen) return;
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setTranscriptOpen(false);
    };
    document.addEventListener("keydown", escape);
    return () => document.removeEventListener("keydown", escape);
  }, [transcriptOpen]);
  const [deleting, setDeleting] = useState(false);
  // "Erase stretch": the lines picked so far (`null`: not picking), the stretch to confirm, and
  // what the last erase did, for the "transcript changed" line.
  const [chosen, setChosen] = useState<number[] | null>(null);
  const [erasing, setErasing] = useState<Stretch | null>(null);
  const [erased, setErased] = useState<MeetingErased | null>(null);
  const [eraseError, setEraseError] = useState<MeetingError | null>(null);
  const [erasingNow, setErasingNow] = useState(false);
  const stretch = chosen === null ? null : stretchOf(detail.lines, chosen);
  useEffect(() => {
    if (props.deleteDisabled) {
      setDeleting(false);
      setChosen(null);
      setErasing(null);
    }
  }, [props.deleteDisabled]);
  // A third click starts over from that line.
  const pick = (line: MeetingLine) =>
    setChosen((current) =>
      current === null || current.includes(line.seq)
        ? current
        : current.length >= 2
          ? [line.seq]
          : [...current, line.seq],
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
  const erase = async (stretch: Stretch) => {
    setErasing(null);
    setChosen(null);
    setErasingNow(true);
    const result = await commands.eraseMeetingRange(
      detail.summary.id,
      stretch.t0_ms,
      stretch.t1_ms,
    );
    if (!mounted.current) return;
    setErasingNow(false);
    setEraseError(result.status === "error" ? result.error : null);
    if (result.status === "ok") {
      setErased(result.data);
      props.onNamesChanged?.();
    }
  };
  const generate = () => {
    setOfferRegenerate(false);
    setErased(null);
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
      placement="up"
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
    // The sheet opens first, with the whole transcript when the search hides the source.
    setTranscriptOpen(true);
    if (!found.some((line) => line.seq === seq)) setQuery("");
    requestAnimationFrame(scroll);
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
      onTextChange={(text) => setTypedHere(text.trim() !== "")}
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
            onDelete={() => {
              // Without audio, the toast's [Undo] is enough.
              if (trashNeedsConfirm(detail)) setDeleting(true);
              else props.onDelete?.();
            }}
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
          kind="trash"
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
        annotations
      ) : (
        <>
          {props.notesFailed && !props.generating && (
            <div
              role="alert"
              className="flex flex-wrap items-center gap-2 text-sm"
            >
              <span>{t("meeting.view.notesFailed")}</span>
              <Button
                variant="secondary"
                size="sm"
                disabled={props.recording || editing || hints.generate !== null}
                onClick={generate}
              >
                {t("meeting.view.retry")}
              </Button>
            </div>
          )}
          {props.takeTranscribed && detail.notes_md !== null && !confirming && (
            <div
              role="status"
              className="flex flex-wrap items-center gap-2 text-sm"
            >
              <span>{t("meeting.erase.changedNone")}</span>
              <Button
                variant="secondary"
                size="sm"
                disabled={notesOff}
                onClick={() => {
                  props.onTakeSeen?.();
                  generate();
                }}
              >
                {t("meeting.rename.generate")}
              </Button>
              <Button variant="ghost" size="sm" onClick={props.onTakeSeen}>
                {t("meeting.rename.dismiss")}
              </Button>
            </div>
          )}
          {erased !== null && detail.notes_md !== null && !confirming && (
            <div
              role="status"
              className="flex flex-wrap items-center gap-2 text-sm"
            >
              <span>
                {erased.notes_lines > 0
                  ? t("meeting.erase.changed", { count: erased.notes_lines })
                  : t("meeting.erase.changedNone")}
              </span>
              <Button
                variant="secondary"
                size="sm"
                disabled={notesOff}
                onClick={generate}
              >
                {t("meeting.rename.generate")}
              </Button>
              <Button variant="ghost" size="sm" onClick={() => setErased(null)}>
                {t("meeting.rename.dismiss")}
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
                disabled={notesOff}
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
                disabled={notesOff}
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
                {granola && (
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
                    locale={i18n.language}
                    t={t}
                  />
                )}
              </>
            )}
          </div>
          <aside
            hidden={!transcriptOpen}
            aria-label={t("meeting.view.transcript")}
            // Over the bar (z-40), under dialogs (z-50), down to the window's bottom: the bar
            // never cuts a line, and the ✕ (or Esc) gives the bar back, as Granola's sheet does.
            className="fixed inset-y-0 end-0 z-[45] w-[min(28rem,100vw)] space-y-2 overflow-y-auto border-s border-border bg-surface-1 p-4 shadow-xl"
          >
            <div className="flex items-center justify-between gap-2">
              <h2 className="text-subtitle font-semibold">
                {t("meeting.view.transcript")}
              </h2>
              <Button
                variant="ghost"
                className="px-2"
                aria-label={t("meeting.bar.close")}
                onClick={() => setTranscriptOpen(false)}
              >
                <X size={16} strokeWidth={1.5} aria-hidden="true" />
              </Button>
            </div>
            {detail.lines.length === 0 ? (
              <p className="text-sm">{t("meeting.view.noTranscript")}</p>
            ) : (
              <>
                {(granola || !detail.audio_available) && (
                  <p className="text-caption text-text-2">
                    {t(
                      granola
                        ? "meeting.granola.noAudio"
                        : "meeting.source.noAudio",
                    )}
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
                {eraseError !== null && (
                  <p role="alert" className="text-caption text-danger">
                    {t(errorKey(eraseError))}
                  </p>
                )}
                {erasingNow ? (
                  <p role="status" className="text-caption text-text-2">
                    {t("meeting.erase.running")}
                  </p>
                ) : erasing !== null ? (
                  <EraseConfirm
                    stretch={erasing}
                    audio={detail.audio_available || !!detail.audio_pending}
                    onConfirm={() => void erase(erasing)}
                    onCancel={() => setErasing(null)}
                    t={t}
                  />
                ) : chosen !== null ? (
                  <EraseBar
                    picked={chosen.length}
                    stretch={stretch}
                    onErase={() => setErasing(stretch)}
                    onCancel={() => setChosen(null)}
                    t={t}
                  />
                ) : (
                  !granola && (
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={props.deleteDisabled ?? true}
                      onClick={() => {
                        setQuery("");
                        setEraseError(null);
                        setChosen([]);
                      }}
                    >
                      {t("meeting.erase.start")}
                    </Button>
                  )
                )}
                <div className="flex items-center gap-2">
                  <SearchField
                    variant="compact"
                    aria-label={t("meeting.view.search")}
                    placeholder={t("meeting.view.search")}
                    // Picking shows every line: a stretch never takes one out of sight.
                    disabled={chosen !== null}
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
                  lines={chosen === null ? found : detail.lines}
                  sleeps={detail.sleeps ?? []}
                  query={query}
                  audioAvailable={detail.audio_available}
                  playing={playing}
                  highlighted={highlighted}
                  onPlay={play}
                  timed={!granola}
                  picking={
                    chosen === null
                      ? undefined
                      : {
                          picked: chosen,
                          taken: stretch?.taken ?? new Set(),
                          onPick: pick,
                        }
                  }
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
          </aside>
        </>
      )}
      <SessionBar
        draft={draft}
        live={props.live ?? null}
        transcriptOpen={transcriptOpen}
        onToggleTranscript={() => setTranscriptOpen(!transcriptOpen)}
        onRecordDraft={props.onRecordDraft}
        onContinue={
          props.onContinue &&
          !draft &&
          !granola &&
          detail.summary.mode !== "import" &&
          detail.summary.ended_at !== null
            ? props.onContinue
            : undefined
        }
        continueDisabled={
          props.recording || props.transcribing || props.generating
        }
        transcribing={props.transcribing}
        transcribeHint={hints.transcribe ? t(hints.transcribe) : undefined}
        transcribeDisabled={
          props.recording ||
          props.transcribing ||
          hints.transcribe !== null ||
          // Granola sends no audio: there is nothing to transcribe.
          detail.summary.imported_from === "granola"
        }
        onTranscribe={props.onTranscribe}
        onCancelTranscription={props.onCancelTranscription}
        generateLabel={
          props.generating ? generatingLabel : t("meeting.view.generate")
        }
        generateDisabled={notesOff}
        generateHint={hints.generate ? t(hints.generate) : undefined}
        onGenerate={generate}
        templateSelect={templateSelect}
        templateMenu={(close) => (
          <TemplateMenu
            templates={props.templates}
            value={picked}
            onChange={pickTemplate}
            onPicked={close}
          />
        )}
        t={t}
      />
    </div>
  );
}
