import type { HistoryEntry, Result } from "@/bindings";

/** The edit button an entry offers: undo the AI edit, or reapply it. */
export type EditAction = "undo" | "redo";

/** The text an entry shows and copies: its dictation's final or raw text, else the history.db text. */
export function shownText(entry: HistoryEntry): string {
  const d = entry.dictation;
  if (!d) return entry.transcription_text;
  return d.showing === "raw" ? d.raw_text : d.final_text;
}

/** "undo" for an AI-edited entry showing the edit, "redo" once undone, `null` otherwise. */
export function editAction(entry: HistoryEntry): EditAction | null {
  const d = entry.dictation;
  if (!d || d.editor !== "llm") return null;
  return d.showing === "final" ? "undo" : "redo";
}

/** The app the text went to, when known. */
export function appName(entry: HistoryEntry): string | null {
  return entry.dictation?.app_name ?? null;
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
