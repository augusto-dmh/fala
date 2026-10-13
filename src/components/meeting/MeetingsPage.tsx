import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { ChevronLeft } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  events,
  type MeetingDetail,
  type ImportSource,
  type MeetingError,
  type MeetingKeys,
  type MeetingMode,
  type MeetingOrphan,
  type MeetingOutputs,
  type MeetingSearchHit,
  type MeetingStatus,
  type MeetingSummary,
  type ProgressStage,
} from "@/bindings";
import { copyToClipboard } from "../settings/history/clipboard";
import { PageHeader } from "../PageHeader";
import { Button } from "../ui/Button";
import { SearchField } from "../ui/SearchField";
import { ConsentNotice } from "./ConsentNotice";
import { GranolaImport } from "./GranolaImport";
import { ImportDialog } from "./ImportDialog";
import { MeetingList } from "./MeetingList";
import { NewMeetingButton, RecordingStrip } from "./MeetingHome";
import { SEARCH_AFTER_MS, searchResults } from "./meetingSearch";
import { MeetingView } from "./MeetingView";
import { useMeetingTemplates } from "./useMeetingTemplates";
import { RecorderPanel } from "./RecorderPanel";
import { RecoverBanner } from "./RecoverBanner";
import {
  errorDetail,
  errorKey,
  takeConsentRequest,
  takeOpenMeetingRequest,
  titleOf,
} from "./meetingModel";

const IDLE: MeetingStatus = {
  state: "idle",
  id: null,
  mode: null,
  recorded_ms: 0,
  mic_level: 0,
  system_level: 0,
  muted_mic: false,
  muted_system: false,
  cap_remaining_ms: null,
  low_disk_bytes: null,
  system_output: null,
  call_output: null,
  zero_mic: false,
  zero_system: false,
  dictating: false,
};

const withoutId = (ids: Set<string>, id: string) => {
  if (!ids.has(id)) return ids;
  const next = new Set(ids);
  next.delete(id);
  return next;
};

export interface MeetingsPageProps {
  /** Settings, where the ElevenLabs key goes. */
  onOpenSettings?: () => void;
}

/** The Meetings page: the sessions by day with "New meeting" on top, or the one open. */
export function MeetingsPage({ onOpenSettings }: MeetingsPageProps) {
  const { t } = useTranslation();
  const [status, setStatus] = useState<MeetingStatus>(IDLE);
  const [sessions, setSessions] = useState<MeetingSummary[]>([]);
  const [selected, setSelected] = useState<MeetingDetail | null>(null);
  const [live, setLive] = useState<MeetingDetail | null>(null);
  const [catalog] = useMeetingTemplates();
  const templates = catalog.templates;
  const [keys, setKeys] = useState<MeetingKeys | null>(null);
  const [mode, setMode] = useState<MeetingMode>("meeting");
  const [title, setTitle] = useState("");
  const [askConsent, setAskConsent] = useState(takeConsentRequest);
  const [pendingDraft, setPendingDraft] = useState<string | null>(null);
  const [transcribing, setTranscribing] = useState<string | null>(null);
  const [generating, setGenerating] = useState<string | null>(null);
  // Sessions whose last notes generation failed; the stage before `failed` says which step.
  const [notesFailed, setNotesFailed] = useState<Set<string>>(new Set());
  const lastStage = useRef(new Map<string, string>());
  // Sessions whose last transcription or notes run failed: the red dot in the list.
  const [failed, setFailed] = useState<Set<string>>(new Set());
  const [error, setError] = useState<MeetingError | string | null>(null);
  const [outputs, setOutputs] = useState<MeetingOutputs | null>(null);
  const [query, setQuery] = useState("");
  // The last search's hits; `null` while the box is empty.
  const [hits, setHits] = useState<MeetingSearchHit[] | null>(null);
  const [orphans, setOrphans] = useState<MeetingOrphan[]>([]);
  const [recovering, setRecovering] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);
  const [importStage, setImportStage] = useState<ProgressStage | null>(null);
  // The session the running import writes, known from its first progress event.
  const importId = useRef<string | null>(null);
  const importRunning = useRef(false);
  const [granolaPath, setGranolaPath] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const list = await commands.listMeetings();
    if (list.status === "ok") setSessions(list.data);
    else setError(list.error);
  }, []);

  const open = useCallback(async (id: string) => {
    const result = await commands.getMeeting(id);
    if (result.status === "ok") setSelected(result.data);
    else setError(result.error);
  }, []);

  // Opening a session replaces the list, so a reload after a run only refreshes the session
  // still on screen and never brings back one the person left. `orHome`: also open it on the
  // list (an import that ends), never over another session.
  const reload = useCallback(async (id: string, orHome = false) => {
    const result = await commands.getMeeting(id);
    if (result.status !== "ok") return;
    setSelected((current) =>
      current?.summary.id === id || (orHome && current === null)
        ? result.data
        : current,
    );
  }, []);

  // The search follows the box and the list: a session transcribed meanwhile is searched again.
  useEffect(() => {
    if (!query.trim()) {
      setHits(null);
      return;
    }
    let current = true;
    const timer = setTimeout(() => {
      void commands.searchMeetings(query).then((result) => {
        if (!current) return;
        if (result.status === "ok") setHits(result.data);
        else setError(result.error);
      });
    }, SEARCH_AFTER_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [query, sessions]);

  useEffect(() => {
    void commands
      .meetingStatus()
      .then((r) => r.status === "ok" && setStatus(r.data));
    void commands
      .meetingKeys()
      .then((r) => r.status === "ok" && setKeys(r.data));
    void refresh();
    void commands
      .meetingOrphans()
      .then((r) => r.status === "ok" && setOrphans(r.data));
    // Mounted by a click on the meeting pill: open the session it records.
    const requested = takeOpenMeetingRequest();
    if (requested) void open(requested);
    const unlisteners = [
      events.meetingStatus.listen((event) => setStatus(event.payload)),
      events.meetingProgress.listen((event) => {
        const { id, stage } = event.payload;
        const before = lastStage.current.get(id);
        lastStage.current.set(id, stage.stage);
        if (stage.stage === "importing") importId.current = id;
        if (id === importId.current) {
          if (stage.stage === "importing" || stage.stage === "retaining")
            setImportStage(stage);
          if (stage.stage === "done" || stage.stage === "failed")
            setImporting(false);
        }
        if (stage.stage === "transcribing" || stage.stage === "notes")
          setFailed((ids) => withoutId(ids, id));
        if (stage.stage === "failed") setFailed((ids) => new Set(ids).add(id));
        if (stage.stage === "transcribing") setTranscribing(id);
        if (stage.stage === "notes") {
          setGenerating(id);
          setNotesFailed((ids) => withoutId(ids, id));
        }
        if (stage.stage === "failed" && before === "notes") {
          setNotesFailed((ids) => new Set(ids).add(id));
        }
        if (stage.stage === "done" || stage.stage === "failed") {
          setTranscribing((current) => (current === id ? null : current));
          setGenerating((current) => (current === id ? null : current));
          void refresh();
          void reload(id);
        }
      }),
      // The page is open: the app does not leave a request for the next mount.
      listen("meeting-consent-required", () => setAskConsent(true)),
      // A card's [Discard] or an empty recording: the session is gone.
      listen<string>("meeting-deleted", (event) => {
        setSelected((current) =>
          current?.summary.id === event.payload ? null : current,
        );
        void refresh();
      }),
      listen<string | null>("open-meeting", (event) => {
        if (event.payload) void open(event.payload);
      }),
    ];
    return () => {
      unlisteners.forEach((p) => void p.then((unlisten) => unlisten()));
    };
  }, [refresh, open, reload]);

  // The notes field of the session being recorded starts from what the draft already had.
  useEffect(() => {
    if (status.id === null) {
      setLive(null);
      void refresh();
      return;
    }
    if (live?.summary.id !== status.id) {
      void commands
        .getMeeting(status.id)
        .then((r) => r.status === "ok" && setLive(r.data));
    }
  }, [status.id, live?.summary.id, refresh]);

  // The outputs are listed again whenever the panel goes back to idle: a headset may have been
  // plugged in during the last call.
  const idle = status.state === "idle";
  useEffect(() => {
    if (!idle) return;
    void commands
      .meetingOutputs()
      .then((r) => r.status === "ok" && setOutputs(r.data));
  }, [idle]);

  const chooseOutput = (name: string | null) => {
    setOutputs((current) => current && { ...current, selected: name });
    void commands.setMeetingOutput(name);
  };

  // The open session follows the recording: a draft that starts, a session that stops.
  const openId = selected?.summary.id ?? null;
  useEffect(() => {
    if (openId !== null) void reload(openId);
  }, [status.state, status.id, openId, reload]);

  const start = async (draftId: string | null) => {
    setError(null);
    const result = await commands.startMeeting(mode, title, draftId);
    if (result.status === "ok") {
      setStatus(result.data);
      setTitle("");
      // The session being recorded opens, with its notes field and the controls.
      if (result.data.id !== null) await open(result.data.id);
    } else if (
      typeof result.error === "object" &&
      result.error.kind === "consent_required"
    ) {
      setPendingDraft(draftId);
      setAskConsent(true);
    } else {
      setError(result.error);
    }
  };

  const act = async (
    call: () => Promise<
      | { status: "ok"; data: MeetingStatus }
      | { status: "error"; error: MeetingError }
    >,
  ) => {
    const result = await call();
    if (result.status === "ok") setStatus(result.data);
    else setError(result.error);
  };

  const transcribe = async (id: string) => {
    setError(null);
    setTranscribing(id);
    const result = await commands.transcribeMeeting(id);
    setTranscribing(null);
    if (result.status === "error") setError(result.error);
    await reload(id);
    await refresh();
  };

  const generate = async (
    id: string,
    templateId: string,
    replaceEdited: boolean,
  ) => {
    setError(null);
    setGenerating(id);
    const result = await commands.generateMeetingNotes(
      id,
      templateId,
      replaceEdited,
    );
    setGenerating((current) => (current === id ? null : current));
    if (result.status === "error") setError(result.error);
    await reload(id);
    await refresh();
  };

  const importSource = async (source: ImportSource) => {
    if (importRunning.current) return;
    importRunning.current = true;
    importId.current = null;
    setError(null);
    setImportStage(null);
    setImporting(true);
    const result = await commands.importMeeting(source);
    importRunning.current = false;
    // `busy`: an import started before this page mounted is still running; its progress
    // events keep the line and Cancel on screen until it ends.
    const busy = result.status === "error" && result.error.kind === "busy";
    if (!busy) setImporting(false);
    await refresh();
    if (result.status === "ok") await reload(result.data, true);
    else if (result.error.kind !== "cancelled" && !busy) setError(result.error);
  };

  // A file dropped on the window while the list is open is imported, one at a time, and
  // never behind the consent notice or over a session, where the import line is hidden.
  const onDrop = useRef<(path: string) => void>(() => {});
  useEffect(() => {
    onDrop.current = (path) => {
      if (!askConsent && selected === null)
        void importSource({ kind: "file", value: path });
    };
  });
  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type !== "drop") return;
      const [path] = event.payload.paths;
      if (path) onDrop.current(path);
    });
    return () => void unlisten.then((stop) => stop());
  }, []);

  const copy = async (id: string) => {
    const result = await commands.meetingMarkdown(id);
    if (result.status === "ok") await copyToClipboard(result.data);
    else setError(result.error);
  };

  const newDraft = async () => {
    const result = await commands.createMeetingDraft(title);
    if (result.status === "ok") {
      setTitle("");
      await refresh();
      await open(result.data);
    } else setError(result.error);
  };

  const recover = async (id: string) => {
    setError(null);
    setRecovering(id);
    const result = await commands.recoverMeeting(id);
    setRecovering(null);
    if (result.status === "error") {
      setError(result.error);
      return;
    }
    setOrphans((current) => current.filter((o) => o.id !== id));
    await refresh();
    await open(id);
  };

  const remove = async (id: string) => {
    setError(null);
    const result = await commands.deleteMeeting(id);
    if (result.status === "ok") {
      setSelected((current) => (current?.summary.id === id ? null : current));
      setOrphans((current) => current.filter((o) => o.id !== id));
    } else setError(result.error);
    await refresh();
  };

  const recordingNow =
    status.state === "recording" || status.state === "paused";
  const recorder = (controls: boolean) => (
    <RecorderPanel
      controls={controls}
      status={status}
      mode={mode}
      title={title}
      outputs={outputs}
      onModeChange={setMode}
      onTitleChange={setTitle}
      onOutputChange={chooseOutput}
      onSwitchOutput={(name) =>
        void act(() => commands.switchMeetingOutput(name))
      }
      onStart={() => void start(null)}
      onNewDraft={() => void newDraft()}
      onPause={() => void act(commands.pauseMeeting)}
      onResume={() => void act(commands.resumeMeeting)}
      onStop={() => void act(commands.stopMeeting)}
      onExtend={() => void act(commands.extendMeetingCap)}
      templates={templates}
      template={live?.notes_template ?? null}
      onTemplateChange={(templateId) => {
        if (!live) return;
        setLive({ ...live, notes_template: templateId });
        void commands
          .setMeetingTemplate(live.summary.id, templateId)
          .then(() => {
            // The same session open below follows the pick.
            if (selected?.summary.id === live.summary.id) {
              void reload(live.summary.id);
            }
          });
      }}
    />
  );
  // From the list, which a rename refreshes; `live` loads once per recording.
  const recordingSummary =
    sessions.find((s) => s.id === status.id) ?? live?.summary;
  const strip = (
    <RecordingStrip
      status={status}
      title={recordingSummary ? titleOf(recordingSummary, t) : ""}
      onOpen={() => status.id !== null && void open(status.id)}
      t={t}
    />
  );
  // On the home and on a session alike: the notice, a recording to recover, the last error.
  const banners = (
    <>
      {askConsent && (
        <ConsentNotice
          onAccept={async () => {
            setAskConsent(false);
            await commands.acceptMeetingConsent();
            await start(pendingDraft);
          }}
          onCancel={() => setAskConsent(false)}
        />
      )}
      <RecoverBanner
        orphans={orphans}
        busy={recovering !== null}
        onRecover={(id) => void recover(id)}
        t={t}
      />
      {error !== null && (
        <p role="alert" className="text-body text-danger">
          {t(errorKey(error), { detail: errorDetail(error) })}
        </p>
      )}
    </>
  );

  // The Granola import, first in both the list and the session: React keeps it mounted (and its
  // preview or progress) when a meeting opens or closes.
  const granola = granolaPath !== null && (
    <GranolaImport
      key={granolaPath}
      path={granolaPath}
      onImported={() => void refresh()}
      onClose={() => setGranolaPath(null)}
    />
  );

  if (selected) {
    // Recording, stopping or processing: the backend keeps the id until the pipeline takes over.
    const recordingThis = status.id === selected.summary.id;
    return (
      <div className="mx-auto w-full max-w-2xl space-y-4">
        {granola}
        <button
          type="button"
          onClick={() => setSelected(null)}
          className="flex items-center gap-1 rounded-lg text-body text-text-2 transition-colors hover:text-text focus-visible:focus-ring"
        >
          <ChevronLeft
            size={16}
            strokeWidth={1.5}
            className="rtl:rotate-180"
            aria-hidden="true"
          />
          {t("meeting.list.title")}
        </button>
        {banners}
        {importing && (
          <ImportDialog
            importing={importing}
            stage={importStage}
            onImport={(source) => void importSource(source)}
            onCancel={() => void commands.cancelMeetingImport()}
            onGranola={setGranolaPath}
          />
        )}
        {/* Recording this one: the levels and warnings on top, the controls in its bar. */}
        {recordingThis ? recordingNow && !askConsent && recorder(false) : strip}
        <div className="rounded-lg border border-border bg-surface-1">
          <MeetingView
            key={selected.summary.id}
            detail={selected}
            templates={templates}
            recording={recordingThis}
            transcribing={transcribing === selected.summary.id}
            generating={
              generating === selected.summary.id || selected.notes_running
            }
            notesFailed={notesFailed.has(selected.summary.id)}
            onRecordDraft={() => void start(selected.summary.id)}
            onTranscribe={() => void transcribe(selected.summary.id)}
            onCancelTranscription={() =>
              void commands.cancelMeetingTranscription()
            }
            onGenerate={(templateId, replaceEdited) =>
              void generate(selected.summary.id, templateId, replaceEdited)
            }
            onCopy={() => void copy(selected.summary.id)}
            onNamesChanged={() => {
              // The title may have changed: the list shows it too.
              void reload(selected.summary.id);
              void refresh();
            }}
            live={
              recordingThis
                ? {
                    status,
                    onPause: () => void act(commands.pauseMeeting),
                    onResume: () => void act(commands.resumeMeeting),
                    onStop: () => void act(commands.stopMeeting),
                  }
                : null
            }
            deleteDisabled={
              recordingThis ||
              transcribing === selected.summary.id ||
              generating === selected.summary.id ||
              selected.notes_running
            }
            onDelete={() => void remove(selected.summary.id)}
          />
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto w-full max-w-2xl space-y-4">
      {granola}
      <PageHeader
        title={t("meeting.list.title")}
        actions={
          <NewMeetingButton
            disabled={askConsent || recordingNow || status.state === "stopping"}
            onStart={() => void start(null)}
            options={recorder(true)}
            t={t}
          />
        }
      />
      {banners}
      {strip}
      <div className="flex flex-wrap items-center gap-2">
        <SearchField
          variant="compact"
          className="min-w-0 flex-1"
          value={query}
          placeholder={t("meeting.list.search")}
          aria-label={t("meeting.list.search")}
          onChange={(event) => setQuery(event.target.value)}
        />
        {!askConsent && (
          <ImportDialog
            importing={importing}
            stage={importStage}
            onImport={(source) => void importSource(source)}
            onCancel={() => void commands.cancelMeetingImport()}
            onGranola={setGranolaPath}
          />
        )}
      </div>
      {keys && !keys.elevenlabs && (
        <p className="flex flex-wrap items-center gap-2 text-caption text-text-2">
          {t("meeting.home.noKey")}
          {onOpenSettings && (
            <Button size="sm" variant="secondary" onClick={onOpenSettings}>
              {t("meeting.home.openSettings")}
            </Button>
          )}
        </p>
      )}
      <MeetingList
        {...(hits ? searchResults(sessions, hits) : { sessions })}
        live={{
          recording: recordingNow ? status.id : null,
          working: new Set(
            [transcribing, generating].filter((id) => id !== null),
          ),
          failed,
        }}
        onOpen={(id) => void open(id)}
      />
    </div>
  );
}
