import { useEffect, type RefObject } from "react";

/** Closes a popover on a click outside `box` or on Escape. */
export function useDismiss(
  open: boolean,
  close: () => void,
  box: RefObject<HTMLElement | null>,
) {
  useEffect(() => {
    if (!open) return;
    const outside = (event: MouseEvent) => {
      if (!box.current?.contains(event.target as Node)) close();
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    document.addEventListener("mousedown", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("mousedown", outside);
      document.removeEventListener("keydown", escape);
    };
  }, [open, close, box]);
}
