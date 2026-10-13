import { useEffect, useState } from "react";

/** Finds the element to focus once the list has settled. */
export type FocusTarget = () => HTMLElement | null | undefined;

/**
 * Focuses the pending target unless the write still runs, and only if focus was lost on the way
 * (the removed entry's button went away and left it on `body`). Returns what is still pending.
 */
export function settleFocus(
  pending: FocusTarget | null,
  busy: boolean,
  active: Element | null,
  body: Element | null,
): FocusTarget | null {
  if (busy || !pending) return pending;
  if (active === null || active === body) pending()?.focus();
  return null;
}

/**
 * Focus that survives a settings write: `focusWhenIdle(target)` focuses what `target` finds as
 * soon as `busy` is false again, unless the person has moved focus elsewhere meanwhile.
 */
export function useFocusWhenIdle(busy: boolean): (target: FocusTarget) => void {
  const [pending, setPending] = useState<FocusTarget | null>(null);
  useEffect(() => {
    if (busy || !pending) return;
    settleFocus(pending, busy, document.activeElement, document.body);
    setPending(null);
  }, [busy, pending]);
  return (target: FocusTarget) => setPending(() => target);
}

/** After removing `removed` from `visible`: the entry that takes its place, else the one before. */
export function neighbourAfterRemoval<T>(
  visible: readonly T[],
  removed: T,
): T | null {
  const at = visible.indexOf(removed);
  if (at < 0) return null;
  return visible[at + 1] ?? visible[at - 1] ?? null;
}

/** The remove button of `entry` inside `list`, marked with `data-entry`. */
export function entryButton(
  list: HTMLElement | null,
  entry: string,
): HTMLElement | undefined {
  if (!list) return undefined;
  return [...list.querySelectorAll<HTMLElement>("[data-entry]")].find(
    (button) => button.dataset.entry === entry,
  );
}
