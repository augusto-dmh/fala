/** The name the backend gives the system's default audio device (`commands/audio.rs`). */
export const DEFAULT_DEVICE_NAME = "Default";

/**
 * A device as a picker shows it: the system default in the UI language, with the tray's word
 * ("Automatic"), every other device by its own name.
 */
export const deviceLabel = (
  name: string,
  t: (key: string) => string,
): string =>
  name === DEFAULT_DEVICE_NAME ? t("settings.sound.defaultDevice") : name;
