import {
  compareVersions,
  parseVersion,
  pickReleaseNote,
} from "./pickReleaseNote";
import type { ReleaseNote } from "./pickReleaseNote";

export type { ReleaseNote };

interface ReleaseNoteRecord {
  version: string;
  markdown: string;
}

interface FindReleaseNoteOptions {
  currentVersion: string;
  lastSeenVersion: string;
  /** The notes the updater kept from `latest.json` before installing (ADR-0018). */
  storedNote?: ReleaseNote | null;
}

const releaseNoteModules = import.meta.glob<string>(
  "../../content/release-notes/*.md",
  {
    eager: true,
    import: "default",
    query: "?raw",
  },
);

const releaseNotesByVersion = new Map<string, ReleaseNoteRecord>();

for (const [path, markdown] of Object.entries(releaseNoteModules)) {
  const match = path.match(/release-notes\/([^/]+)\.md$/);
  if (!match) continue;

  const version = match[1];

  if (!parseVersion(version)) continue;

  releaseNotesByVersion.set(version, {
    version,
    markdown,
  });
}

export const findReleaseNoteToShow = ({
  currentVersion,
  lastSeenVersion,
  storedNote,
}: FindReleaseNoteOptions): ReleaseNote | null =>
  pickReleaseNote(
    [...releaseNotesByVersion.values(), ...(storedNote ? [storedNote] : [])],
    currentVersion,
    lastSeenVersion,
  );

export const findLatestReleaseNote = (): ReleaseNote | null => {
  const candidate = Array.from(releaseNotesByVersion.values()).sort((a, b) =>
    compareVersions(b.version, a.version),
  )[0];

  if (!candidate) return null;

  return candidate;
};
