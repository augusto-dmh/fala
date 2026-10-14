// What leaves this computer, per channel, for the Privacy section in About. Pure: the section
// reads the settings, `meeting_keys` and `mcp.toml`, and this module reduces them to booleans
// and a provider name, so no key value ever reaches the markup.
//
// Each row documents an ADR: dictation 0004/0012, meeting transcription 0005, meeting notes
// 0016, MCP 0010, no backend 0008. A change to one of those outputs changes this file too.
import type { AppSettings, MeetingKeys } from "@/bindings";

export const DECISIONS_URL =
  "https://github.com/augusto-dmh/fala/tree/main/docs/decisions";

/** The provider id of the automatic dictation LLM (ADR-0004), as in `LlmSettings`. */
const GEMINI_PROVIDER_ID = "gemini";

export type Channel =
  | "dictation"
  | "experimentalPostProcess"
  | "meetingTranscription"
  | "meetingNotes"
  | "mcp"
  | "telemetry";

/** One badge of a row's current state. */
export type Status =
  | "on"
  | "off"
  | "keySet"
  | "keyMissing"
  /** The AI is on but has no key: nothing goes out until one is saved. */
  | "waitingForKey"
  | "unknown"
  | "doesNotExist";

export interface PrivacyRow {
  channel: Channel;
  statuses: Status[];
  /** Interpolated into the destination, for the experimental post-processing row. */
  provider?: string;
}

export interface PrivacyInputs {
  llmEnabled: boolean;
  /** `undefined`: the settings have not loaded, so the key is not known. */
  geminiKey: boolean | undefined;
  /** The provider of the inherited post-processing shortcut, when it is on. */
  experimentalProvider: string | null;
  /** `null`: `meeting_keys` failed or has not answered. */
  meetingKeys: MeetingKeys | null;
  mcpEnabled: boolean;
}

const hasValue = (value: string | null | undefined) =>
  (value ?? "").trim() !== "";

/**
 * `true` only when `mcp.toml` turns the MCP on. `crates/mcp` reads the TOML and accepts only
 * the top-level `enabled = true`; this is looser on purpose (any `enabled = true` line, even
 * under a table or quoted), so when the two disagree the screen says "on", never "off".
 */
export function mcpEnabledFromToml(text: string | null): boolean {
  if (text === null) return false;
  return text
    .split(/\r?\n/)
    .some((line) => /^\s*["']?enabled["']?\s*=\s*true\s*(#.*)?$/.test(line));
}

/** Booleans and a provider name out of the settings: the key values stop here. */
export function privacyInputs(
  settings: AppSettings | null | undefined,
  meetingKeys: MeetingKeys | null,
  mcpToml: string | null,
): PrivacyInputs {
  const keys = settings?.post_process_api_keys ?? {};
  let experimentalProvider: string | null = null;
  if (settings?.post_process_enabled) {
    const id = settings.post_process_provider_id ?? "";
    const provider = settings.post_process_providers?.find((p) => p.id === id);
    experimentalProvider = provider?.label || id;
  }
  return {
    // Same defaults as the backend: the LLM starts on (`default_llm_enabled`).
    llmEnabled: settings?.llm_enabled ?? true,
    geminiKey: settings ? hasValue(keys[GEMINI_PROVIDER_ID]) : undefined,
    experimentalProvider,
    meetingKeys,
    mcpEnabled: mcpEnabledFromToml(mcpToml),
  };
}

const keyStatus = (present: boolean | undefined): Status =>
  present === undefined ? "unknown" : present ? "keySet" : "keyMissing";

/** The rows, in the order the section shows them. */
export function privacyRows(inputs: PrivacyInputs): PrivacyRow[] {
  const rows: PrivacyRow[] = [
    {
      channel: "dictation",
      // One state, as the settings row says it: on but without a key, nothing goes out yet.
      // An unknown key (settings not loaded) still says "on", never less than what may leave.
      statuses: !inputs.llmEnabled
        ? ["off", keyStatus(inputs.geminiKey)]
        : inputs.geminiKey === false
          ? ["waitingForKey"]
          : ["on", keyStatus(inputs.geminiKey)],
    },
  ];
  if (inputs.experimentalProvider !== null) {
    rows.push({
      channel: "experimentalPostProcess",
      statuses: ["on"],
      provider: inputs.experimentalProvider,
    });
  }
  rows.push(
    {
      channel: "meetingTranscription",
      statuses: [keyStatus(inputs.meetingKeys?.elevenlabs)],
    },
    {
      channel: "meetingNotes",
      statuses: [keyStatus(inputs.meetingKeys?.anthropic)],
    },
    { channel: "mcp", statuses: [inputs.mcpEnabled ? "on" : "off"] },
    { channel: "telemetry", statuses: ["doesNotExist"] },
  );
  return rows;
}

/** `<data dir>/audio`, where the meeting audio is retained (ADR-0005, ADR-0014). */
export function meetingAudioDir(appDir: string): string {
  if (appDir === "") return "";
  const sep = appDir.includes("\\") ? "\\" : "/";
  return appDir.replace(/[\\/]+$/, "") + sep + "audio";
}
