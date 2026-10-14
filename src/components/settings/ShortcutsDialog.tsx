import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Pencil, Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { commands, type BindingSlot, type ShortcutBinding } from "@/bindings";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { useSettings } from "../../hooks/useSettings";
import { useOsType } from "../../hooks/useOsType";
import { formatKeyCombination, type OSType } from "../../lib/utils/keyboard";
import { ShortcutActivationSetting } from "./ShortcutActivation";
import {
  actionName,
  dialogActions,
  messageText,
  responseMessage,
  rowView,
  standingNotes,
  type RowNote,
} from "./shortcutsModel";
import { useComboRecorder, type RecordingTarget } from "./useComboRecorder";

/** The keys of a combination as chips, with the names the shortcut inputs already show. */
export const KeyChips: React.FC<{ combo: string; osType: OSType }> = ({
  combo,
  osType,
}) => (
  <span className="flex flex-wrap items-center gap-1">
    {formatKeyCombination(combo, osType)
      .split(" + ")
      .map((key, i) => (
        <kbd
          key={`${key}-${i}`}
          className="rounded-md border border-border bg-surface-2 px-1.5 py-0.5 font-sans text-caption font-semibold text-text"
        >
          {key}
        </kbd>
      ))}
  </span>
);

interface RowProps {
  binding: ShortcutBinding;
  osType: OSType;
  recording: RecordingTarget | null;
  recordedKeys: string;
  note: RowNote | undefined;
  onRecord: (slot: BindingSlot) => void;
  onRemove: (slot: BindingSlot) => void;
  onTakeOver: (note: RowNote) => void;
}

export const ShortcutRow: React.FC<RowProps> = ({
  binding,
  osType,
  recording,
  recordedKeys,
  note,
  onRecord,
  onRemove,
  onTakeOver,
}) => {
  const { t } = useTranslation();
  const id = binding.id;
  const view = rowView(binding);
  const recordingSlot = recording?.actionId === id ? recording.slot : null;

  const recordingChip = (
    <span className="rounded-md border border-accent bg-accent/10 px-2 py-0.5 text-caption font-semibold text-text">
      {recordedKeys
        ? formatKeyCombination(recordedKeys, osType)
        : t("settings.shortcuts.pressKeys")}
    </span>
  );

  return (
    <li className="border-b border-border py-3 last:border-b-0">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="text-body font-medium text-text">
            {actionName(t, id)}
          </div>
          <div className="text-caption text-mid-gray">
            {t(`settings.shortcuts.actions.${id}.description`)}
          </div>
        </div>
        <div className="flex shrink-0 flex-col items-end gap-1.5">
          {view.slots.length === 0 &&
            (recordingSlot ? (
              recordingChip
            ) : (
              <button
                type="button"
                onClick={() => onRecord("primary")}
                className="cursor-pointer text-caption text-mid-gray underline-offset-2 hover:text-text hover:underline"
              >
                {t("settings.shortcuts.choose")}
              </button>
            ))}
          {view.slots.map((slot) => (
            <div key={slot.slot} className="flex items-center gap-1">
              {recordingSlot === slot.slot ? (
                recordingChip
              ) : (
                <KeyChips combo={slot.combo} osType={osType} />
              )}
              <button
                type="button"
                aria-label={t("settings.shortcuts.edit")}
                onClick={() => onRecord(slot.slot)}
                className="cursor-pointer rounded-md p-1 text-mid-gray hover:bg-text/5 hover:text-text"
              >
                <Pencil className="h-3.5 w-3.5" aria-hidden="true" />
              </button>
              {slot.removable && (
                <button
                  type="button"
                  aria-label={t("settings.shortcuts.remove")}
                  onClick={() => onRemove(slot.slot)}
                  className="cursor-pointer rounded-md p-1 text-mid-gray hover:bg-danger/10 hover:text-danger"
                >
                  <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
                </button>
              )}
            </div>
          ))}
          {view.canAdd &&
            (recordingSlot === "alternate" ? (
              recordingChip
            ) : (
              <button
                type="button"
                aria-label={t("settings.shortcuts.add")}
                onClick={() => onRecord("alternate")}
                className="cursor-pointer rounded-md p-1 text-mid-gray hover:bg-text/5 hover:text-text"
              >
                <Plus className="h-3.5 w-3.5" aria-hidden="true" />
              </button>
            ))}
        </div>
      </div>
      {id === "transcribe" && (
        <div className="mt-2">
          <ShortcutActivationSetting descriptionMode="tooltip" grouped={true} />
        </div>
      )}
      {note && (
        <div
          role={note.message.kind === "overlap" ? "status" : "alert"}
          className={`mt-2 flex items-center justify-between gap-2 rounded-md px-2 py-1.5 text-caption text-text ${
            note.message.kind === "overlap" ? "bg-warn/15" : "bg-danger/10"
          }`}
        >
          <span>{messageText(t, note.message, note.combo)}</span>
          {note.message.kind === "refused" &&
            note.message.refusal.kind === "in_use" &&
            note.message.refusal.take_over && (
              <Button
                size="sm"
                variant="secondary"
                onClick={() => onTakeOver(note)}
              >
                {t("settings.shortcuts.useHere")}
              </Button>
            )}
        </div>
      )}
    </li>
  );
};

interface ShortcutsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export const ShortcutsDialog: React.FC<ShortcutsDialogProps> = ({
  open,
  onOpenChange,
}) => {
  const { t } = useTranslation();
  const { getSetting, refreshSettings } = useSettings();
  const osType = useOsType();
  const bindings = getSetting("bindings") ?? {};
  const [notes, setNotes] = useState<Record<string, RowNote | undefined>>({});

  // On opening too, not only after a save (R11 of the second QA pass): asked again whenever the
  // saved shortcuts change.
  const [standing, setStanding] = useState<Record<string, RowNote>>({});
  const bindingsKey = JSON.stringify(bindings);
  useEffect(() => {
    if (!open) return;
    let current = true;
    void commands.bindingProblems().then((problems) => {
      if (current) setStanding(standingNotes(problems));
    });
    return () => {
      current = false;
    };
  }, [open, bindingsKey]);

  const setNote = (id: string, note: RowNote | undefined) =>
    setNotes((prev) => ({ ...prev, [id]: note }));

  const save = useCallback(
    async (target: RecordingTarget, combo: string, takeOver = false) => {
      const result = await commands.setBinding(
        target.actionId,
        target.slot,
        combo,
        takeOver,
      );
      if (result.status === "error") {
        setNote(target.actionId, {
          message: { kind: "error", text: String(result.error) },
          combo,
          slot: target.slot,
        });
        return;
      }
      await refreshSettings();
      const message = responseMessage(result.data);
      const note = message ? { message, combo, slot: target.slot } : undefined;
      // A save changes what the other rows overlap: their old notes give way to the standing ones.
      if (result.data.success) setNotes({ [target.actionId]: note });
      else setNote(target.actionId, note);
      if (result.data.success) toast.success(t("settings.shortcuts.saved"));
    },
    [refreshSettings, t],
  );

  const recorder = useComboRecorder(
    getSetting("keyboard_implementation"),
    osType,
    save,
  );

  const record = async (actionId: string, slot: BindingSlot) => {
    setNote(actionId, undefined);
    const error = await recorder.start({ actionId, slot });
    if (error) {
      toast.error(
        error.includes("secure-input-active")
          ? t("secureInput.recorderBlocked")
          : t("settings.shortcuts.messages.error", { error }),
      );
    }
  };

  const remove = async (actionId: string, slot: BindingSlot) => {
    setNote(actionId, undefined);
    const result = await commands.removeBinding(actionId, slot);
    if (result.status === "error") {
      toast.error(
        t("settings.shortcuts.messages.error", { error: result.error }),
      );
    }
    await refreshSettings();
  };

  const resetAll = async () => {
    if (recorder.target) await recorder.cancel();
    const result = await commands.resetAllBindings();
    if (result.status === "error") {
      const kind = result.error.kind;
      toast.error(
        kind === "failed"
          ? t("settings.shortcuts.messages.error", {
              error: result.error.detail,
            })
          : t(`settings.shortcuts.resetRefused.${kind}`),
      );
      return;
    }
    setNotes({});
    await refreshSettings();
    toast.success(t("settings.shortcuts.restored"));
  };

  // A recording in progress ends first, so no shortcut stays suspended.
  const close = () => {
    if (recorder.target) void recorder.cancel();
    setNotes({});
    onOpenChange(false);
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => (next ? onOpenChange(true) : close())}
      title={t("settings.shortcuts.title")}
      description={t("settings.shortcuts.subtitle")}
      closeLabel={t("common.close")}
      dismissible={!recorder.target}
      size="lg"
      footer={
        <>
          <Button variant="secondary" onClick={resetAll}>
            {t("settings.shortcuts.reset")}
          </Button>
          <Button onClick={close}>{t("settings.shortcuts.done")}</Button>
        </>
      }
    >
      <ul>
        {dialogActions(osType === "linux").map((id) => {
          // A save's own note first; otherwise what the saved shortcuts already get wrong.
          const note = notes[id] ?? standing[id];
          const binding = bindings[id];
          if (!binding) return null;
          return (
            <ShortcutRow
              key={id}
              binding={binding}
              osType={osType}
              recording={recorder.target}
              recordedKeys={recorder.keys}
              note={note}
              onRecord={(slot) => void record(id, slot)}
              onRemove={(slot) => void remove(id, slot)}
              onTakeOver={(note) =>
                void save({ actionId: id, slot: note.slot }, note.combo, true)
              }
            />
          );
        })}
      </ul>
    </Dialog>
  );
};
