// Prova dos checks de `.specs/features/qa-meeting-shortcut/checks.md` (R1 e R11 da segunda passada
// de QA, 2026-10-13). As regras ficam no backend (`conflict.rs`, com testes em Rust); aqui, como o
// diálogo mostra o que o backend responde.
// Rode com `bun src/components/settings/meetingShortcut.test.ts`: imprime `<check> ok` e sai com
// erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { BindingProblem } from "@/bindings";
import { refusalText, standingNotes } from "./shortcutsModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const t = (key: string, options?: Record<string, unknown>) =>
  options?.action ? `${key}(${String(options.action)})` : key;

const overlap = (action: string, combo: string): BindingProblem => ({
  action,
  slot: "primary",
  combo,
  refusal: null,
  overlap: { action: "copy_last", smaller: combo, larger: "alt+shift+x" },
});
const meeting = (action: string, combo: string): BindingProblem => ({
  action,
  slot: "alternate",
  combo,
  refusal: { kind: "meeting_overlap", action: "meeting_toggle" },
  overlap: null,
});

// S1: o diálogo mostra ao abrir o que o backend acha nos atalhos salvos; uma regra da reunião vence
// um aviso de sobreposição na mesma linha, em qualquer ordem.
{
  assert.deepEqual(standingNotes([]), {});
  const warnFirst = standingNotes([
    overlap("transcribe", "alt+shift"),
    meeting("transcribe", "ctrl+alt"),
  ]);
  assert.deepEqual(warnFirst.transcribe, {
    message: {
      kind: "refused",
      refusal: { kind: "meeting_overlap", action: "meeting_toggle" },
    },
    combo: "ctrl+alt",
    slot: "alternate",
  });
  const refusalFirst = standingNotes([
    meeting("transcribe", "ctrl+alt"),
    overlap("transcribe", "alt+shift"),
  ]);
  assert.equal(refusalFirst.transcribe?.message.kind, "refused");
  const onlyWarn = standingNotes([overlap("paste_last", "alt+shift")]);
  assert.deepEqual(onlyWarn.paste_last?.message, {
    kind: "overlap",
    action: "copy_last",
    smaller: "alt+shift",
    larger: "alt+shift+x",
  });
  ok("S1");
}

// S2: as recusas novas têm texto nos dois idiomas; o diálogo pede os problemas ao abrir e quando
// os atalhos mudam, e um save limpa as notas velhas das outras linhas.
{
  assert.equal(
    refusalText(t, { kind: "meeting_needs_key" }, "ctrl+shift"),
    "settings.shortcuts.messages.meetingNeedsKey",
  );
  assert.match(
    refusalText(
      t,
      { kind: "meeting_overlap", action: "transcribe" },
      "ctrl+alt",
    ),
    /^settings\.shortcuts\.messages\.meetingOverlap\(/,
  );
  for (const lang of ["pt", "en"]) {
    const messages = JSON.parse(
      read(`../../i18n/locales/${lang}/translation.json`),
    ).settings.shortcuts.messages;
    assert.ok(messages.meetingNeedsKey, lang);
    assert.ok(messages.meetingOverlap.includes("{{action}}"), lang);
  }
  const dialog = read("./ShortcutsDialog.tsx");
  assert.match(dialog, /commands\.bindingProblems\(\)/);
  assert.match(dialog, /\[open, bindingsKey\]/);
  assert.match(dialog, /notes\[id\] \?\? standing\[id\]/);
  assert.match(
    dialog,
    /if \(result\.data\.success\) setNotes\(\{ \[target\.actionId\]: note \}\);/,
  );
  ok("S2");
}

console.log("meeting shortcut: all assertions passed");
