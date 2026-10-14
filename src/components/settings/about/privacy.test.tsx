// Prova dos checks da seção Privacidade (`.specs/features/privacy-outbound-panel/checks.md`).
// Rode com `bun src/components/settings/about/privacy.test.tsx`: imprime `<check> ok` e sai com
// erro na primeira falha. Renderiza com um i18next de verdade, nos dois idiomas.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { renderToStaticMarkup } from "react-dom/server";
import type { AppSettings, MeetingKeys } from "@/bindings";
import { PrivacyView } from "./PrivacyView";
import {
  DECISIONS_URL,
  mcpEnabledFromToml,
  meetingAudioDir,
  privacyInputs,
  privacyRows,
  type PrivacyRow,
} from "./privacyModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../../i18n/locales/en/translation.json"));
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};

await i18n.use(initReactI18next).init({
  resources: { pt: { translation: pt }, en: { translation: en } },
  lng: "pt",
  fallbackLng: false,
  interpolation: { escapeValue: false },
});

const decode = (html: string) =>
  html
    .replace(/&#x27;/g, "'")
    .replace(/&quot;/g, '"')
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");

const render = (rows: PrivacyRow[], audioDir = "C:\\dados\\audio") =>
  decode(
    renderToStaticMarkup(
      <PrivacyView
        rows={rows}
        audioDir={audioDir}
        onOpenAudio={noop}
        onOpenDecisions={noop}
      />,
    ),
  );

const settings = (over: Partial<AppSettings> = {}) =>
  ({ ...over }) as unknown as AppSettings;
const KEYS: MeetingKeys = { elevenlabs: true, anthropic: false };
const rowsFor = (
  s: AppSettings | null,
  keys: MeetingKeys | null = KEYS,
  toml: string | null = null,
) => privacyRows(privacyInputs(s, keys, toml));
const statusesOf = (rows: PrivacyRow[], channel: string) =>
  rows.find((r) => r.channel === channel)?.statuses;

// C1: the five channels, in order, with nothing experimental turned on.
assert.deepEqual(
  rowsFor(settings()).map((r) => r.channel),
  ["dictation", "meetingTranscription", "meetingNotes", "mcp", "telemetry"],
);
assert.deepEqual(
  rowsFor(null).map((r) => r.channel),
  ["dictation", "meetingTranscription", "meetingNotes", "mcp", "telemetry"],
);
ok("C1");

// C2: dictation = AI on/off × Gemini key present/absent; blank counts as absent; on without a
// key is "waiting for the key".
const dictationCases: [Partial<AppSettings>, string[]][] = [
  [
    { llm_enabled: true, post_process_api_keys: { gemini: "k" } },
    ["on", "keySet"],
  ],
  // On without the key: waiting, as the settings row says (Q37 do QA).
  [{ llm_enabled: true, post_process_api_keys: {} }, ["waitingForKey"]],
  [
    { llm_enabled: false, post_process_api_keys: { gemini: "k" } },
    ["off", "keySet"],
  ],
  [
    { llm_enabled: false, post_process_api_keys: { gemini: "  " } },
    ["off", "keyMissing"],
  ],
  // The backend default (`default_llm_enabled`) is on.
  [{}, ["waitingForKey"]],
  [{ post_process_api_keys: { gemini: "k" } }, ["on", "keySet"]],
  // Another provider's key is not the dictation key.
  [{ post_process_api_keys: { openai: "k" } }, ["waitingForKey"]],
];
for (const [over, want] of dictationCases) {
  assert.deepEqual(statusesOf(rowsFor(settings(over)), "dictation"), want);
}
ok("C2");

// C3: meeting transcription and notes follow `meeting_keys`; unknown when it did not answer.
const meetingCases: [MeetingKeys | null, string, string][] = [
  [{ elevenlabs: true, anthropic: true }, "keySet", "keySet"],
  [{ elevenlabs: true, anthropic: false }, "keySet", "keyMissing"],
  [{ elevenlabs: false, anthropic: true }, "keyMissing", "keySet"],
  [{ elevenlabs: false, anthropic: false }, "keyMissing", "keyMissing"],
  [null, "unknown", "unknown"],
];
for (const [keys, transcription, notes] of meetingCases) {
  const rows = rowsFor(settings(), keys);
  assert.deepEqual(statusesOf(rows, "meetingTranscription"), [transcription]);
  assert.deepEqual(statusesOf(rows, "meetingNotes"), [notes]);
}
ok("C3");

// C4: MCP on only with `enabled = true`; when unsure, the screen says on, never off.
const tomlCases: [string | null, boolean][] = [
  [null, false],
  ["", false],
  ["enabled = true", true],
  ["enabled=true\n", true],
  ["# consent\r\nenabled = true # set by hand\r\n", true],
  ['"enabled" = true', true],
  ["[server]\nenabled = true", true],
  ["enabled = false", false],
  ['enabled = "true"', false],
  ["# enabled = true", false],
  ["enabled = truex", false],
];
for (const [toml, want] of tomlCases) {
  assert.equal(mcpEnabledFromToml(toml), want, JSON.stringify(toml));
  assert.deepEqual(statusesOf(rowsFor(settings(), KEYS, toml), "mcp"), [
    want ? "on" : "off",
  ]);
}
ok("C4");

// C5: no telemetry, account or Fala server.
assert.deepEqual(statusesOf(rowsFor(settings()), "telemetry"), [
  "doesNotExist",
]);
ok("C5");

// C6: the inherited post-processing shortcut shows only while on, with the provider's name.
const providers = [
  {
    id: "openai",
    label: "OpenAI",
    base_url: "",
    allow_base_url_edit: false,
    models_endpoint: null,
    supports_structured_output: false,
  },
];
const withExperimental = rowsFor(
  settings({
    post_process_enabled: true,
    post_process_provider_id: "openai",
    post_process_providers: providers as AppSettings["post_process_providers"],
  }),
);
assert.deepEqual(
  withExperimental.map((r) => r.channel),
  [
    "dictation",
    "experimentalPostProcess",
    "meetingTranscription",
    "meetingNotes",
    "mcp",
    "telemetry",
  ],
);
assert.equal(withExperimental[1].provider, "OpenAI");
assert.deepEqual(withExperimental[1].statuses, ["on"]);
assert.equal(
  rowsFor(
    settings({
      post_process_enabled: true,
      post_process_provider_id: "custom",
    }),
  )[1].provider,
  "custom",
);
assert.ok(
  !rowsFor(
    settings({
      post_process_enabled: false,
      post_process_provider_id: "openai",
    }),
  ).some((r) => r.channel === "experimentalPostProcess"),
);
assert.match(render(withExperimental), /OpenAI, pela sua chave/);
ok("C6");

// C7: no key value reaches the markup, in either language, nor the section's own code.
const SENTINELS = ["AIzaSENTINEL-gemini-0001", "sk-SENTINEL-openai-0002"];
const secret = settings({
  llm_enabled: true,
  post_process_enabled: true,
  post_process_provider_id: "openai",
  post_process_providers: providers as AppSettings["post_process_providers"],
  post_process_api_keys: { gemini: SENTINELS[0], openai: SENTINELS[1] },
});
for (const lang of ["pt", "en"]) {
  await i18n.changeLanguage(lang);
  const html = render(rowsFor(secret, { elevenlabs: true, anthropic: true }));
  for (const s of SENTINELS) assert.ok(!html.includes(s), `${lang}: ${s}`);
  assert.ok(!html.includes("SENTINEL"), lang);
}
for (const file of ["./PrivacyView.tsx", "./PrivacySection.tsx"]) {
  assert.doesNotMatch(read(file), /api_keys/, file);
}
await i18n.changeLanguage("pt");
ok("C7");

// C8: every row, status and label exists and renders in pt and en, with no raw key left.
const allStatuses = [
  "on",
  "off",
  "keySet",
  "keyMissing",
  "unknown",
  "doesNotExist",
];
const everyRow: PrivacyRow[] = [...withExperimental];
for (const [lang, json] of [
  ["pt", pt],
  ["en", en],
] as const) {
  await i18n.changeLanguage(lang);
  const p = json.settings.about.privacy;
  const html = render(everyRow);
  for (const row of everyRow) {
    const c = p.channels[row.channel];
    for (const field of ["name", "sends", "destination"]) {
      assert.equal(
        typeof c?.[field],
        "string",
        `${lang}: ${row.channel}.${field}`,
      );
      assert.ok(c[field].trim().length > 0, `${lang}: ${row.channel}.${field}`);
    }
    assert.ok(html.includes(c.name), `${lang}: ${row.channel} name`);
    assert.ok(html.includes(c.sends), `${lang}: ${row.channel} sends`);
    assert.ok(
      html.includes(c.destination.replace("{{provider}}", "OpenAI")),
      `${lang}: ${row.channel} destination`,
    );
  }
  for (const status of allStatuses) {
    assert.equal(
      typeof p.status[status],
      "string",
      `${lang}: status.${status}`,
    );
  }
  const unknown = render(
    rowsFor(settings({ llm_enabled: false }), null, "enabled = true"),
  );
  for (const status of ["off", "keyMissing", "unknown", "on", "doesNotExist"]) {
    assert.ok(unknown.includes(p.status[status]), `${lang}: shows ${status}`);
  }
  for (const text of [
    p.title,
    p.description,
    p.columns.sends,
    p.columns.destination,
    p.keys.title,
    p.keys.description,
    p.audio.title,
    p.audio.description,
    p.decisions.title,
    p.decisions.button,
  ]) {
    assert.ok(html.includes(text), `${lang}: ${text}`);
  }
  assert.ok(!html.includes("settings.about.privacy"), `${lang}: raw key`);
  assert.ok(!html.includes("{{"), `${lang}: uninterpolated`);
}
await i18n.changeLanguage("pt");
ok("C8");

// C9: where the keys and the meeting audio are; the link to the decisions.
assert.equal(
  meetingAudioDir("C:\\Users\\a\\AppData\\Roaming\\br.com.augusto.fala"),
  "C:\\Users\\a\\AppData\\Roaming\\br.com.augusto.fala\\audio",
);
assert.equal(
  meetingAudioDir("/home/a/.local/share/br.com.augusto.fala/"),
  "/home/a/.local/share/br.com.augusto.fala/audio",
);
assert.equal(meetingAudioDir(""), "");
// The backend keeps meeting audio in `<data dir>/audio`.
assert.match(
  read("../../../../apps/desktop/src/meeting/mod.rs"),
  /audio_root\s*[:=]\s*data\.join\("audio"\)/,
);
const withPath = render(
  rowsFor(settings()),
  "C:\\dados\\br.com.augusto.fala\\audio",
);
assert.ok(withPath.includes("C:\\dados\\br.com.augusto.fala\\audio"));
assert.ok(withPath.includes(pt.common.open));
assert.ok(withPath.includes("Gerenciador de Credenciais"));
assert.equal(
  DECISIONS_URL,
  "https://github.com/augusto-dmh/fala/tree/main/docs/decisions",
);
const section = read("./PrivacySection.tsx");
assert.match(section, /openUrl\(DECISIONS_URL\)/);
assert.match(section, /revealItemInDir\(audioDir\)/);
assert.match(section, /commands\.openAppDataDir\(\)/);
assert.match(
  section,
  /readTextFile\("mcp\.toml", \{ baseDir: BaseDirectory\.AppData \}\)/,
);
assert.match(section, /commands\s*\.meetingKeys\(\)/);
ok("C9");

// C10: the section is in About, after the About group.
const about = read("./AboutSettings.tsx");
assert.match(about, /import \{ PrivacySection \} from "\.\/PrivacySection";/);
assert.match(
  about,
  /<LogDirectory grouped=\{true\} \/>\s*<\/SettingsGroup>\s*<PrivacySection \/>/,
);
ok("C10");
