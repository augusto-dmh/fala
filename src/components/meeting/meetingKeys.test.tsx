// Prova dos checks de `.specs/features/qa-meeting-keys/checks.md` (Q16, Q17, Q21, Q22 e a largura
// do Q54 do QA de 2026-10-13).
// Rode com `bun src/components/meeting/meetingKeys.test.tsx`: imprime `<check> ok` e sai com erro
// na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import {
  missingKeyMessage,
  sessionHints,
  type Translate,
} from "./meetingModel";
import { SessionBar } from "./SessionBar";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};
const keyOnly: Translate = (key) => key;
const page = read("./MeetingsPage.tsx");
const view = read("./MeetingView.tsx");
const dialog = read("./ImportDialog.tsx");

// K1: o erro de chave diz qual falta, pelo passo que falhou; os outros erros seguem iguais.
{
  const missing = { kind: "missing_key" } as const;
  assert.equal(
    missingKeyMessage(missing, "transcribe"),
    "meeting.keys.missingElevenlabs",
  );
  assert.equal(
    missingKeyMessage(missing, "notes"),
    "meeting.keys.missingAnthropic",
  );
  assert.equal(missingKeyMessage(missing, null), null);
  assert.equal(missingKeyMessage({ kind: "busy" }, "notes"), null);
  assert.equal(missingKeyMessage("texto", "notes"), null);
  assert.equal(missingKeyMessage(null, "notes"), null);
  assert.match(page, /fail\(id, result\.error, "transcribe"\)/);
  assert.match(page, /fail\(id, result\.error, "notes"\)/);
  // Outro erro não herda o passo: erro e passo são um estado só.
  assert.match(page, /setShown\(\{ error: failure, step: null \}\)/);
  assert.match(
    page,
    /missingKey && onOpenSettings && \([\s\S]*?t\("meeting\.keys\.add"\)/,
  );
  ok("K1");
}

// K2: o erro some ao trocar de reunião ou voltar à lista, e o de uma execução que termina
// depois que a pessoa saiu da reunião não aparece em outra tela.
{
  assert.match(
    page,
    /useEffect\(\(\) => \{\s*selectedRef\.current = selectedId;\s*setError\(null\);\s*\}, \[selectedId, setError\]\);/,
  );
  assert.match(
    page,
    /if \(selectedRef\.current === id\) setShown\(\{ error: failure, step \}\);/,
  );
  ok("K2");
}

// K3: sem a chave, "Transcrever" e "Gerar notas" ficam desligados com o motivo; as notas pedem
// transcrição ou notas digitadas.
{
  const both = { elevenlabs: true, anthropic: true };
  const none = { elevenlabs: false, anthropic: false };
  const content = (over = {}) => ({
    transcript: true,
    typed: false,
    noAudio: false,
    ...over,
  });
  const off = (transcribe: string | null, generate: string | null) => ({
    transcribe,
    generate,
  });
  assert.deepEqual(sessionHints(both, content()), off(null, null));
  assert.deepEqual(
    sessionHints(both, content({ transcript: false, typed: true })),
    off(null, null),
  );
  assert.deepEqual(
    sessionHints(both, content({ transcript: false })),
    off(null, "meeting.keys.notesNeedContent"),
  );
  assert.deepEqual(
    sessionHints(none, content()),
    off("meeting.keys.transcribeNeedsKey", "meeting.keys.notesNeedKey"),
  );
  // Sem áudio (Granola), nenhuma chave faz transcrever: a dica não promete isso.
  assert.deepEqual(
    sessionHints(none, content({ noAudio: true })),
    off(null, "meeting.keys.notesNeedKey"),
  );
  // Antes de saber das chaves, nada desliga por elas.
  assert.deepEqual(sessionHints(null, content()), off(null, null));
  const html = renderToStaticMarkup(
    <SessionBar
      draft={false}
      live={null}
      transcriptOpen={false}
      onToggleTranscript={noop}
      onRecordDraft={noop}
      transcribing={false}
      transcribeDisabled
      transcribeHint="falta a ElevenLabs"
      onTranscribe={noop}
      onCancelTranscription={noop}
      generateLabel="meeting.view.generate"
      generateDisabled
      generateHint="falta a Anthropic"
      onGenerate={noop}
      templateSelect={null}
      templateMenu={() => null}
      t={keyOnly}
    />,
  );
  assert.match(html, /disabled=""[^>]*title="falta a ElevenLabs"/);
  assert.match(html, /disabled=""[^>]*title="falta a Anthropic"/);
  assert.match(view, /keys=|props\.keys \?\? null/);
  assert.match(page, /keys=\{keys\}/);
  // O motivo também aparece em texto no meio da barra (o botão desligado não recebe foco).
  assert.match(html, /text-caption text-text-2">falta a ElevenLabs<\/div>/);
  assert.match(view, /setTypedHere\(text\.trim\(\) !== ""\)/);
  assert.match(view, /typedHere \|\| detail\.annotations\.trim\(\) !== ""/);
  assert.match(view, /generateDisabled=\{notesOff\}/);
  // "Tentar de novo" também espera a chave e o conteúdo.
  assert.match(
    view,
    /disabled=\{props\.recording \|\| editing \|\| hints\.generate !== null\}/,
  );
  for (const lang of ["pt", "en"]) {
    const keys = JSON.parse(read(`../../i18n/locales/${lang}/translation.json`))
      .meeting.keys;
    for (const key of [
      "missingElevenlabs",
      "missingAnthropic",
      "add",
      "transcribeNeedsKey",
      "notesNeedKey",
      "notesNeedContent",
    ]) {
      assert.ok(keys[key], `${lang} meeting.keys.${key}`);
    }
  }
  ok("K3");
}

// K4: a prévia do Granola vem num card depois das ações, não acima do título; a página tem a
// largura das outras.
{
  const home = page.slice(page.lastIndexOf("<PageHeader"));
  assert.ok(
    home.indexOf("{granola}") > home.indexOf("<ImportDialog"),
    "na lista, depois da busca e do Importar",
  );
  const session = page.slice(page.indexOf("if (selected) {"));
  assert.ok(
    session.indexOf("{granola}") > session.indexOf("{banners}"),
    "na reunião, depois dos avisos",
  );
  assert.match(
    page,
    /key=\{`granola-\$\{granolaPath\}`\}\s*className="rounded-lg border border-border bg-surface-1 p-4 text-sm"/,
  );
  assert.ok(!page.includes("max-w-2xl"));
  ok("K4");
}

// K5: o painel de importar põe a URL numa linha própria e o Fechar num ✕ no canto, com rótulo.
{
  assert.match(
    dialog,
    /absolute end-2 top-2[\s\S]*?aria-label=\{t\("meeting\.import\.close"\)\}/,
  );
  assert.match(
    dialog,
    /type="url"\s*className="min-w-0 flex-1"[\s\S]*?aria-label=\{t\("meeting\.import\.urlPlaceholder"\)\}/,
  );
  ok("K5");
}

console.log("meeting keys: all assertions passed");
