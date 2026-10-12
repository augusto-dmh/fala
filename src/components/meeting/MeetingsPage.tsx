import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
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
  type MeetingStatus,
  type MeetingSummary,
  type MeetingTemplates,
  type ProgressStage,
} from "@/bindings";
import { copyToClipboard } from "../settings/history/clipboard";
import { SettingsGroup } from "../ui/SettingsGroup";
import { AnnotationsField } from "./AnnotationsField";
import { ConsentNotice } from "./ConsentNotice";
import { DeleteMeeting } from "./DeleteMeeting";
import { GranolaImport } from "./GranolaImport";
import { ImportDialog } from "./ImportDialog";
import { KeyField } from "./KeyField";
import { MeetingList } from "./MeetingList";
import { MeetingView } from "./MeetingView";
import { NotesContextSettings } from "./NotesContextSettings";
import { RecorderPanel } from "./RecorderPanel";
import { RecoverBanner } from "./RecoverBanner";
import { TemplatesSettings } from "./TemplatesSettings";
import {
  errorDetail,
  errorKey,
  takeConsentRequest,
  takeOpenMeetingRequest,
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
};

const withoutId = (ids: Set<string>, id: string) => {
  if (!ids.has(id)) return ids;
  const next = new Set(ids);
  next.delete(id);
  return next;
};

/** The Meetings page: record, the list of sessions and the open one. */
export function MeetingsPage() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<MeetingStatus>(IDLE);
  const [sessions, setSessions] = useState<MeetingSummary[]>([]);
  const [selected, setSelected] = useState<MeetingDetail | null>(null);
  const [live, setLive] = useState<MeetingDetail | null>(null);
  const [catalog, setCatalog] = useState<MeetingTemplates>({
    templates: [],
    errors: [],
  });
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
  const [error, setError] = useState<MeetingError | string | null>(null);
  const [outputs, setOutputs] = useState<MeetingOutputs | null>(null);
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

  // The templates folder is read again on mount and whenever the app gets the focus back:
  // the person edits the JSON in another app (no file watcher).
  useEffect(() => {
    const load = () =>
      void commands
        .meetingTemplates()
        .then((r) => r.status === "ok" && setCatalog(r.data));
    load();
    window.addEventListener("focus", load);
    return () => window.removeEventListener("focus", load);
  }, []);

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
          setSelected((current) => {
            if (current?.summary.id === id) void open(id);
            return current;
          });
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
  }, [refresh, open]);

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
    if (openId !== null) void open(openId);
  }, [status.state, status.id, openId, open]);

  const start = async (draftId: string | null) => {
    setError(null);
    const result = await commands.startMeeting(mode, title, draftId);
    if (result.status === "ok") {
      setStatus(result.data);
      setTitle("");
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
    await open(id);
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
    await open(id);
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
    if (result.status === "ok") await open(result.data);
    else if (result.error.kind !== "cancelled" && !busy) setError(result.error);
  };

  // A file dropped on the window while the page is open is imported, one at a time, and
  // never behind the consent notice, where the import line is hidden.
  const onDrop = useRef<(path: string) => void>(() => {});
  useEffect(() => {
    onDrop.current = (path) => {
      if (!askConsent) void importSource({ kind: "file", value: path });
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

  const saveKey = async (key: string) => {
    const result = await commands.setMeetingTranscriptionKey(key);
    if (result.status === "ok") setKeys(result.data);
    else setError(result.error);
  };

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6">
      <SettingsGroup title={t("meeting.title")}>
        <RecoverBanner
          orphans={orphans}
          busy={recovering !== null}
          onRecover={(id) => void recover(id)}
          t={t}
        />
        {askConsent ? (
          <ConsentNotice
            onAccept={async () => {
              setAskConsent(false);
              await commands.acceptMeetingConsent();
              await start(pendingDraft);
            }}
            onCancel={() => setAskConsent(false)}
          />
        ) : (
          <RecorderPanel
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
                    void open(live.summary.id);
                  }
                });
            }}
          />
        )}
        {!askConsent && (
          <ImportDialog
            importing={importing}
            stage={importStage}
            onImport={(source) => void importSource(source)}
            onCancel={() => void commands.cancelMeetingImport()}
            onGranola={setGranolaPath}
          />
        )}
        {granolaPath !== null && (
          <GranolaImport
            key={granolaPath}
            path={granolaPath}
            onImported={() => void refresh()}
            onClose={() => setGranolaPath(null)}
          />
        )}
        {live && (
          <div className="p-4">
            <AnnotationsField
              id={live.summary.id}
              initial={live.annotations}
              label={t("meeting.view.annotations")}
            />
          </div>
        )}
        {keys && !keys.elevenlabs && (
          <KeyField onSave={(key) => void saveKey(key)} />
        )}
        {error !== null && (
          <p role="alert" className="p-4 text-sm text-red-500">
            {t(errorKey(error), { detail: errorDetail(error) })}
          </p>
        )}
      </SettingsGroup>
      <SettingsGroup title={t("meeting.list.title")}>
        <MeetingList
          sessions={sessions}
          selectedId={selected?.summary.id ?? null}
          onOpen={(id) => void open(id)}
        />
      </SettingsGroup>
      {selected && (
        <SettingsGroup>
          <MeetingView
            key={selected.summary.id}
            detail={selected}
            templates={templates}
            recording={status.id === selected.summary.id}
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
            onNamesChanged={() => void open(selected.summary.id)}
          />
          <DeleteMeeting
            key={`delete-${selected.summary.id}`}
            disabled={
              status.id === selected.summary.id ||
              transcribing === selected.summary.id ||
              generating === selected.summary.id
            }
            onDelete={() => void remove(selected.summary.id)}
          />
        </SettingsGroup>
      )}
      <NotesContextSettings />
      <TemplatesSettings catalog={catalog} onChange={setCatalog} />
    </div>
  );
}
