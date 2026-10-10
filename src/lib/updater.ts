// The updater is off (`plugins.updater` is empty in tauri.conf.json; see
// ADR-0008 and docs/RELEASE.md). While it is, the release-notes modal, the
// update checker and their settings stay hidden: they would do nothing.
export const UPDATER_ENABLED = false;
