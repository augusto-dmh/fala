// A portable copy cannot update itself in place (no installer, and Windows won't let a running
// exe replace itself): the About section links the installer from the manifest instead, or the
// releases page when there is none.
export const PORTABLE_RELEASES_URL =
  "https://github.com/augusto-dmh/fala/releases/latest";

/** Release page of a version; append the version without the `v`. */
export const RELEASE_NOTES_URL =
  "https://github.com/augusto-dmh/fala/releases/tag/v";
