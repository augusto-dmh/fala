// Which release note the What's New modal shows, apart from where the notes come from (bundled
// Markdown or the notes the updater kept), so it runs without Vite's `import.meta.glob`.

export interface ReleaseNote {
  version: string;
  markdown: string;
}

export const parseVersion = (
  version: string,
): [number, number, number] | null => {
  const normalized = version.trim().replace(/^v/i, "");
  const match = normalized.match(/^(\d+)\.(\d+)\.(\d+)/);
  if (!match) return null;

  return [Number(match[1]), Number(match[2]), Number(match[3])];
};

export const compareVersions = (a: string, b: string) => {
  const parsedA = parseVersion(a);
  const parsedB = parseVersion(b);

  if (!parsedA || !parsedB) return 0;

  for (let index = 0; index < parsedA.length; index += 1) {
    if (parsedA[index] !== parsedB[index]) {
      return parsedA[index] - parsedB[index];
    }
  }

  return 0;
};

/** The newest note not newer than the running version and newer than the last one seen. On a
 * tie the first note in `notes` wins. */
export const pickReleaseNote = (
  notes: ReleaseNote[],
  currentVersion: string,
  lastSeenVersion: string,
): ReleaseNote | null => {
  if (!parseVersion(currentVersion)) return null;

  const hasValidLastSeenVersion = parseVersion(lastSeenVersion) !== null;
  const candidate = notes
    .filter((note) => compareVersions(note.version, currentVersion) <= 0)
    .filter(
      (note) =>
        !hasValidLastSeenVersion ||
        compareVersions(note.version, lastSeenVersion) > 0,
    )
    .sort((a, b) => compareVersions(b.version, a.version))[0];

  return candidate ?? null;
};
