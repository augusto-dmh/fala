// Prova da reunião como nota (`.specs/features/meetings-ui-redesign/checks.md`: C7–C9).
// Rode com `bun src/components/meeting/screen.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha. Sem instância do i18next, `t` devolve a chave.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingDetail, MeetingSummary } from "@/bindings";
import { AnnotationsField } from "./AnnotationsField";
import { MeetingPeoplePanel } from "./MeetingPeoplePanel";
import { MeetingView } from "./MeetingView";
import type { Translate } from "./meetingModel";
import { metaLine, TitleField } from "./SessionHeader";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const keys: Translate = (key) => key;
const noop = () => {};
const MENU = "opções";

function ok(id: string) {
  console.log(`${id} ok`);
}

const now = new Date(2026, 9, 12, 18, 0);
const summary = (over: Partial<MeetingSummary> = {}): MeetingSummary => ({
  id: "s",
  title: "Daily",
  mode: "meeting",
  created_at: new Date(2026, 9, 12, 14, 0).toISOString(),
  started_at: new Date(2026, 9, 12, 14, 2).toISOString(),
  ended_at: null,
  recorded_ms: 2_250_000,
  audio_retained: true,
  transcribed: true,
  has_notes: true,
  ...over,
});
const detail = (over: Partial<MeetingDetail> = {}): MeetingDetail => ({
  summary: summary(),
  annotations: "",
  lines: [],
  notes_md: "## Notas · Reunião geral\n\n### Resumo\n\n- Data fechada",
  notes_template: "geral",
  notes_edited: false,
  notes_running: false,
  audio_available: true,
  ...over,
});
const view = (d: MeetingDetail, over: Record<string, unknown> = {}) =>
  renderToStaticMarkup(
    <MeetingView
      detail={d}
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
      {...over}
    />,
  );

// C7: o título grande e editável, salvo no Enter ou ao sair; a linha de quando, quanto e como.
{
  const title = renderToStaticMarkup(
    <TitleField
      id="s"
      title=""
      fallback="Reunião de 2026-10-12 14:00"
      onSaved={noop}
      t={keys}
    />,
  );
  assert.ok(title.startsWith("<h1"), title);
  assert.ok(title.includes('aria-label="meeting.screen.title"'));
  assert.ok(title.includes('placeholder="Reunião de 2026-10-12 14:00"'));
  const field = read("./SessionHeader.tsx");
  assert.match(field, /commands\.setMeetingTitle\(id, text\.trim\(\)\)/);
  assert.match(
    field,
    /event\.key === "Enter"\) event\.currentTarget\.blur\(\)/,
  );

  assert.equal(
    metaLine(summary(), now, "pt-BR", keys),
    "meeting.home.today · 14:02 · 37:30 · meeting.recorder.online",
  );
  assert.ok(
    metaLine(summary({ started_at: null }), now, "pt-BR", keys).includes(
      "meeting.list.draft",
    ),
  );
  assert.ok(
    metaLine(summary({ mode: "import" }), now, "pt-BR", keys).endsWith(
      "meeting.screen.imported",
    ),
  );
  // Gravando, a duração guardada está velha: a linha diz que grava.
  assert.ok(
    metaLine(summary(), now, "pt-BR", keys, true).includes(
      "meeting.home.state.recording",
    ),
  );
  const page = view(detail());
  assert.ok(page.includes('value="Daily"'));
  assert.ok(page.includes("meeting.recorder.online"));
  ok("C7");
}

// C8: "Minhas anotações | Notas": com notas, abre nas notas; sem elas, nas anotações; gravando,
// nas anotações; "/" sozinho no campo abre o template das notas.
{
  const tab = (html: string) =>
    /aria-selected="true"[^>]*>([^<]*)</.exec(html)?.[1];
  assert.equal(tab(view(detail())), "meeting.view.notes");
  assert.equal(tab(view(detail({ notes_md: null }))), "meeting.screen.myNotes");
  assert.equal(
    tab(view(detail(), { recording: true })),
    "meeting.screen.myNotes",
  );
  assert.ok(view(detail()).includes('role="tablist"'));
  // Os dois painéis ficam montados: trocar de aba não perde o que está sendo digitado.
  const panels = view(detail()).match(/role="tabpanel"( hidden="")?/g);
  assert.deepEqual(panels, ['role="tabpanel" hidden=""', 'role="tabpanel"']);
  const source = read("./MeetingView.tsx");
  assert.match(
    source,
    /if \(hasNotes && !hadNotes\.current\) setTab\("notes"\);/,
  );
  // "/": um botão por template, que escolhe e limpa o campo.
  assert.match(
    source,
    /onClick=\{\(\) => \{\s*pickTemplate\(template\.id\);\s*clear\(\);/,
  );
  // O rascunho é só a pauta, sem abas.
  const draft = view(detail({ summary: summary({ started_at: null }) }));
  assert.ok(!draft.includes('role="tablist"') && draft.includes("<textarea"));

  const field = (initial: string) =>
    renderToStaticMarkup(
      <AnnotationsField
        id="s"
        initial={initial}
        label="Anotações"
        labelHidden
        slashMenu={() => <span>{MENU}</span>}
      />,
    );
  assert.ok(field("/").includes(MENU));
  assert.ok(!field("/ pauta").includes(MENU));
  assert.ok(field("").includes('class="sr-only"'));
  assert.ok(
    field("").includes('placeholder="meeting.annotations.placeholder"'),
  );
  // Só outra sessão zera o campo: recarregar esta (pausa, parar) não volta o texto.
  assert.match(
    read("./AnnotationsField.tsx"),
    /return \(\) => current\.flush\(\);\s*\}, \[id\]\);/,
  );
  ok("C8");
}

// C9: "⋯" abre os participantes; as sugestões de nome aparecem sem abrir.
{
  const panel = (expanded: boolean) =>
    renderToStaticMarkup(
      <MeetingPeoplePanel id="s" reloadOn={null} expanded={expanded} />,
    );
  assert.ok(!panel(false).includes("meeting.people.participants"));
  assert.ok(panel(true).includes("meeting.people.participants"));
  const source = read("./MeetingPeoplePanel.tsx");
  assert.match(source, /\{expanded && \(/);
  assert.match(source, /\{people && \(\s*<SuggestionList/);
  assert.match(
    read("./MeetingView.tsx"),
    /onPeople=\{\(\) => setShowPeople\(true\)\}/,
  );
  // Apagar espera as notas que o pipeline ainda escreve.
  assert.match(
    read("./MeetingsPage.tsx"),
    /deleteDisabled=\{[^}]*selected\.notes_running\s*\}/,
  );
  ok("C9");
}
