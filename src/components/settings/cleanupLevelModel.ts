import type { CleanupLevel } from "@/bindings";

/** The four levels in card order, from the one that changes least to the one that changes most. */
export const CLEANUP_LEVELS: readonly CleanupLevel[] = [
  "verbatim",
  "light",
  "medium",
  "heavy",
];

export const DEFAULT_CLEANUP_LEVEL: CleanupLevel = "light";

/** The level a key moves to in the radio group, wrapping around; `null` for other keys. */
export function levelForKey(
  current: CleanupLevel,
  key: string,
): CleanupLevel | null {
  const index = CLEANUP_LEVELS.indexOf(current);
  const last = CLEANUP_LEVELS.length - 1;
  switch (key) {
    case "ArrowRight":
    case "ArrowDown":
      return CLEANUP_LEVELS[index === last ? 0 : index + 1];
    case "ArrowLeft":
    case "ArrowUp":
      return CLEANUP_LEVELS[index <= 0 ? last : index - 1];
    case "Home":
      return CLEANUP_LEVELS[0];
    case "End":
      return CLEANUP_LEVELS[last];
    default:
      return null;
  }
}
