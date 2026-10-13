// Prova da importação e do modo "só o som do computador" na página Reuniões. Rode com
// `bun src/components/meeting/import.test.tsx`: imprime `<check> ok` e sai com erro na primeira
// falha. Sem instância do i18next, `t` devolve a chave.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingStatus } from "@/bindings";
import { ImportDialog } from "./ImportDialog";
import { RecorderPanel } from "./RecorderPanel";
import {
  errorDetail,
  errorKey,
  importLine,
  isHttpUrl,
  type Translate,
} from "./meetingModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const page = read("./MeetingsPage.tsx");

function ok(id: string) {
  console.log(`${id} ok`);
}

const tr: Translate = (key, options = {}) => {
  const value = key.split(".").reduce((node, part) => node?.[part], pt);
  assert.equal(typeof value, "string", `chave sem texto em pt: ${key}`);
  return (value as string).replace(/\{\{(\w+)\}\}/g, (_, name) =>
    String(options[name]),
  );
};

const noop = () => {};

const status = (over: Partial<MeetingStatus> = {}): MeetingStatus => ({
  state: "idle",
  id: null,
  mode: null,
  recorded_ms: 0,
  mic_level: 0,
  system_level: 0,
  muted_mic: false,
  muted_system: false,
  cap_remaining_ms: null,
  low_disk_bytes: null,
  system_output: null,
  call_output: null,
  zero_mic: false,
  zero_system: false,
  dictating: false,
  ...over,
});

const panel = (s: MeetingStatus) =>
  renderToStaticMarkup(
    <RecorderPanel
      status={s}
      mode="system_only"
      title=""
      outputs={null}
      onModeChange={noop}
      onTitleChange={noop}
      onOutputChange={noop}
      onSwitchOutput={noop}
      onStart={noop}
      onNewDraft={noop}
      onPause={noop}
      onResume={noop}
      onStop={noop}
      onExtend={noop}
    />,
  );

// O modo "só o som do computador" é uma opção do seletor e, gravando, não mostra o nível do mic.
{
  const idle = panel(status());
  assert.match(idle, /class="truncate">meeting\.recorder\.systemOnly</);
  assert.ok(idle.includes("meeting.recorder.systemOnly"));
  const live = (mode: MeetingStatus["mode"]) =>
    panel(status({ state: "recording", id: "x", mode }));
  assert.ok(!live("system_only").includes("meeting.recorder.mic"));
  assert.ok(live("system_only").includes("meeting.recorder.system"));
  assert.ok(live("meeting").includes("meeting.recorder.mic"));
  ok("system-only");
}

// "Importar" fechado é um botão; importando, mostra o avanço e o cancelar.
{
  const closed = renderToStaticMarkup(
    <ImportDialog
      importing={false}
      stage={null}
      onImport={noop}
      onCancel={noop}
      onGranola={noop}
    />,
  );
  assert.ok(closed.includes("meeting.import.open"));
  const running = renderToStaticMarkup(
    <ImportDialog
      importing
      stage={{ stage: "importing", phase: "convert", done: 5, total: 10 }}
      onImport={noop}
      onCancel={noop}
      onGranola={noop}
    />,
  );
  assert.ok(running.includes('role="status"'));
  assert.ok(running.includes("meeting.import.cancel"));
  assert.ok(/<ImportDialog[\s\S]*importing=\{importing\}/.test(page));
  // Um arquivo solto na janela entra pelo mesmo caminho, nunca atrás do aviso de consentimento,
  // e só uma importação por vez sai da página.
  assert.ok(/onDragDropEvent[\s\S]*type !== "drop"/.test(page));
  assert.ok(
    /if \(!askConsent && selected === null\)\s*void importSource\(\{ kind: "file"/.test(
      page,
    ),
  );
  assert.ok(/if \(importRunning\.current\) return;/.test(page));
  // O avanço de outra sessão (uma gravação parada) não muda a linha da importação.
  assert.ok(/if \(id === importId\.current\) \{/.test(page));
  ok("import-dialog");
}

// O avanço vira uma linha legível em cada fase.
{
  assert.equal(importLine(null, tr), pt.meeting.import.starting);
  assert.equal(
    importLine(
      { stage: "importing", phase: "download", done: 512, total: 1024 },
      tr,
    ),
    "Baixando o áudio… 50%",
  );
  assert.equal(
    importLine(
      { stage: "importing", phase: "download", done: 3 * 1048576, total: 0 },
      tr,
    ),
    "Baixando o áudio… 3.0 MB",
  );
  assert.equal(
    importLine(
      { stage: "importing", phase: "convert", done: 900, total: 600 },
      tr,
    ),
    "Convertendo o áudio… 100%",
  );
  assert.equal(
    importLine({ stage: "retaining" }, tr),
    pt.meeting.import.saving,
  );
  ok("import-progress");
}

// Só http(s) com host passa no campo de URL.
{
  for (const url of [
    "https://www.youtube.com/watch?v=jNQXAC9IVRw",
    " HTTP://example.com/a ",
  ])
    assert.ok(isHttpUrl(url), url);
  for (const url of [
    "",
    "ftp://x/a.mp3",
    "file:///C:/a.mp4",
    "https://",
    "https:///a",
    "javascript:alert(1)",
    "https://a.com/b c",
  ])
    assert.ok(!isHttpUrl(url), url);
  ok("import-url");
}

// Cada erro novo tem chave própria, nas duas línguas, e o detalhe chega à mensagem.
{
  for (const kind of ["tool_missing", "download", "import"] as const) {
    const error = { kind, detail: "yt-dlp" };
    assert.equal(errorKey(error), `meeting.errors.${kind}`);
    assert.equal(typeof en.meeting.errors[kind], "string");
    assert.ok(
      tr(errorKey(error), { detail: errorDetail(error) }).includes("yt-dlp"),
    );
  }
  for (const kind of [
    "invalid_url",
    "import_no_audio",
    "import_too_long",
  ] as const) {
    assert.equal(errorKey({ kind }), `meeting.errors.${kind}`);
    assert.equal(typeof en.meeting.errors[kind], "string");
  }
  assert.equal(errorDetail({ kind: "invalid_url" }), "");
  assert.equal(typeof en.meeting.import.hint, "string");
  assert.equal(typeof en.meeting.recorder.systemOnly, "string");
  ok("import-errors");
}
