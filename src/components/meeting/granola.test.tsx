// Prova do diálogo de importação do Granola. Rode com `bun src/components/meeting/granola.test.tsx`:
// imprime `<check> ok` e sai com erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { GranolaPreview, MeetingSummary } from "@/bindings";
import { GranolaImportView, type GranolaPhase } from "./GranolaImport";
import { MeetingList } from "./MeetingList";
import type { Translate } from "./meetingModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const page = read("./MeetingsPage.tsx");
const dialog = read("./ImportDialog.tsx");
const view_ = read("./MeetingView.tsx");

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
const view = (phase: GranolaPhase) =>
  renderToStaticMarkup(
    <GranolaImportView
      phase={phase}
      onRun={noop}
      onCancel={noop}
      onClose={noop}
      t={tr}
    />,
  );

const preview = (over: Partial<GranolaPreview> = {}): GranolaPreview => ({
  meetings: 12,
  first: "2024-11-03T10:00:00+00:00",
  last: "2025-09-30T15:30:00+01:00",
  duplicates: 0,
  rejected: [],
  ...over,
});

// A prévia diz quantas reuniões e de quando a quando, e lista as linhas ignoradas com o motivo.
{
  const clean = view({ kind: "preview", preview: preview() });
  assert.ok(
    clean.includes("Reuniões no arquivo: 12, de 2024-11-03 a 2025-09-30."),
  );
  assert.ok(clean.includes(">Importar</button>"));
  assert.ok(!clean.includes("Linhas ignoradas"));

  const rejected = Array.from({ length: 7 }, (_, n) => ({
    file: null,
    line: n + 2,
    reason: { kind: "missing_id" as const },
  }));
  rejected[1] = {
    file: "export/Acme.csv",
    line: 3,
    reason: { kind: "invalid_date", detail: "ontem" },
  } as never;
  const dirty = view({
    kind: "preview",
    preview: preview({ rejected, duplicates: 2 }),
  });
  assert.ok(dirty.includes("Linhas ignoradas: 7."));
  assert.ok(dirty.includes("<li>linha 2: sem document_id</li>"));
  assert.ok(
    dirty.includes(
      "<li>export/Acme.csv, linha 3: data não reconhecida (ontem)</li>",
    ),
  );
  assert.ok(dirty.includes("<li>e mais 2</li>"));
  assert.ok(
    dirty.includes("Linhas repetidas (vale a última de cada reunião): 2."),
  );

  const none = view({ kind: "preview", preview: preview({ meetings: 0 }) });
  assert.ok(none.includes("Nenhuma reunião nesse arquivo."));
  assert.ok(/<button[^>]*disabled=""[^>]*>Importar<\/button>/.test(none));
  ok("granola-preview");
}

// Importando mostra "N de M" e o cancelar; no fim, as contagens; um erro diz o que falta.
{
  const running = view({ kind: "running", done: 3, total: 12 });
  assert.ok(running.includes('role="status"'));
  assert.ok(running.includes("Importando 3 de 12…"));
  assert.ok(running.includes(">Cancelar</button>"));
  const done = view({
    kind: "done",
    imported: { created: 10, updated: 2, rejected: 1 },
  });
  assert.ok(
    done.includes(
      "Reuniões importadas: 10 novas e 2 atualizadas. Linhas ignoradas: 1.",
    ),
  );
  const missing = view({
    kind: "error",
    error: { kind: "missing_columns", detail: "document_id, document_created" },
  });
  assert.ok(missing.includes('role="alert"'));
  assert.ok(
    missing.includes("faltam as colunas document_id, document_created"),
  );
  for (const kind of Object.keys(pt.meeting.granola.errors)) {
    assert.ok(
      !view({ kind: "error", error: { kind } as never }).includes("meeting."),
    );
  }
  ok("granola-progress");
}

// A entrada fica no menu "Importar", o diálogo vive na página, e a lista marca a reunião importada.
{
  assert.ok(/chooseGranola[\s\S]*extensions: \["csv", "zip"\]/.test(dialog));
  assert.ok(/onGranola=\{setGranolaPath\}/.test(page));
  // Sem áudio, "Transcrever" fica desligado, e as notas dizem que não têm âncoras.
  assert.ok(
    /props\.transcribing \|\|[\s\S]*imported_from === "granola"\s*\}\s*onClick=\{props\.onTranscribe\}/.test(
      view_,
    ),
  );
  assert.ok(view_.includes('t("meeting.granola.noAnchors")'));
  assert.ok(
    /<GranolaImport[\s\S]*onImported=\{\(\) => void refresh\(\)\}/.test(page),
  );
  const session: MeetingSummary = {
    id: "g",
    title: "Preço com a Acme",
    mode: "import",
    created_at: "2025-03-14T10:30:00+01:00",
    started_at: "2025-03-14T10:30:00+01:00",
    ended_at: "2025-03-14T10:30:00+01:00",
    recorded_ms: 0,
    audio_retained: false,
    transcribed: true,
    has_notes: true,
    imported_from: "granola",
  };
  const list = renderToStaticMarkup(
    <MeetingList sessions={[session]} selectedId={null} onOpen={noop} />,
  );
  assert.ok(list.includes("meeting.list.granola"));
  assert.ok(!list.includes("00:00"));
  ok("granola-list");
}
