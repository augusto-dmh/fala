// Prova dos checks de `.specs/features/qa-shortcuts-dialog-width/checks.md` (R3, R9, R10 e o Q52
// da segunda passada de QA, 2026-10-13).
// Rode com `bun src/components/settings/shortcutsWidth.test.tsx`: imprime `<check> ok` e sai com
// erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { KeyChips } from "./ShortcutsDialog";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);

// W1: o diálogo de atalhos sai largo de verdade: a largura é uma prop, e não um className que
// perde para o max-w-lg padrão (R3).
{
  // O Dialog vai por portal ao body: a regra fica conferida na fonte.
  const ui = read("../ui/Dialog.tsx");
  assert.match(
    ui,
    /const WIDTHS = \{ md: "max-w-lg", lg: "max-w-2xl" \} as const;/,
  );
  // Sem largura própria, o tamanho; com um max-w-* no className, ele manda.
  assert.ok(
    ui.includes('/(^|\\s)max-w-/.test(className) ? "" : WIDTHS[size]'),
    "um max-w-* no className substitui o tamanho",
  );
  assert.doesNotMatch(ui, /w-full max-w-lg flex-col/);
  const source = read("./ShortcutsDialog.tsx");
  assert.match(source, /size="lg"/);
  assert.doesNotMatch(source, /className="max-w-2xl"/);
  ok("W1");
}

// W2: as teclas em chips sans, como no onboarding e na pílula (R9).
{
  const html = renderToStaticMarkup(
    <KeyChips combo="ctrl+shift+space" osType="windows" />,
  );
  assert.match(html, /<kbd class="[^"]*font-sans[^"]*">Ctrl<\/kbd>/);
  ok("W2");
}

// W3: "Comportamento do atalho" fala do atalho de ditar (R10); o campo do atalho não quebra no
// onboarding (Q52).
{
  const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
  const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
  assert.match(
    pt.settings.general.shortcutActivation.description,
    /atalho de ditar/,
  );
  assert.match(
    en.settings.general.shortcutActivation.description,
    /dictation shortcut/,
  );
  for (const file of [
    "./FalaKeysShortcutInput.tsx",
    "./GlobalShortcutInput.tsx",
  ]) {
    const source = read(file);
    assert.equal(
      source.match(
        /className="whitespace-nowrap px-2 py-1 text-sm font-semibold/g,
      )?.length,
      2,
      file,
    );
  }
  ok("W3");
}

console.log("shortcuts width: all assertions passed");
