import React, { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { readFile } from "@tauri-apps/plugin-fs";
import {
  Check,
  Copy,
  FolderOpen,
  Redo2,
  RotateCcw,
  Search,
  Star,
  Trash2,
  Undo2,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  commands,
  events,
  type HistoryEntry,
  type HistoryUpdatePayload,
} from "@/bindings";
import { useOsType } from "@/hooks/useOsType";
import { formatDate } from "@/utils/dateFormat";
import { AudioPlayer, AudioPlayerGroup } from "../../ui/AudioPlayer";
import { Button } from "../../ui/Button";
import { copyToClipboard } from "./clipboard";
import {
  appName,
  editAction,
  groupByDay,
  recoverEntry,
  rowKind,
  SEARCH_DEBOUNCE_MS,
  searchQuery,
  shownText,
  switchText,
  timeLabel,
  type DayGroup,
  type EditAction,
  type SwitchTextOutcome,
} from "./historyModel";

const IconButton: React.FC<{
  onClick: () => void;
  title: string;
  disabled?: boolean;
  active?: boolean;
  children: React.ReactNode;
}> = ({ onClick, title, disabled, active, children }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    className={`p-1.5 rounded-md flex items-center justify-center transition-colors cursor-pointer disabled:cursor-not-allowed disabled:text-text/20 ${
      active
        ? "text-logo-primary hover:text-logo-primary/80"
        : "text-text/50 hover:text-logo-primary"
    }`}
    title={title}
  >
    {children}
  </button>
);

const PAGE_SIZE = 30;

interface OpenRecordingsButtonProps {
  onClick: () => void;
  label: string;
}

const OpenRecordingsButton: React.FC<OpenRecordingsButtonProps> = ({
  onClick,
  label,
}) => (
  <Button
    onClick={onClick}
    variant="secondary"
    size="sm"
    className="flex items-center gap-2 shrink-0"
    title={label}
  >
    <FolderOpen className="w-4 h-4" />
    <span>{label}</span>
  </Button>
);

const replaceIn = (list: HistoryEntry[], entry: HistoryEntry) =>
  list.map((e) => (e.id === entry.id ? entry : e));

export const HistorySettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  const osType = useOsType();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasMore, setHasMore] = useState(true);
  const [query, setQuery] = useState("");
  // `null` while the field is blank: the paged list shows.
  const [results, setResults] = useState<HistoryEntry[] | null>(null);
  const sentinelRef = useRef<HTMLDivElement>(null);
  const entriesRef = useRef<HistoryEntry[]>([]);
  const loadingRef = useRef(false);

  // Keep ref in sync for use in IntersectionObserver callback
  useEffect(() => {
    entriesRef.current = entries;
  }, [entries]);

  const loadPage = useCallback(async (cursor?: number) => {
    const isFirstPage = cursor === undefined;
    if (!isFirstPage && loadingRef.current) return;
    loadingRef.current = true;

    if (isFirstPage) setLoading(true);

    try {
      const result = await commands.getHistoryEntries(
        cursor ?? null,
        PAGE_SIZE,
      );
      if (result.status === "ok") {
        const { entries: newEntries, has_more } = result.data;
        setEntries((prev) =>
          isFirstPage ? newEntries : [...prev, ...newEntries],
        );
        setHasMore(has_more);
      }
    } catch (error) {
      console.error("Failed to load history entries:", error);
    } finally {
      setLoading(false);
      loadingRef.current = false;
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadPage();
  }, [loadPage]);

  // Search 200 ms after the last keystroke; a blank field returns to the list.
  useEffect(() => {
    const q = searchQuery(query);
    if (q === null) {
      setResults(null);
      return;
    }
    let stale = false;
    const timer = setTimeout(async () => {
      try {
        const result = await commands.historySearch(q);
        if (stale) return;
        if (result.status === "ok") {
          setResults(result.data);
        } else {
          console.error("Failed to search the history:", result.error);
          toast.error(t("settings.history.searchError"));
        }
      } catch (error) {
        console.error("Failed to search the history:", error);
      }
    }, SEARCH_DEBOUNCE_MS);
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  }, [query, t]);

  // Infinite scroll via IntersectionObserver
  useEffect(() => {
    if (loading || results !== null) return;

    const sentinel = sentinelRef.current;
    if (!sentinel || !hasMore) return;

    const observer = new IntersectionObserver(
      (observerEntries) => {
        const first = observerEntries[0];
        if (first.isIntersecting) {
          const lastEntry = entriesRef.current[entriesRef.current.length - 1];
          if (lastEntry) {
            loadPage(lastEntry.id);
          }
        }
      },
      { threshold: 0 },
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [loading, hasMore, loadPage, results]);

  const updateEntry = (entry: HistoryEntry) => {
    setEntries((prev) => replaceIn(prev, entry));
    setResults((prev) => (prev ? replaceIn(prev, entry) : prev));
  };

  // Listen for new entries added from the transcription pipeline
  useEffect(() => {
    const unlisten = events.historyUpdatePayload.listen((event) => {
      const payload: HistoryUpdatePayload = event.payload;
      if (payload.action === "added") {
        setEntries((prev) => [payload.entry, ...prev]);
      } else if (payload.action === "updated") {
        setEntries((prev) => replaceIn(prev, payload.entry));
        setResults((prev) => (prev ? replaceIn(prev, payload.entry) : prev));
      }
      // "deleted" and "toggled" are handled by optimistic updates only,
      // so we intentionally ignore them here to avoid double-mutation.
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const toggleSaved = async (id: number) => {
    const flip = (list: HistoryEntry[]) =>
      list.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e));
    // Optimistic update
    setEntries(flip);
    setResults((prev) => (prev ? flip(prev) : prev));
    try {
      const result = await commands.toggleHistoryEntrySaved(id);
      if (result.status !== "ok") {
        // Revert on failure
        setEntries(flip);
        setResults((prev) => (prev ? flip(prev) : prev));
      }
    } catch (error) {
      console.error("Failed to toggle saved status:", error);
      // Revert on failure
      setEntries(flip);
      setResults((prev) => (prev ? flip(prev) : prev));
    }
  };

  const getAudioUrl = useCallback(
    async (fileName: string) => {
      try {
        const result = await commands.getAudioFilePath(fileName);
        if (result.status === "ok") {
          if (osType === "linux") {
            const fileData = await readFile(result.data);
            const blob = new Blob([fileData], { type: "audio/wav" });
            return URL.createObjectURL(blob);
          }
          return convertFileSrc(result.data, "asset");
        }
        return null;
      } catch (error) {
        console.error("Failed to get audio file path:", error);
        return null;
      }
    },
    [osType],
  );

  const deleteAudioEntry = async (id: number) => {
    // Optimistically remove
    setEntries((prev) => prev.filter((e) => e.id !== id));
    setResults((prev) => (prev ? prev.filter((e) => e.id !== id) : prev));
    try {
      const result = await commands.deleteHistoryEntry(id);
      if (result.status !== "ok") {
        // Reload on failure
        loadPage();
      }
    } catch (error) {
      console.error("Failed to delete entry:", error);
      loadPage();
    }
  };

  const applyOutcome = (outcome: SwitchTextOutcome) => {
    if (outcome.entry) {
      updateEntry(outcome.entry);
    }
    outcome.success
      ? toast.success(t(outcome.toastKey))
      : toast.error(t(outcome.toastKey));
  };

  const switchEntryText = async (entry: HistoryEntry, action: EditAction) => {
    applyOutcome(
      await switchText(entry, action, {
        undo: commands.undoHistoryEntryEdit,
        redo: commands.redoHistoryEntryEdit,
        copy: copyToClipboard,
      }),
    );
  };

  const recoverHistoryEntry = async (entry: HistoryEntry) => {
    applyOutcome(
      await recoverEntry(entry, {
        recover: commands.recoverHistoryEntry,
        copy: copyToClipboard,
      }),
    );
  };

  const retryHistoryEntry = async (id: number) => {
    const result = await commands.retryHistoryEntryTranscription(id);
    if (result.status !== "ok") {
      throw new Error(String(result.error));
    }
  };

  const openRecordingsFolder = async () => {
    try {
      const result = await commands.openRecordingsFolder();
      if (result.status !== "ok") {
        throw new Error(String(result.error));
      }
    } catch (error) {
      console.error("Failed to open recordings folder:", error);
    }
  };

  const groupLabel = (group: DayGroup) =>
    group.day === "date"
      ? formatDate(String(group.timestamp), i18n.language)
      : t(`settings.history.${group.day}`);

  const shown = results ?? entries;
  let content: React.ReactNode;

  if (loading && results === null) {
    content = (
      <div className="px-4 py-3 text-center text-text/60">
        {t("settings.history.loading")}
      </div>
    );
  } else if (shown.length === 0) {
    content = (
      <div className="px-4 py-3 text-center text-text/60">
        {results === null
          ? t("settings.history.empty")
          : t("settings.history.searchNoResults")}
      </div>
    );
  } else {
    content = (
      <>
        <AudioPlayerGroup>
          <div className="space-y-6">
            {groupByDay(shown, new Date()).map((group) => (
              <section key={group.key} className="space-y-2">
                <h3 className="px-4 text-xs font-medium text-mid-gray uppercase tracking-wide">
                  {groupLabel(group)}
                </h3>
                <div className="bg-background border border-mid-gray/20 rounded-lg divide-y divide-mid-gray/20">
                  {group.entries.map((entry) => (
                    <HistoryEntryComponent
                      key={entry.id}
                      entry={entry}
                      onToggleSaved={() => toggleSaved(entry.id)}
                      onCopyText={() => copyToClipboard(shownText(entry))}
                      onSwitchText={(action) => switchEntryText(entry, action)}
                      onRecover={() => recoverHistoryEntry(entry)}
                      getAudioUrl={getAudioUrl}
                      deleteAudio={deleteAudioEntry}
                      retryTranscription={retryHistoryEntry}
                    />
                  ))}
                </div>
              </section>
            ))}
          </div>
        </AudioPlayerGroup>
        {/* Sentinel for infinite scroll */}
        {results === null && <div ref={sentinelRef} className="h-1" />}
      </>
    );
  }

  return (
    <div className="max-w-3xl w-full mx-auto space-y-4">
      <div className="px-4 flex items-center gap-3">
        <h2 className="text-xs font-medium text-mid-gray uppercase tracking-wide shrink-0">
          {t("settings.history.title")}
        </h2>
        <div className="flex-1 flex items-center gap-2 px-3 py-1.5 rounded-lg border border-mid-gray/20 bg-background focus-within:border-logo-primary">
          <Search className="w-4 h-4 text-text/40 shrink-0" />
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("settings.history.searchPlaceholder")}
            aria-label={t("settings.history.searchPlaceholder")}
            className="flex-1 min-w-0 bg-transparent text-sm outline-none select-text"
          />
          {query && (
            <button
              onClick={() => setQuery("")}
              title={t("settings.history.clearSearch")}
              className="text-text/40 hover:text-logo-primary cursor-pointer"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>
        <OpenRecordingsButton
          onClick={openRecordingsFolder}
          label={t("settings.history.openFolder")}
        />
      </div>
      {content}
    </div>
  );
};

interface HistoryEntryProps {
  entry: HistoryEntry;
  onToggleSaved: () => void;
  onCopyText: () => Promise<boolean>;
  onSwitchText: (action: EditAction) => Promise<void>;
  onRecover: () => Promise<void>;
  getAudioUrl: (fileName: string) => Promise<string | null>;
  deleteAudio: (id: number) => Promise<void>;
  retryTranscription: (id: number) => Promise<void>;
}

const HistoryEntryComponent: React.FC<HistoryEntryProps> = ({
  entry,
  onToggleSaved,
  onCopyText,
  onSwitchText,
  onRecover,
  getAudioUrl,
  deleteAudio,
  retryTranscription,
}) => {
  const { t, i18n } = useTranslation();
  const [showCopied, setShowCopied] = useState(false);
  const [retrying, setRetrying] = useState(false);
  const [recovering, setRecovering] = useState(false);
  const [expanded, setExpanded] = useState(false);

  const kind = rowKind(entry);
  const hasTranscription = shownText(entry).trim().length > 0;
  const action = editAction(entry);
  const app = appName(entry);

  const handleLoadAudio = useCallback(
    () => getAudioUrl(entry.file_name),
    [getAudioUrl, entry.file_name],
  );

  const handleCopyText = async () => {
    if (!hasTranscription) {
      return;
    }

    const copied = await onCopyText();
    if (!copied) {
      toast.error(t("settings.history.copyError"));
      return;
    }

    setShowCopied(true);
    setTimeout(() => setShowCopied(false), 2000);
  };

  const handleDeleteEntry = async () => {
    try {
      await deleteAudio(entry.id);
    } catch (error) {
      console.error("Failed to delete entry:", error);
      toast.error(t("settings.history.deleteError"));
    }
  };

  const handleRetranscribe = async () => {
    try {
      setRetrying(true);
      await retryTranscription(entry.id);
    } catch (error) {
      console.error("Failed to re-transcribe:", error);
      toast.error(t("settings.history.retranscribeError"));
    } finally {
      setRetrying(false);
    }
  };

  const handleRecover = async () => {
    setRecovering(true);
    try {
      await onRecover();
    } finally {
      setRecovering(false);
    }
  };

  let text: React.ReactNode;
  if (retrying) {
    text = (
      <p
        className="italic text-sm"
        style={{ animation: "transcribe-pulse 3s ease-in-out infinite" }}
      >
        <style>{`
          @keyframes transcribe-pulse {
            0%, 100% { color: color-mix(in srgb, var(--color-text) 40%, transparent); }
            50% { color: color-mix(in srgb, var(--color-text) 90%, transparent); }
          }
        `}</style>
        {t("settings.history.transcribing")}
      </p>
    );
  } else if (kind === "discarded") {
    text = (
      <p className="text-sm text-text/40">
        {t("settings.history.discarded")}{" "}
        <button
          onClick={handleRecover}
          disabled={recovering}
          className="underline hover:text-logo-primary cursor-pointer disabled:cursor-not-allowed"
        >
          {t("settings.history.recover")}
        </button>
      </p>
    );
  } else if (kind === "failed") {
    text = (
      <p
        onClick={() => setExpanded((open) => !open)}
        className="italic text-sm text-text/40 cursor-pointer"
      >
        {t("settings.history.transcriptionFailed")}
      </p>
    );
  } else {
    text = (
      <p
        onClick={() => setExpanded((open) => !open)}
        className={`text-sm text-text/90 select-text cursor-text break-words ${
          expanded ? "whitespace-pre-wrap" : "line-clamp-2"
        }`}
      >
        {shownText(entry)}
      </p>
    );
  }

  return (
    <div className="group px-4 py-3 flex gap-4 hover:bg-mid-gray/5">
      <span className="w-12 shrink-0 pt-0.5 text-xs text-text/50 tabular-nums">
        {timeLabel(entry.timestamp, i18n.language)}
      </span>
      <div className="flex-1 min-w-0 flex flex-col gap-1">
        {text}
        {app && (
          <span className="text-xs text-text/50">
            {t("settings.history.inApp", { app })}
          </span>
        )}
        {expanded && (
          <AudioPlayer onLoadRequest={handleLoadAudio} className="w-full" />
        )}
      </div>
      <div
        className={`flex items-start shrink-0 transition-opacity ${
          expanded
            ? "opacity-100"
            : "opacity-0 group-hover:opacity-100 focus-within:opacity-100"
        }`}
      >
        <IconButton
          onClick={handleCopyText}
          disabled={!hasTranscription || retrying}
          title={t("settings.history.copyToClipboard")}
        >
          {showCopied ? (
            <Check width={16} height={16} />
          ) : (
            <Copy width={16} height={16} />
          )}
        </IconButton>
        {action && kind === "normal" && (
          <IconButton
            onClick={() => onSwitchText(action)}
            disabled={retrying}
            title={
              action === "undo"
                ? t("settings.history.undoAiEdit")
                : t("settings.history.applyAiEdit")
            }
          >
            {action === "undo" ? (
              <Undo2 width={16} height={16} />
            ) : (
              <Redo2 width={16} height={16} />
            )}
          </IconButton>
        )}
        <IconButton
          onClick={onToggleSaved}
          disabled={retrying}
          active={entry.saved}
          title={
            entry.saved
              ? t("settings.history.unsave")
              : t("settings.history.save")
          }
        >
          <Star
            width={16}
            height={16}
            fill={entry.saved ? "currentColor" : "none"}
          />
        </IconButton>
        <IconButton
          onClick={handleRetranscribe}
          disabled={retrying}
          title={t("settings.history.retranscribe")}
        >
          <RotateCcw
            width={16}
            height={16}
            style={
              retrying
                ? { animation: "spin 1s linear infinite reverse" }
                : undefined
            }
          />
        </IconButton>
        <IconButton
          onClick={handleDeleteEntry}
          disabled={retrying}
          title={t("settings.history.delete")}
        >
          <Trash2 width={16} height={16} />
        </IconButton>
      </div>
    </div>
  );
};
