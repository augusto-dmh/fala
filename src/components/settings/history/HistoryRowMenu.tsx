import React, { useCallback, useEffect, useRef, useState } from "react";
import { MoreVertical } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useDismiss } from "@/hooks/useDismiss";
import type { EditAction } from "./historyModel";

export interface HistoryRowMenuProps {
  /** Undo or apply the AI edit, when the row has one to switch. */
  editAction: EditAction | null;
  /** The recording is still on disk, so the row can be transcribed again. */
  canRetranscribe: boolean;
  /** A retry is running: nothing else can change the row. */
  busy: boolean;
  onSwitchText: (action: EditAction) => void;
  onRetranscribe: () => void;
  onDelete: () => void;
}

/** What "⋮" offers on a dictation: the AI edit, transcribe again and, apart and in red,
 *  delete. */
export function HistoryRowMenuItems(
  props: HistoryRowMenuProps & { onPick?: () => void },
) {
  const { t } = useTranslation();
  const pick = (action: () => void) => () => {
    props.onPick?.();
    action();
  };
  const item = (label: string, onClick: () => void, danger = false) => (
    <button
      key={label}
      type="button"
      role="menuitem"
      disabled={props.busy}
      className={`block w-full rounded-md px-3 py-1.5 text-start text-body transition-colors hover:bg-surface-2 focus-visible:focus-ring disabled:cursor-not-allowed disabled:opacity-50 ${
        danger ? "text-danger" : "text-text"
      }`}
      onClick={pick(onClick)}
    >
      {label}
    </button>
  );
  const { editAction } = props;
  const top = [
    editAction &&
      item(
        editAction === "undo"
          ? t("settings.history.undoAiEdit")
          : t("settings.history.applyAiEdit"),
        () => props.onSwitchText(editAction),
      ),
    props.canRetranscribe &&
      item(t("settings.history.retranscribe"), props.onRetranscribe),
  ].filter(Boolean);
  return (
    <div role="menu" className="p-1">
      {top}
      {top.length > 0 && (
        <div role="separator" className="my-1 border-t border-border" />
      )}
      {item(t("settings.history.delete"), props.onDelete, true)}
    </div>
  );
}

/** The "⋮" button of a history row and its menu; picking an item closes it. */
export function HistoryRowMenu(props: HistoryRowMenuProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const close = useCallback(() => setOpen(false), []);
  // Esc and a picked item put focus back on "⋮", as the Dropdown does; a click elsewhere leaves it
  // where the click put it.
  const closeToButton = () => {
    setOpen(false);
    button.current?.focus();
  };
  useDismiss(open, close, box);
  const items = () =>
    Array.from(
      box.current?.querySelectorAll<HTMLButtonElement>(
        '[role="menuitem"]:not(:disabled)',
      ) ?? [],
    );
  // Opened, the focus goes to the first item; the arrows move it, and Tab out closes the menu.
  useEffect(() => {
    if (open) items()[0]?.focus();
  }, [open]);
  const onKeyDown = (event: React.KeyboardEvent) => {
    if (open && event.key === "Escape") {
      event.preventDefault();
      closeToButton();
      return;
    }
    if (!open || (event.key !== "ArrowDown" && event.key !== "ArrowUp")) return;
    event.preventDefault();
    const list = items();
    const at = list.indexOf(document.activeElement as HTMLButtonElement);
    const step = event.key === "ArrowDown" ? 1 : -1;
    list[(at + step + list.length) % list.length]?.focus();
  };
  return (
    <div
      ref={box}
      className="relative"
      onKeyDown={onKeyDown}
      onBlur={(event) => {
        if (!box.current?.contains(event.relatedTarget as Node)) close();
      }}
    >
      <button
        ref={button}
        type="button"
        aria-label={t("settings.history.more")}
        title={t("settings.history.more")}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
        className="p-1.5 rounded-md flex items-center justify-center transition-colors cursor-pointer text-text/50 hover:text-logo-primary"
      >
        <MoreVertical width={16} height={16} aria-hidden="true" />
      </button>
      {open && (
        <div className="absolute end-0 top-full z-20 mt-1 w-56 rounded-xl border border-border bg-surface-1 shadow-lg">
          <HistoryRowMenuItems {...props} onPick={closeToButton} />
        </div>
      )}
    </div>
  );
}
