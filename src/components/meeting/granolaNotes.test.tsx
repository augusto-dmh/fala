// Prova dos checks de `.specs/features/qa-granola-notes/checks.md` (Q13, Q14, Q15 e Q23 do QA de
// 2026-10-13).
// Rode com `bun src/components/meeting/granolaNotes.test.tsx`: imprime `<check> ok` e sai com erro
// na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingLine } from "@/bindings";
import type { Translate } from "./meetingModel";
import { NotesView } from "./NotesView";
import {
  editableText,
  GENERATED_MARKER,
  granolaImportDate,
  inlinePieces,
  parseNotes,
} from "./notesModel";
import { TranscriptView } from "./TranscriptView";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};
const tr: Translate = (key, options) =>
  options ? `${key}(${JSON.stringify(options)})` : key;

// O resumo como o import do Granola grava (`crates/storage/src/meeting_imports.rs`): marcador nos
// títulos e nas linhas, bloco de código e tabela sem marcador, rodapé em pt.
const imported = [
  `### Decisões ${GENERATED_MARKER}`,
  `- Lançar em **novembro**, com _beta_ antes ${GENERATED_MARKER}`,
  "```",
  "def f(*args, **kwargs): pass",
  "```",
  "| a_b | *c* |",
  "",
  "_Importado do Granola em 2026-10-12._",
].join("\n");

// G1: o título de seção sai sem o marcador, na leitura e no editor.
{
  const { blocks } = parseNotes(imported, []);
  assert.deepEqual(blocks[0], { kind: "heading", level: 3, text: "Decisões" });
  assert.ok(editableText(imported).startsWith("### Decisões\n"));
  ok("G1");
}

// G2: negrito e itálico viram marcação; o que não é ênfase fica como digitado; bloco de código
// e tabela ficam como escritos.
{
  assert.deepEqual(inlinePieces("Lançar em **novembro**, com _beta_ antes"), [
    { text: "Lançar em " },
    { text: "novembro", strong: true },
    { text: ", com " },
    { text: "beta", em: true },
    { text: " antes" },
  ]);
  assert.deepEqual(inlinePieces("*ênfase* e mais"), [
    { text: "ênfase", em: true },
    { text: " e mais" },
  ]);
  for (const typed of [
    "o campo user_id e 2 * 3",
    "5*3*2",
    "2 ** 3 ** 4",
    "a * b * c",
  ]) {
    assert.deepEqual(inlinePieces(typed), [{ text: typed }], typed);
  }
  assert.deepEqual(inlinePieces("**aberto e *em*"), [
    { text: "**aberto e " },
    { text: "em", em: true },
  ]);
  assert.deepEqual(inlinePieces(""), []);
  const literal = parseNotes(imported, []).blocks.flatMap((block) =>
    block.kind === "line" && block.literal ? [block.text] : [],
  );
  assert.deepEqual(literal, [
    "```",
    "def f(*args, **kwargs): pass",
    "```",
    "| a_b | *c* |",
  ]);
  ok("G2");
}

// G3: o rodapé do import sai no idioma da UI, com a data local; outra linha não é rodapé.
{
  assert.equal(
    granolaImportDate("_Importado do Granola em 2026-10-12._"),
    "2026-10-12",
  );
  assert.equal(granolaImportDate("Importado do Granola em 2026-10-12."), null);
  assert.equal(granolaImportDate("_Outra coisa._"), null);
  const html = renderToStaticMarkup(
    <NotesView
      notes={parseNotes(imported, [])}
      segments={new Map()}
      audioAvailable={false}
      playing={null}
      onPlay={noop}
      onReveal={noop}
      locale="en-US"
      t={tr}
    />,
  );
  assert.ok(!html.includes("fala:ia") && !html.includes("&lt;!--"), html);
  assert.ok(html.includes("Decisões</p>"), html);
  assert.ok(html.includes('<strong class="font-semibold">novembro</strong>'));
  assert.ok(html.includes("<em>beta</em>"));
  assert.ok(html.includes("def f(*args, **kwargs): pass</p>"), html);
  assert.ok(html.includes("| a_b | *c* |</p>"), html);
  assert.ok(html.includes("meeting.granola.importedOn"), html);
  assert.ok(html.includes("Oct 12, 2026"), html);
  assert.ok(!html.includes("Importado do Granola"), html);
  for (const lang of ["pt", "en"]) {
    const g = JSON.parse(read(`../../i18n/locales/${lang}/translation.json`))
      .meeting.granola;
    assert.ok(g.importedOn.includes("{{date}}"), lang);
    assert.ok(g.noAudio, lang);
  }
  ok("G3");
}

// G4: a transcrição importada não tem ▶ nem tempos (nem no trecho omitido); a gravada continua
// com os dois.
{
  const lines = [
    { seq: 0, t0_ms: 0, t1_ms: 0, person: null, name: null, text: "Oi" },
    { seq: 1, t0_ms: 0, t1_ms: 0, person: 1, name: "Ana", text: "Olá" },
    { seq: 2, t0_ms: 0, t1_ms: 0, person: null, text: "", omitted: true },
  ] as unknown as MeetingLine[];
  const props = {
    lines,
    audioAvailable: false,
    playing: null,
    highlighted: null,
    onPlay: noop,
    t: tr,
  };
  const untimed = renderToStaticMarkup(
    <TranscriptView {...props} timed={false} />,
  );
  assert.ok(!untimed.includes("meeting.source.play"), untimed);
  assert.ok(!untimed.includes("00:00"), untimed);
  assert.ok(untimed.includes("meeting.omitted"), untimed);
  const timed = renderToStaticMarkup(<TranscriptView {...props} />);
  assert.ok(timed.includes("meeting.source.play"));
  assert.ok(timed.includes("00:00"));
  const view = read("./MeetingView.tsx");
  assert.match(view, /timed=\{!granola\}/);
  assert.match(view, /granola\s*\?\s*"meeting\.granola\.noAudio"/);
  ok("G4");
}

// G5: a folha da transcrição fica sobre a barra (z-40) e sob os diálogos (z-50), até o fim da
// janela: a barra não corta mais a folha.
{
  const view = read("./MeetingView.tsx");
  assert.match(view, /fixed inset-y-0 end-0 z-\[45\]/);
  assert.doesNotMatch(view, /pb-24/);
  assert.match(read("./SessionBar.tsx"), /sticky bottom-4 z-40/);
  assert.match(read("../ui/Dialog.tsx"), /fixed inset-0 z-50/);
  ok("G5");
}

console.log("granola notes: all assertions passed");
