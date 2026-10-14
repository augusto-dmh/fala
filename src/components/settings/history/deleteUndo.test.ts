// Prova dos checks de `.specs/features/qa-undo-deleted-dictation/checks.md` (R2 e R6 da segunda
// passada de QA, 2026-10-13).
// Rode com `bun src/components/settings/history/deleteUndo.test.ts`: imprime `<check> ok` e sai com
// erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { HistoryEntry } from "@/bindings";
import { DELETE_UNDO_MS, restoreEntry } from "./historyModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const entry = (id: number, timestamp: number) =>
  ({ id, timestamp }) as unknown as HistoryEntry;

// U1: "Desfazer" devolve o ditado ao lugar que o horário dele dá, sem repetir.
{
  const list = [entry(3, 300), entry(1, 100)];
  assert.deepEqual(
    restoreEntry(list, entry(2, 200)).map((e) => e.id),
    [3, 2, 1],
  );
  assert.deepEqual(
    restoreEntry(list, entry(4, 400)).map((e) => e.id),
    [4, 3, 1],
  );
  assert.deepEqual(
    restoreEntry(list, entry(0, 50)).map((e) => e.id),
    [3, 1, 0],
  );
  assert.deepEqual(
    restoreEntry(list, entry(3, 300)).map((e) => e.id),
    [3, 1],
  );
  assert.deepEqual(
    restoreEntry([], entry(1, 1)).map((e) => e.id),
    [1],
  );
  ok("U1");
}

// U2: apagar tira a linha na hora e mostra "Ditado apagado [Desfazer]"; o comando vai quando o
// toast fecha (o relógio dele pausa sob o ponteiro), uma vez só; o que espera fica fora das páginas
// e buscas que chegam; sair da página manda o que espera; o foco vai ao ⋮ da linha vizinha.
{
  assert.equal(DELETE_UNDO_MS, 5000);
  const page = read("./HistorySettings.tsx");
  assert.match(page, /if \(!pendingDeletes\.current\.delete\(id\)\) return;/);
  assert.match(page, /onDismiss: \(\) => void commitDelete\(id\),/);
  assert.match(page, /onAutoClose: \(\) => void commitDelete\(id\),/);
  assert.doesNotMatch(page, /setTimeout\(\(\) => void sendDelete/);
  assert.match(
    page,
    /setEntries\(\(prev\) => \(isFirstPage \? shown : \[\.\.\.prev, \.\.\.shown\]\)\)/,
  );
  assert.match(page, /setResults\(notPending\(result\.data\)\)/);
  assert.match(page, /queryRef\.current === queryAtDelete/);
  assert.match(page, /action: entry && \{/);
  assert.match(page, /void commitDelete\(id\);\s*toast\.dismiss\(toastId\);/);
  assert.match(page, /data-history-row=\{entry\.id\}/);
  assert.match(
    page,
    /querySelector<HTMLElement>\('\[aria-haspopup="menu"\]'\)/,
  );
  for (const lang of ["pt", "en"]) {
    const history = JSON.parse(
      read(`../../../i18n/locales/${lang}/translation.json`),
    ).settings.history;
    assert.ok(history.deleted && history.undoDelete, lang);
  }
  ok("U2");
}

// U3: Esc ou escolher um item do menu ⋮ devolve o foco ao ⋮ (R6).
{
  const menu = read("./HistoryRowMenu.tsx");
  assert.match(menu, /<button\s+ref=\{button\}/);
  assert.match(menu, /setOpen\(false\);\s*button\.current\?\.focus\(\);/);
  assert.match(
    menu,
    /event\.key === "Escape"\) \{\s*event\.preventDefault\(\);\s*closeToButton\(\);/,
  );
  assert.match(menu, /onPick=\{closeToButton\}/);
  ok("U3");
}

console.log("delete undo: all assertions passed");
