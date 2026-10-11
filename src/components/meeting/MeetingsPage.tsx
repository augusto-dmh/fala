import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  events,
  type MeetingDetail,
  type MeetingError,
  type MeetingKeys,
  type MeetingMode,
  type MeetingStatus,
  type MeetingSummary,
  type MeetingTemplate,
} from "@/bindings";
import { copyToClipboard } from "../settings/history/clipboard";
import { SettingsGroup } from "../ui/SettingsGroup";
import { AnnotationsField } from "./AnnotationsField";
import { ConsentNotice } from "./ConsentNotice";
import { KeyField } from "./KeyField";
import { MeetingList } from "./MeetingList";
import { MeetingView } from "./MeetingView";
import { RecorderPanel } from "./RecorderPanel";
import { errorKey, takeConsentRequest } from "./meetingModel";

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
};

/** The Meetings page: record, the list of sessions and the open one. */
export function MeetingsPage() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<MeetingStatus>(IDLE);
  const [sessions, setSessions] = useState<MeetingSummary[]>([]);
  const [selected, setSelected] = useState<MeetingDetail | null>(null);
  const [live, setLive] = useState<MeetingDetail | null>(null);
  const [templates, setTemplates] = useState<MeetingTemplate[]>([]);
  const [keys, setKeys] = useState<MeetingKeys | null>(null);
  const [mode, setMode] = useState<MeetingMode>("meeting");
  const [title, setTitle] = useState("");
  const [askConsent, setAskConsent] = useState(takeConsentRequest);
  const [pendingDraft, setPendingDraft] = useState<string | null>(null);
  const [transcribing, setTranscribing] = useState<string | null>(null);
  const [generating, setGenerating] = useState<string | null>(null);
  const [error, setError] = useState<MeetingError | string | null>(null);

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

  useEffect(() => {
    void commands
      .meetingStatus()
      .then((r) => r.status === "ok" && setStatus(r.data));
    void commands
      .meetingTemplates()
      .then((r) => r.status === "ok" && setTemplates(r.data));
    void commands
      .meetingKeys()
      .then((r) => r.status === "ok" && setKeys(r.data));
    void refresh();
    const unlisteners = [
      events.meetingStatus.listen((event) => setStatus(event.payload)),
      events.meetingProgress.listen((event) => {
        const { id, stage } = event.payload;
        if (stage.stage === "transcribing") setTranscribing(id);
        if (stage.stage === "done" || stage.stage === "failed") {
          setTranscribing((current) => (current === id ? null : current));
          void refresh();
          setSelected((current) => {
            if (current?.summary.id === id) void open(id);
            return current;
          });
        }
      }),
      // The page is open: the app does not leave a request for the next mount.
      listen("meeting-consent-required", () => setAskConsent(true)),
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

  const generate = async (id: string, templateId: string) => {
    setError(null);
    setGenerating(id);
    const result = await commands.generateMeetingNotes(id, templateId);
    setGenerating(null);
    if (result.status === "error") setError(result.error);
    await open(id);
    await refresh();
  };

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

  const saveKey = async (key: string) => {
    const result = await commands.setMeetingTranscriptionKey(key);
    if (result.status === "ok") setKeys(result.data);
    else setError(result.error);
  };

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6">
      <SettingsGroup title={t("meeting.title")}>
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
            onModeChange={setMode}
            onTitleChange={setTitle}
            onStart={() => void start(null)}
            onNewDraft={() => void newDraft()}
            onPause={() => void act(commands.pauseMeeting)}
            onResume={() => void act(commands.resumeMeeting)}
            onStop={() => void act(commands.stopMeeting)}
            onExtend={() => void act(commands.extendMeetingCap)}
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
            {t(errorKey(error))}
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
            generating={generating === selected.summary.id}
            onRecordDraft={() => void start(selected.summary.id)}
            onTranscribe={() => void transcribe(selected.summary.id)}
            onCancelTranscription={() =>
              void commands.cancelMeetingTranscription()
            }
            onGenerate={(templateId) =>
              void generate(selected.summary.id, templateId)
            }
            onCopy={() => void copy(selected.summary.id)}
          />
        </SettingsGroup>
      )}
    </div>
  );
}
