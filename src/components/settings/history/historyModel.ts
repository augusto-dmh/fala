import type { HistoryEntry, Result } from "@/bindings";
import { formatClock } from "@/utils/dateFormat";

/** The edit button an entry offers: undo the AI edit, or apply it (after an undo, or when the
 * AI answer arrived after the paste). */
export type EditAction = "undo" | "redo";

/** The text an entry shows and copies: its dictation's final or raw text, else the history.db text. */
export function shownText(entry: HistoryEntry): string {
  const d = entry.dictation;
  if (!d) return entry.transcription_text;
  return d.showing === "raw" ? d.raw_text : d.final_text;
}

/** "undo" for an AI-edited entry showing the edit, "redo" while it shows the raw text, `null`
 * otherwise. */
export function editAction(entry: HistoryEntry): EditAction | null {
  const d = entry.dictation;
  if (!d || d.editor !== "llm") return null;
  return d.showing === "final" ? "undo" : "redo";
}

/** A long dictation pasted with the AI text in some blocks and the rules in others. */
export function partialAi(entry: HistoryEntry): boolean {
  return entry.llm_partial;
}

/** The app the text went to, when known. */
export function appName(entry: HistoryEntry): string | null {
  return entry.dictation?.app_name ?? null;
}

// Executables whose product name is not the capitalised file name.
const APP_LABELS: Record<string, string> = {
  chrome: "Chrome",
  code: "VS Code",
  excel: "Excel",
  explorer: "File Explorer",
  firefox: "Firefox",
  msedge: "Edge",
  ms_teams: "Teams",
  notepad: "Notepad",
  olk: "Outlook",
  outlook: "Outlook",
  powerpnt: "PowerPoint",
  slack: "Slack",
  teams: "Teams",
  windowsterminal: "Windows Terminal",
  winword: "Word",
};

/** The app as people know it: `winword` -> Word, else the executable name capitalised. */
export function appLabel(name: string): string {
  const known = APP_LABELS[name.toLowerCase()];
  if (known) return known;
  return name.charAt(0).toUpperCase() + name.slice(1);
}

export interface SwitchTextDeps {
  undo: (id: number) => Promise<Result<HistoryEntry, string>>;
  redo: (id: number) => Promise<Result<HistoryEntry, string>>;
  copy: (text: string) => Promise<boolean>;
}

export interface SwitchTextOutcome {
  /** The entry to put in place of the old one; `null` when nothing changed. */
  entry: HistoryEntry | null;
  success: boolean;
  toastKey: string;
}

/** How a row renders: discarded (grey, "Recuperar"), failed (no ASR text, retry) or normal. */
export type RowKind = "discarded" | "failed" | "normal";

export function rowKind(entry: HistoryEntry): RowKind {
  if (entry.discarded) return "discarded";
  return shownText(entry).trim().length > 0 ? "normal" : "failed";
}

/** The empty Home: how to dictate, with the shortcut as it is set now. */
export function emptyHint(
  keys: string,
  toggle: boolean,
  t: (key: string, options?: Record<string, unknown>) => string,
): string {
  if (!keys) return t("settings.history.empty");
  return t(
    toggle ? "settings.history.emptyToggle" : "settings.history.emptyHold",
    { keys },
  );
}

/** How long a deleted dictation can still come back with the toast's "Undo". */
export const DELETE_UNDO_MS = 5000;

/** `entry` back in `list` (newest first) after an undone delete, where its time puts it. */
export function restoreEntry(
  list: readonly HistoryEntry[],
  entry: HistoryEntry,
): HistoryEntry[] {
  if (list.some((e) => e.id === entry.id)) return [...list];
  const at = list.findIndex((e) => e.timestamp < entry.timestamp);
  return at < 0
    ? [...list, entry]
    : [...list.slice(0, at), entry, ...list.slice(at)];
}

/** Milliseconds between the last keystroke and the search. */
export const SEARCH_DEBOUNCE_MS = 200;

/** The query to send, or `null` when the field is blank and the paged list shows. */
export function searchQuery(text: string): string | null {
  const query = text.trim();
  return query.length > 0 ? query : null;
}

/** A day of the history: "today", "yesterday" or a date, entries newest first. */
export interface DayGroup {
  key: string;
  day: "today" | "yesterday" | "date";
  /** Seconds of the group's first (newest) entry, for formatting the date. */
  timestamp: number;
  entries: HistoryEntry[];
}

function localDayKey(date: Date): string {
  return `${date.getFullYear()}-${date.getMonth() + 1}-${date.getDate()}`;
}

/** Groups entries (already newest first) by local calendar day. */
export function groupByDay(entries: HistoryEntry[], now: Date): DayGroup[] {
  const today = localDayKey(now);
  const yesterdayDate = new Date(now);
  yesterdayDate.setDate(now.getDate() - 1);
  const yesterday = localDayKey(yesterdayDate);

  const groups: DayGroup[] = [];
  for (const entry of entries) {
    const key = localDayKey(new Date(entry.timestamp * 1000));
    const last = groups[groups.length - 1];
    if (last && last.key === key) {
      last.entries.push(entry);
      continue;
    }
    const day =
      key === today ? "today" : key === yesterday ? "yesterday" : "date";
    groups.push({ key, day, timestamp: entry.timestamp, entries: [entry] });
  }
  return groups;
}

/** The entry's local time, hours and minutes, in the UI language ("3:50 PM", "15:50"). */
export function timeLabel(timestamp: number, locale: string): string {
  return formatClock(new Date(timestamp * 1000), locale);
}

export interface RecoverDeps {
  recover: (id: number) => Promise<Result<HistoryEntry, string>>;
  copy: (text: string) => Promise<boolean>;
}

/** "Recuperar": the backend restores the entry, then its text goes to the clipboard. */
export async function recoverEntry(
  entry: HistoryEntry,
  deps: RecoverDeps,
): Promise<SwitchTextOutcome> {
  const failed: SwitchTextOutcome = {
    entry: null,
    success: false,
    toastKey: "settings.history.recoverError",
  };
  let result: Result<HistoryEntry, string>;
  try {
    result = await deps.recover(entry.id);
  } catch (error) {
    console.error("Failed to recover the history entry:", error);
    return failed;
  }
  if (result.status !== "ok") {
    console.error("Failed to recover the history entry:", result.error);
    return failed;
  }
  const copied = await deps.copy(shownText(result.data));
  return {
    entry: result.data,
    success: copied,
    toastKey: copied
      ? "settings.history.recovered"
      : "settings.history.copyError",
  };
}

/** Undo or redo the AI edit, then copy the text the entry now shows. */
export async function switchText(
  entry: HistoryEntry,
  action: EditAction,
  deps: SwitchTextDeps,
): Promise<SwitchTextOutcome> {
  const failed: SwitchTextOutcome = {
    entry: null,
    success: false,
    toastKey: "settings.history.editToggleError",
  };
  let result: Result<HistoryEntry, string>;
  try {
    result =
      action === "undo" ? await deps.undo(entry.id) : await deps.redo(entry.id);
  } catch (error) {
    console.error("Failed to switch the history text:", error);
    return failed;
  }
  if (result.status !== "ok") {
    console.error("Failed to switch the history text:", result.error);
    return failed;
  }
  const copied = await deps.copy(shownText(result.data));
  if (!copied) {
    return {
      entry: result.data,
      success: false,
      toastKey: "settings.history.copyError",
    };
  }
  return {
    entry: result.data,
    success: true,
    toastKey:
      action === "undo"
        ? "settings.history.originalCopied"
        : "settings.history.editedCopied",
  };
}
