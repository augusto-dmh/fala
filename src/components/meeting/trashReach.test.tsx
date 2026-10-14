// Prova dos checks de `.specs/features/qa-red-trash/checks.md` (R7, R8, R16, R17 e R18 da segunda
// passada de QA, 2026-10-13).
// Rode com `bun src/components/meeting/trashReach.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { day } from "./GranolaImport";
import { SessionMenuItems } from "./SessionHeader";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));

// T1: "Mover para a lixeira" fica separado e em vermelho no menu ⋯ (R8).
{
  const html = renderToStaticMarkup(
    <SessionMenuItems
      transcript
      onCopy={noop}
      onCopyTranscript={noop}
      onPeople={noop}
      onDelete={noop}
      deleteDisabled={false}
      t={(key) => key}
    />,
  );
  assert.match(
    html,
    /<div role="separator"[^>]*><\/div><button[^>]*class="[^"]*text-danger[^"]*"[^>]*>meeting\.trash\.action<\/button>/,
  );
  ok("T1");
}

// T2: a lixeira fica num botão fixo na linha das ações, com a contagem, e não só num link no pé
// que some no zero (R18).
{
  const page = read("./MeetingsPage.tsx");
  assert.match(
    page,
    /aria-label=\{t\("meeting\.trash\.link", \{ count: trashed\.length \}\)\}/,
  );
  assert.doesNotMatch(page, /trashed\.length > 0 && \(\s*<button/);
  ok("T2");
}

// T3: o resultado do Granola sai quando uma reunião abre; as datas no idioma da UI (R16).
{
  const page = read("./MeetingsPage.tsx");
  // Só o resultado de uma importação que terminou bem, e só daquele arquivo.
  assert.match(page, /if \(finished\) setGranolaDone\(granolaPath\);/);
  assert.match(page, /granolaDone !== null &&\s*granolaDone === granolaPath/);
  assert.match(page, /\}, \[openedId\]\);/);
  assert.equal(day("2026-10-10T09:00:00-03:00", "en-US"), "Oct 10, 2026");
  assert.equal(day("2026-10-10T09:00:00-03:00", "pt-BR"), "10 de out. de 2026");
  assert.equal(day(null, "pt-BR"), "");
  // Um dia fora do calendário fica como veio, sem virar outra data.
  assert.equal(day("2024-13-45", "pt-BR"), "2024-13-45");
  assert.equal(day("0024-01-02", "en-US").endsWith("24"), true);
  assert.equal(pt.meeting.granola.row, "Linha {{line}}: {{reason}}");
  assert.equal(en.meeting.granola.row, "Row {{line}}: {{reason}}");
  ok("T3");
}

// T4: "apagar para sempre" não usa "sair deste computador", que a Privacidade usa para envio; e
// sem o jargão "âncoras" (R7, R17).
{
  for (const locale of [pt, en]) {
    assert.doesNotMatch(
      locale.meeting.delete.body,
      /leave this computer|saem deste computador/,
    );
    assert.doesNotMatch(locale.meeting.granola.noAnchors, /anchors|âncoras/i);
  }
  ok("T4");
}

console.log("trash reach: all assertions passed");
