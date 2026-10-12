// Prova da nota editável e da confirmação antes de regenerar (`meeting-notes-polish`, PR 2).
// Rode com `bun src/components/meeting/notesPolish.test.tsx`: imprime `<check> ok` e sai com
// erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingDetail, MeetingLine } from "@/bindings";
import { MeetingView } from "./MeetingView";
import { errorKey, needsConfirmation, type Translate } from "./meetingModel";
import {
  editableText,
  GENERATED_MARKER,
  mergeEdit,
  parseNotes,
} from "./notesModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const viewSource = read("./MeetingView.tsx");
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

const BASE = [
  "## Anotações",
  "",
  "decidir data ^a1",
  "",
  "## Notas · Reunião geral",
  "",
  "### Resumo",
  "",
  "- Lançamento em 15/11. [[#^s12|01:05]] <!-- fala:ia -->",
  "- Ana cuida do contrato. [[#^s13|01:10]] [[#^a1|a1]] <!-- fala:ia -->",
  "",
].join("\n");

const lines: MeetingLine[] = [
  {
    seq: 12,
    channel: "system",
    person: 1,
    t0_ms: 65_000,
    t1_ms: 70_000,
    text: "Proponho 15 de novembro.",
  },
  {
    seq: 13,
    channel: "system",
    person: 2,
    name: "Ana",
    t0_ms: 70_000,
    t1_ms: 72_000,
    text: "Eu fico com o contrato.",
  },
  {
    seq: 14,
    channel: "mic",
    person: null,
    t0_ms: 72_000,
    t1_ms: 74_000,
    text: "Fechado.",
  },
];

// O editor mostra o texto que a pessoa lê: sem marcador, ponteiros nem âncora.
{
  assert.equal(
    editableText(BASE),
    [
      "## Anotações",
      "",
      "decidir data",
      "",
      "## Notas · Reunião geral",
      "",
      "### Resumo",
      "",
      "- Lançamento em 15/11.",
      "- Ana cuida do contrato.",
    ].join("\n"),
  );
  assert.ok(!editableText(BASE).includes(GENERATED_MARKER));
  ok("NP-editable");
}

// Sem mudança, a nota volta igual; a linha gerada mudada perde o marcador e as fontes, as outras
// ficam como estavam, e uma linha nova é texto do usuário.
{
  assert.equal(mergeEdit(BASE, editableText(BASE)), BASE);

  const edited = editableText(BASE)
    .replace("- Lançamento em 15/11.", "- Lançamento em 20/11.")
    .concat("\n- Ligar para o jurídico <!-- fala:ia -->");
  const merged = mergeEdit(BASE, edited);
  assert.ok(merged.includes("\n- Lançamento em 20/11.\n"), merged);
  assert.ok(merged.includes("\ndecidir data ^a1\n"), merged);
  assert.ok(
    merged.includes(
      "- Ana cuida do contrato. [[#^s13|01:10]] [[#^a1|a1]] <!-- fala:ia -->",
    ),
    merged,
  );
  assert.ok(merged.endsWith("\n- Ligar para o jurídico\n"), merged);

  const blocks = parseNotes(merged, lines).blocks.filter(
    (block) => block.kind === "line",
  );
  const byText = new Map(blocks.map((block) => [block.text, block]));
  assert.deepEqual(
    [
      byText.get("Lançamento em 20/11.")?.generated,
      byText.get("Lançamento em 20/11.")?.sources,
    ],
    [false, []],
  );
  assert.equal(byText.get("Ana cuida do contrato.")?.generated, true);
  assert.equal(byText.get("Ana cuida do contrato.")?.sources.length, 2);
  assert.equal(byText.get("Ligar para o jurídico")?.generated, false);

  // Apagar uma linha a tira da nota; duas linhas iguais voltam as duas.
  const removed = mergeEdit(
    BASE,
    editableText(BASE).replace("- Ana cuida do contrato.", ""),
  );
  assert.ok(!removed.includes("contrato"), removed);
  const twice = "- a [[#^s12|01:05]] <!-- fala:ia -->\n- a <!-- fala:ia -->\n";
  assert.equal(mergeEdit(twice, editableText(twice)), twice);
  ok("NP-merge");
}

// Nota gerada: "Editar" ao lado do título; a confirmação só manda `true` depois do clique, e a
// página repassa a escolha ao comando.
{
  const detail: MeetingDetail = {
    summary: {
      id: "01J9Z3K8M4Q7R2S5T6V7W8X9YZ",
      title: "Planejamento",
      mode: "meeting",
      created_at: "2026-10-02T13:50:00-03:00",
      started_at: "2026-10-02T14:02:30-03:00",
      ended_at: "2026-10-02T14:40:00-03:00",
      recorded_ms: 2_250_000,
      audio_retained: true,
      transcribed: true,
      has_notes: true,
    },
    annotations: "decidir data",
    lines,
    notes_md: BASE,
    notes_template: "geral",
    notes_edited: true,
    notes_running: false,
    audio_available: true,
  };
  const html = renderToStaticMarkup(
    <MeetingView
      detail={detail}
      templates={[
        { id: "geral", name: "Reunião geral", purpose: "", source: "builtin" },
      ]}
      recording={false}
      transcribing={false}
      generating={false}
      onRecordDraft={noop}
      onTranscribe={noop}
      onCancelTranscription={noop}
      onGenerate={noop}
      onCopy={noop}
    />,
  );
  assert.ok(html.includes("meeting.notesEdit.edit"), html);
  // A confirmação só aparece depois do clique em "Gerar notas".
  assert.ok(!html.includes("meeting.notesEdit.confirm"), html);
  // Uma edição feita aqui conta antes do recarregar trazer `notes_edited`.
  assert.match(
    viewSource,
    /const edited = detail\.notes_edited \|\| editedHere;\s*if \(needsConfirmation\(\{ \.\.\.detail, notes_edited: edited \}, templateId\)\) \{\s*setConfirming\(true\);\s*\} else \{\s*props\.onGenerate\(templateId, false\);/,
  );
  // "Gerar de novo" da confirmação fica desligado como o "Gerar notas".
  assert.equal(
    viewSource.match(
      /disabled=\{props\.recording \|\| props\.generating \|\| editing\}/g,
    )?.length,
    3,
  );
  // Só o mesmo template pergunta; outro template é uma versão nova (como no Granola).
  assert.equal(needsConfirmation(detail, "geral"), true);
  assert.equal(needsConfirmation(detail, "um-a-um"), false);
  assert.equal(
    needsConfirmation({ ...detail, notes_edited: false }, "geral"),
    false,
  );
  // "Gerar notas", o "Gerar de novo" depois de um nome trocado e o "Tentar de novo" passam pela
  // mesma regra.
  assert.equal(viewSource.match(/onClick=\{generate\}/g)?.length, 3);
  assert.equal(
    viewSource.match(/props\.onGenerate\(templateId, true\)/g)?.length,
    1,
  );
  assert.match(
    page,
    /commands\.generateMeetingNotes\(\s*id,\s*templateId,\s*replaceEdited,\s*\)/,
  );
  assert.equal(
    errorKey({ kind: "notes_edited" }),
    "meeting.errors.notes_edited",
  );
  ok("NP-confirm");
}

// Toda string nova existe em pt e en.
{
  const keys = [
    "meeting.notesEdit.edit",
    "meeting.notesEdit.label",
    "meeting.notesEdit.hint",
    "meeting.notesEdit.done",
    "meeting.notesEdit.confirm",
    "meeting.notesEdit.regenerate",
    "meeting.errors.notes_edited",
  ];
  for (const key of keys) {
    for (const resources of [pt, en]) {
      const value = key
        .split(".")
        .reduce((node, part) => node?.[part], resources);
      assert.equal(typeof value, "string", key);
    }
  }
  ok("NP-i18n");
}
