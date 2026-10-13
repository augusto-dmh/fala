import type {
  BindingResponse,
  BindingSlot,
  ShortcutBinding,
  ShortcutRefusal,
} from "@/bindings";
import { formatKeyCombination } from "../../lib/utils/keyboard";

/** The rows of the shortcuts dialog, in order. */
export const DIALOG_ACTIONS = [
  "transcribe",
  "cancel",
  "paste_last",
  "copy_last",
  "meeting_toggle",
] as const;

/** Cancel stays hidden on Linux, where its dynamic registration is unstable. */
export const dialogActions = (isLinux: boolean): string[] =>
  DIALOG_ACTIONS.filter((id) => !(isLinux && id === "cancel"));

export interface SlotView {
  slot: BindingSlot;
  combo: string;
  /** 🗑 is shown: never on cancel, nor on dictation's only combination. */
  removable: boolean;
}

export interface RowView {
  slots: SlotView[];
  /** "+" is shown: one combination and room for a second. */
  canAdd: boolean;
}

export const rowView = (binding: ShortcutBinding): RowView => {
  const combos = (
    [
      ["primary", binding.current_binding],
      ["alternate", binding.alternate_binding ?? ""],
    ] as [BindingSlot, string][]
  ).filter(([, combo]) => combo.trim());
  const keepsOne =
    binding.id === "cancel" ||
    (binding.id === "transcribe" && combos.length === 1);
  return {
    slots: combos.map(([slot, combo]) => ({
      slot,
      combo,
      removable: !keepsOne,
    })),
    canAdd: binding.id !== "cancel" && combos.length === 1,
  };
};

/** What a row says under its keys after a save. */
export type RowMessage =
  | { kind: "refused"; refusal: ShortcutRefusal }
  | { kind: "overlap"; action: string; smaller: string; larger: string }
  | { kind: "error"; text: string };

/** The message a save leaves on the row, or none when it saved cleanly. */
export const responseMessage = (
  response: BindingResponse,
): RowMessage | null => {
  if (response.refusal) return { kind: "refused", refusal: response.refusal };
  if (!response.success) return { kind: "error", text: response.error ?? "" };
  if (response.overlap) return { kind: "overlap", ...response.overlap };
  return null;
};

type Translate = (key: string, options?: Record<string, unknown>) => string;

export const actionName = (t: Translate, id: string) =>
  t(`settings.shortcuts.actions.${id}.name`, { defaultValue: id });

/** Why a combination was refused, in the person's language. */
export const refusalText = (
  t: Translate,
  refusal: ShortcutRefusal,
  combo: string,
): string => {
  switch (refusal.kind) {
    case "in_use":
      return t("settings.shortcuts.messages.inUse", {
        action: actionName(t, refusal.action),
      });
    case "reserved":
      return t("settings.shortcuts.messages.reserved", {
        keys: formatKeyCombination(combo, "windows"),
      });
    case "lone_key":
      return t("settings.shortcuts.messages.loneKey");
  }
};

export const messageText = (
  t: Translate,
  message: RowMessage,
  combo: string,
): string => {
  switch (message.kind) {
    case "refused":
      return refusalText(t, message.refusal, combo);
    case "overlap":
      return t("settings.shortcuts.messages.overlap", {
        smaller: formatKeyCombination(message.smaller, "windows"),
        larger: formatKeyCombination(message.larger, "windows"),
        action: actionName(t, message.action),
      });
    case "error":
      return t("settings.shortcuts.messages.error", { error: message.text });
  }
};

/** Esc ends the recording, except on the cancel row, where Esc is the key being recorded. */
export const escapeCancelsRecording = (actionId: string) =>
  actionId !== "cancel";
