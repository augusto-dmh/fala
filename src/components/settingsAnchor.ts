// Links from elsewhere in the app ("Set up" in Meetings, [Choose microphone] in a notice) open
// Settings on the row they are about, not at the top of the page.

/** The rows a link can open, marked by `SettingContainer`'s `anchor`. */
export type SettingsAnchor = "microphone" | "meetings-key";

/** Scrolls the anchored row into the middle of the view and focuses the row itself (its
 *  control can still be loading or disabled; Tab goes on from there). False when the row is not
 *  on the page. */
export function revealSettingsAnchor(
  root: ParentNode,
  anchor: SettingsAnchor,
): boolean {
  const row = root.querySelector<HTMLElement>(
    `[data-settings-anchor="${anchor}"]`,
  );
  if (!row) return false;
  row.scrollIntoView({ block: "center" });
  row.focus({ preventScroll: true });
  return true;
}
