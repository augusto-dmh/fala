import { listen } from "@tauri-apps/api/event";
import React, {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import "./RecordingOverlay.css";
import "./Pill.css";
import "./Notice.css";
import "./IdlePill.css";
import { Notice } from "./Notice";
import { noticeTextKey, type OverlayNotice } from "./noticeModel";
import { Pill } from "./Pill";
import {
  CallOutputCard,
  ConsentCard,
  MeetingDictationCard,
  MeetingPill,
} from "./MeetingPill";
import { IdlePill } from "./IdlePill";
import { PasteFailedPill } from "./PasteFailedPill";
import { createPillDrag } from "./pillDrag";
import { useOsType } from "@/hooks/useOsType";
import {
  SLOW_LOAD_MS,
  idleKeys,
  isHoldToTalk,
  nextModelLoadStart,
  pasteFailedHint,
  pasteFailedTimeout,
  pasteLastShortcut,
  showsConsentChip,
  showsDoubleTapHint,
  showsModelLoading,
  toPillMode,
} from "./pillModel";
import { commands, events } from "@/bindings";
import type {
  StreamPhase,
  StreamPhaseEvent,
  StreamTextEvent,
  StreamWorkKind,
} from "@/bindings";
import i18n, { syncLanguageFromSettings } from "@/i18n";
import type { ModelStateEvent } from "@/lib/types/events";
import { getLanguageDirection } from "@/lib/utils/rtl";

type OverlayState =
  | "recording"
  | "streaming"
  | "transcribing"
  | "processing"
  | "meeting"
  | "meeting_paused"
  | "paste_failed"
  | "idle";

// Number of reactive bars in the waveform (the simple, smoothed style shared by
// every overlay form). Mic levels arrive as 16 FFT buckets; we take the first N.
const WAVE_BARS = 9;

const RecordingOverlay: React.FC = () => {
  const { t } = useTranslation();
  const [isVisible, setIsVisible] = useState(false);
  const [state, setState] = useState<OverlayState>("recording");
  // `Stream::play()` returning does not mean hardware callbacks are flowing.
  // Stay visually in an arming state until the backend processes the first
  // actual microphone sample chunk.
  const [captureReady, setCaptureReady] = useState(false);
  // The recording is close to the session limit (19 of 20 minutes): the dot
  // turns amber until the overlay hides or a new session shows.
  const [limitWarning, setLimitWarning] = useState(false);
  // A notice from the backend (`overlay-notice`): a failed gesture or a muted microphone
  // alone, or the microphone's name stacked over the recording pill.
  const [notice, setNotice] = useState<OverlayNotice | null>(null);
  // Recording starts while the model loads in the background. When the in-flight load started
  // (ms), or null: after the key is released the wait is the load, not the transcription, and a
  // cold start would otherwise look like a stuck pill.
  const [loadStart, setLoadStart] = useState<number | null>(null);
  // Re-render once the load crosses SLOW_LOAD_MS during the recording.
  const [now, setNow] = useState(() => Date.now());
  const [levels, setLevels] = useState<number[]>(Array(WAVE_BARS).fill(0));
  const [streamText, setStreamText] = useState<StreamTextEvent>({
    committed: "",
    tentative: "",
  });
  // Push-to-talk style activation: the minimal pill turns red while recording.
  const [holdToTalk, setHoldToTalk] = useState(false);
  const [phase, setPhase] = useState<StreamPhase>("listening");
  const [workKind, setWorkKind] = useState<StreamWorkKind>("transcribing");
  const [elapsed, setElapsed] = useState(0);
  // The meeting indicator: recorded time and the silent-channel warning, from `MeetingStatus`.
  const [meetingMs, setMeetingMs] = useState(0);
  const [meetingMuted, setMeetingMuted] = useState(false);
  // The output the call plays on while the system channel records zeros, if any.
  const [callOutput, setCallOutput] = useState<string | null>(null);
  // The meeting's mic was never heard: its name ("" when unknown), or `null`.
  const [mutedMic, setMutedMic] = useState<string | null>(null);
  // A dictation zeroes the meeting's mic now: a card over the recording pill says so.
  const [meetingDictation, setMeetingDictation] = useState(false);
  // The pill at rest: the cursor is over it, and the shortcut it shows on hover.
  const [idleHover, setIdleHover] = useState(false);
  const [idleKeyList, setIdleKeyList] = useState<string[]>([]);
  const [idleDoubleTap, setIdleDoubleTap] = useState(false);
  // After a paste that failed: the pointer is over the pill (its timer stops), the click copied
  // the text, and the shortcut that pastes the last dictation, if any.
  const [pasteHover, setPasteHover] = useState(false);
  const [pasteCopied, setPasteCopied] = useState(false);
  const [pasteLastKeys, setPasteLastKeys] = useState<string | null>(null);
  // The show the pill belongs to (`paste-failed`): a second failure restarts the timer, and the
  // end names the pill it ends.
  const [pasteGeneration, setPasteGeneration] = useState<number | null>(null);
  // Bumped on each new streaming session so the Live card remounts fresh (replays
  // the pop-in, and never animates in from the previous panel's open size).
  const [session, setSession] = useState(0);
  // Overlay placement (top vs bottom of the screen). The Live panel grows downward
  // from a top overlay (oldest line under the pill) and upward from a bottom one.
  const [position, setPosition] = useState<"top" | "bottom">("bottom");
  // True once live text overflows the cap. A top overlay fades its top edge only
  // while overflowing, so the resting first line stays crisp flush under the pill.
  const [overflowing, setOverflowing] = useState(false);

  const smoothedLevelsRef = useRef<number[]>(Array(16).fill(0));
  // Dragging the idle or the meeting pill slides it along its edge. Windows only: elsewhere a
  // small slip of the hand must not swallow the click.
  const osType = useOsType();
  const drag = useMemo(
    () =>
      osType === "windows"
        ? createPillDrag(() => void commands.startPillDrag())
        : {},
    [osType],
  );
  // Live-text scroll-back: the text region "sticks" to the newest line while the
  // user is at the bottom; if they scroll up to read history, auto-follow pauses
  // until they scroll back down.
  const capRef = useRef<HTMLDivElement>(null);
  const pinnedRef = useRef(true);
  const direction = getLanguageDirection(i18n.language);

  useEffect(() => {
    const setupEventListeners = async () => {
      // The window opened for a notice: what it shows comes with `overlay-notice`. The
      // overlay may not have shown since launch, so read the language and the placement.
      const showForNotice = async () => {
        setIsVisible(true);
        await syncLanguageFromSettings();
        try {
          const settings = await commands.getAppSettings();
          if (settings.status === "ok") {
            setPosition(
              settings.data.overlay_position === "top" ? "top" : "bottom",
            );
          }
        } catch {
          // Keep the previous/default placement if settings can't be read.
        }
      };

      // The pill at rest, between sessions: reads the shortcut it shows on hover.
      const showIdle = async () => {
        setNotice(null);
        setIdleHover(false);
        setState("idle");
        setIsVisible(true);
        await syncLanguageFromSettings();
        try {
          const settings = await commands.getAppSettings();
          if (settings.status === "ok") {
            setPosition(
              settings.data.overlay_position === "top" ? "top" : "bottom",
            );
            setIdleKeyList(
              idleKeys(settings.data.bindings?.transcribe?.current_binding),
            );
            setIdleDoubleTap(
              showsDoubleTapHint(settings.data.shortcut_activation),
            );
          }
        } catch {
          // Keep the previous shortcut and placement if settings can't be read.
        }
      };

      const unlistenShow = await listen("show-overlay", async (event) => {
        if (event.payload === "notice") {
          void showForNotice();
          return;
        }
        if (event.payload === "idle") {
          void showIdle();
          return;
        }
        const overlayState = event.payload as OverlayState;
        // Reset synchronously before settings I/O. A fast microphone can emit
        // recording-ready while the awaits below are in flight; resetting after
        // them would overwrite that event and leave the overlay stuck arming.
        if (overlayState === "recording" || overlayState === "streaming") {
          setCaptureReady(false);
          setLimitWarning(false);
          setNotice(null);
          smoothedLevelsRef.current = Array(16).fill(0);
          setLevels(Array(WAVE_BARS).fill(0));
          setStreamText({ committed: "", tentative: "" });
        }
        if (overlayState === "paste_failed") {
          setNotice(null);
          setPasteCopied(false);
        }

        await syncLanguageFromSettings();
        // The Live panel flows downward from a top overlay and upward from a
        // bottom one; read the placement so the layout can flip to match.
        try {
          const settings = await commands.getAppSettings();
          if (settings.status === "ok") {
            setPosition(
              settings.data.overlay_position === "top" ? "top" : "bottom",
            );
            setHoldToTalk(isHoldToTalk(settings.data.shortcut_activation));
            setPasteLastKeys(
              pasteLastShortcut(
                settings.data.bindings?.paste_last?.current_binding,
              ),
            );
          }
        } catch {
          // Keep the previous/default placement if settings can't be read.
        }
        setState(overlayState);
        if (overlayState === "streaming") {
          setPhase("listening");
          setWorkKind("transcribing");
          setElapsed(0);
          setSession((s) => s + 1); // remount the card fresh for this session
        }
        setIsVisible(true);
      });

      const unlistenHide = await listen("hide-overlay", () => {
        setIsVisible(false);
        setIdleHover(false);
        setPasteHover(false);
        setCaptureReady(false);
        setLimitWarning(false);
        setNotice(null);
        // The next meeting starts without the last one's mic card.
        setMutedMic(null);
      });

      const unlistenNotice = await listen<OverlayNotice>(
        "overlay-notice",
        (event) => {
          setNotice(event.payload);
        },
      );

      const unlistenPasteFailed = await listen<{ generation: number }>(
        "paste-failed",
        (event) => {
          setPasteCopied(false);
          setPasteGeneration(event.payload.generation);
        },
      );

      const unlistenLimit = await listen("recording-limit-warning", () => {
        setLimitWarning(true);
      });

      const unlistenReady = await listen("recording-ready", () => {
        setElapsed(0);
        setCaptureReady(true);
      });

      const unlistenLevel = await listen<number[]>("mic-level", (event) => {
        const newLevels = event.payload as number[];
        // Exponential smoothing across the 16 buckets, then take the first N
        // bars for the shared waveform.
        const smoothed = smoothedLevelsRef.current.map((prev, i) => {
          const target = newLevels[i] || 0;
          return prev * 0.7 + target * 0.3;
        });
        smoothedLevelsRef.current = smoothed;
        setLevels(smoothed.slice(0, WAVE_BARS));
      });

      const unlistenMeeting = await events.meetingStatus.listen((event) => {
        setMeetingMs(event.payload.recorded_ms);
        // Amber after 2 min of silence or ~3 s of digital zero in a channel.
        const muted = event.payload.muted_mic || event.payload.muted_system;
        const zero = event.payload.zero_mic || event.payload.zero_system;
        setMeetingMuted(muted || zero);
        setCallOutput(event.payload.call_output);
        setMutedMic(
          event.payload.mic_unheard ? (event.payload.mic_input ?? "") : null,
        );
        setMeetingDictation(event.payload.dictating);
      });

      const unlistenStream = await events.streamTextEvent.listen((event) => {
        setStreamText(event.payload);
      });

      const unlistenPhase = await events.streamPhaseEvent.listen((event) => {
        const payload: StreamPhaseEvent = event.payload;
        setPhase(payload.phase);
        if (payload.kind) setWorkKind(payload.kind);
      });

      const unlistenModel = await listen<ModelStateEvent>(
        "model-state-changed",
        (event) => {
          const at = Date.now();
          setNow(at);
          setLoadStart((current) =>
            nextModelLoadStart(current, event.payload.event_type, at),
          );
        },
      );

      // Listening now: an idle pill shown before this point is shown again.
      void commands.pillOverlayReady();

      return () => {
        unlistenModel();
        unlistenShow();
        unlistenHide();
        unlistenReady();
        unlistenLimit();
        unlistenNotice();
        unlistenPasteFailed();
        unlistenLevel();
        unlistenStream();
        unlistenMeeting();
        unlistenPhase();
      };
    };

    setupEventListeners();
  }, []);

  // Elapsed capture timer starts only once microphone samples are flowing.
  useEffect(() => {
    if (state !== "streaming" || !isVisible || !captureReady) return;
    const id = setInterval(() => setElapsed((e) => e + 1), 1000);
    return () => clearInterval(id);
  }, [state, isVisible, captureReady]);

  // A notice clears itself after its duration; an alone notice's window is hidden by the
  // backend at the same time.
  useEffect(() => {
    if (!notice) return;
    const id = setTimeout(
      () => setNotice((current) => (current === notice ? null : current)),
      notice.duration_ms,
    );
    return () => clearTimeout(id);
  }, [notice]);

  // The card unmounts without a mouseleave when another state replaces it.
  useEffect(() => {
    if (state !== "paste_failed") setPasteHover(false);
  }, [state]);

  // The paste-failed pill ends 5 s after the pointer leaves it; it stays while the pointer is on it.
  useEffect(() => {
    if (state !== "paste_failed" || !isVisible || pasteGeneration === null)
      return;
    const wait = pasteFailedTimeout(pasteHover);
    if (wait === null) return;
    const id = setTimeout(
      () => void commands.endPasteFailed(pasteGeneration),
      wait,
    );
    return () => clearTimeout(id);
  }, [state, isVisible, pasteHover, pasteGeneration]);

  // While a load is in flight, wake up when it becomes slow so the recording pill can show it.
  useEffect(() => {
    if (loadStart === null) return;
    const wait = loadStart + SLOW_LOAD_MS - Date.now();
    if (wait <= 0) {
      setNow(Date.now());
      return;
    }
    const id = setTimeout(() => setNow(Date.now()), wait);
    return () => clearTimeout(id);
  }, [loadStart]);

  // Stick to the bottom as text streams in — but only while pinned, so a user who
  // has scrolled up to read history isn't yanked back down by the next chunk.
  useLayoutEffect(() => {
    const el = capRef.current;
    if (!el) return;
    // Fade the top edge only once text actually overflows the cap.
    setOverflowing(el.scrollHeight > el.clientHeight + 1);
    if (pinnedRef.current) el.scrollTop = el.scrollHeight;
  }, [streamText]);

  // Each fresh streaming session starts pinned to the bottom, fade cleared.
  useEffect(() => {
    pinnedRef.current = true;
    setOverflowing(false);
  }, [session]);

  if (!isVisible) return null;

  // Re-pin when the user is within ~a line of the bottom; unpin otherwise.
  const handleStreamScroll = () => {
    const el = capRef.current;
    if (!el) return;
    pinnedRef.current = el.scrollHeight - el.scrollTop - el.clientHeight <= 16;
  };

  const noticeCard = notice && (
    <Notice
      kind={notice.kind}
      text={t(noticeTextKey(notice), { device: notice.device ?? "" })}
      chooseLabel={t("overlay.notice.chooseMic")}
      resolveLabel={t("overlay.notice.resolve")}
      onChoose={() => commands.openMicrophoneSettings()}
      onResolve={() => commands.openMicrophoneTroubleshooting()}
    />
  );

  if (notice?.alone) {
    return (
      <div dir={direction} className={`ov-stage ${position}`}>
        {noticeCard}
      </div>
    );
  }

  if (state === "paste_failed") {
    const hint = pasteFailedHint(pasteCopied, pasteLastKeys);
    return (
      <div dir={direction} className={`ov-stage ${position}`}>
        <PasteFailedPill
          onHover={setPasteHover}
          title={t("overlay.pasteFailed.title")}
          hint={t(hint.key, { keys: hint.keys })}
          label={t("overlay.pasteFailed.label")}
          onCopy={() =>
            void commands
              .copyFailedPaste()
              .then((copied) => copied && setPasteCopied(true))
          }
        />
      </div>
    );
  }

  if (state === "idle") {
    const hover = (on: boolean) => {
      setIdleHover(on);
      void commands.setIdlePillHover(on);
    };
    return (
      <div
        dir={direction}
        className={`ov-stage ${position}`}
        {...drag}
        onMouseEnter={() => hover(true)}
        onMouseLeave={() => hover(false)}
        onContextMenu={(event) => {
          // The pill menu is native (`open_pill_menu`), not the webview's.
          event.preventDefault();
          void commands.openPillMenu();
        }}
      >
        <IdlePill
          hover={idleHover}
          keys={idleKeyList}
          dictateLabel={t("overlay.idle.dictate")}
          doubleTapLabel={idleDoubleTap ? t("overlay.idle.doubleTap") : null}
          label={t("overlay.idle.label")}
          onOpen={() => void commands.openHomeFromPill()}
          newMeetingLabel={t("overlay.idle.newMeeting")}
          onNewMeeting={() => void commands.startMeetingFromPill()}
        />
      </div>
    );
  }

  const transcribingLabel =
    loadStart !== null ? t("overlay.loadingModel") : t("overlay.transcribing");

  const fmtTime = (s: number) =>
    `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;

  // ---- Shared building blocks (one visual language for every overlay form) ----
  const waveform = (
    <div className={`swave ${captureReady ? "ready" : "arming"}`}>
      {levels.map((v, i) => (
        <i
          key={i}
          style={{
            height: `${Math.max(3, Math.min(18, 3 + Math.pow(v, 0.7) * 15))}px`,
          }}
        />
      ))}
    </div>
  );

  const cancelBtn = (
    <button
      className="sx"
      aria-label="cancel"
      onClick={() => commands.cancelOperation()}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path
          d="M4 4 L12 12 M12 4 L4 12"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
        />
      </svg>
    </button>
  );

  // dot (left) | waveform (center) | timer + cancel (right) — same structure for
  // pill & panel, so the Live morph is a pure width change.
  const listeningRow = (showTimer: boolean, showCancel: boolean) => (
    <div className="sbase">
      <div className="sbase-l">
        <span
          className={`sdot ${captureReady ? "ready" : "arming"}${limitWarning ? " limit" : ""}`}
        />
      </div>
      {waveform}
      <div className="sbase-r">
        {showTimer && <span className="stimer">{fmtTime(elapsed)}</span>}
        {showCancel && cancelBtn}
      </div>
    </div>
  );

  // spinner (left) | label (center) | cancel (right) — same 3-zone grid as the
  // listening row, so the label is centered.
  const workingRow = (label: string, showCancel: boolean) => (
    <div className="sbase">
      <div className="sbase-l">
        <span className="sspinner" />
      </div>
      <span className="swork-label">{label}</span>
      <div className="sbase-r">{showCancel && cancelBtn}</div>
    </div>
  );

  // ---- Live overlay: a pill that sculpts open into a panel ----
  if (state === "streaming") {
    const hasText =
      streamText.committed.length > 0 || streamText.tentative.length > 0;
    const working = phase === "working";
    // Keep the panel open whenever there's text — even while finalizing — so the
    // transcript stays put under a working spinner instead of collapsing and
    // squishing the text mid-stream. Only fall back to the small working pill
    // when there was no text to preserve.
    const open = hasText;
    const collapsed = working && !hasText;

    return (
      <div dir={direction} className={`ov-stage ${position}`}>
        <div
          key={session}
          className={`scard ${open ? "open" : ""} ${collapsed ? "working" : ""} ${
            isVisible ? "" : "leaving"
          }`}
        >
          <div className="stext">
            <div className="stext-clip">
              <div
                className={`stext-cap ${overflowing ? "overflowing" : ""}`}
                ref={capRef}
                onScroll={handleStreamScroll}
              >
                <p>
                  <span className="committed">
                    {streamText.committed ? streamText.committed + " " : ""}
                  </span>
                  <span className="tentative">{streamText.tentative}</span>
                  {/* Drop the blinking caret once finalizing — it's no longer
                      capturing, and a static spinner conveys the work. */}
                  {!working && <span className="scaret" />}
                </p>
              </div>
            </div>
          </div>
          {working
            ? workingRow(
                workKind === "polishing"
                  ? t("overlay.processing")
                  : transcribingLabel,
                true,
              )
            : listeningRow(open, true)}
        </div>
      </div>
    );
  }

  // ---- Minimal overlay: the phase 1 pill. Recording draws ten bars that follow
  // the mic (red while the key is held); transcribing and processing share one
  // still, pulsing state. No icon and no text; cancelling stays on Esc.
  if (state === "meeting" || state === "meeting_paused") {
    const meetingPill = (
      <MeetingPill
        paused={state === "meeting_paused"}
        recordedMs={meetingMs}
        muted={meetingMuted}
        onOpen={() => void commands.openMeetingFromPill()}
        label={
          state === "meeting"
            ? t("overlay.meeting")
            : t("overlay.meetingPaused")
        }
      />
    );
    return (
      <div
        dir={direction}
        className={`ov-stage ${position} ov-fade ${isVisible ? "show" : ""}`}
        {...drag}
      >
        {callOutput ? (
          <div className="ov-stack">
            <CallOutputCard
              text={t("overlay.callElsewhere", { device: callOutput })}
              switchLabel={t("overlay.recordFrom")}
              onSwitch={() => void commands.switchMeetingOutput(callOutput)}
            />
            {meetingPill}
          </div>
        ) : mutedMic !== null ? (
          <div className="ov-stack">
            <CallOutputCard
              kind="muted_mic"
              text={
                mutedMic
                  ? t("overlay.mutedMic", { device: mutedMic })
                  : t("overlay.mutedMicUnnamed")
              }
              switchLabel={t("overlay.switchMic")}
              onSwitch={() => void commands.openMeetingMicPicker()}
            />
            {meetingPill}
          </div>
        ) : showsConsentChip(meetingMs) ? (
          <div className="ov-stack">
            <ConsentCard text={t("overlay.meetingConsent")} />
            {meetingPill}
          </div>
        ) : (
          meetingPill
        )}
      </div>
    );
  }

  const pillMode = toPillMode(state) ?? "recording";
  const pillLoading = showsModelLoading(pillMode, loadStart, now);
  const loadingLabel =
    pillMode === "recording"
      ? t("overlay.recordingLoadingModel")
      : t("overlay.loadingModel");
  const pillLabel = pillLoading
    ? loadingLabel
    : pillMode === "recording"
      ? t("overlay.recording")
      : t("overlay.processing");

  const pill = (
    <Pill
      mode={pillMode}
      holdToTalk={holdToTalk}
      limit={limitWarning}
      modelLoading={pillLoading}
      ready={captureReady}
      levels={levels}
      label={pillLabel}
    />
  );

  return (
    <div
      dir={direction}
      className={`ov-stage ${position} ov-fade ${isVisible ? "show" : ""}`}
    >
      {noticeCard ? (
        <div className="ov-stack">
          {noticeCard}
          {pill}
        </div>
      ) : meetingDictation && pillMode === "recording" ? (
        <div className="ov-stack">
          <MeetingDictationCard text={t("overlay.meetingDictation")} />
          {pill}
        </div>
      ) : (
        pill
      )}
    </div>
  );
};

export default RecordingOverlay;
