// Prova dos checks C27-C30 da tela de histórico (`.specs/features/history-undo/checks.md`) e
// C2, C8 e C9 da edição tardia (`.specs/features/llm-late-edit/checks.md`).
// Rode com `bun src/components/settings/history/historyModel.test.ts`: imprime `<check> ok` e sai
// com erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { HistoryDictation, HistoryEntry, Result } from "@/bindings";
import { appName, editAction, shownText, switchText } from "./historyModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const screen = read("./HistorySettings.tsx");
const pt = JSON.parse(read("../../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../../i18n/locales/en/translation.json"));

function dictation(over: Partial<HistoryDictation> = {}): HistoryDictation {
  return {
    raw_text: "acao de amanha",
    final_text: "Ação de amanhã.",
    editor: "llm",
    showing: "final",
    app_name: "notepad",
    ...over,
  };
}

function entry(d: HistoryDictation | null, id = 7): HistoryEntry {
  return {
    id,
    file_name: "fala-1.wav",
    timestamp: 1,
    saved: false,
    title: "t",
    transcription_text: "texto do history.db",
    post_processed_text: null,
    post_process_prompt: null,
    post_process_requested: true,
    dictation_id: d ? "0190-id" : null,
    dictation: d,
  };
}

function ok(id: string) {
  console.log(`${id} ok`);
}

type Calls = { undo: number[]; redo: number[]; copied: string[] };

function deps(
  answer: Result<HistoryEntry, string> | "throw",
  copyWorks = true,
) {
  const calls: Calls = { undo: [], redo: [], copied: [] };
  const reply = async () => {
    if (answer === "throw") throw new Error("ipc");
    return answer;
  };
  return {
    calls,
    deps: {
      undo: async (id: number) => {
        calls.undo.push(id);
        return reply();
      },
      redo: async (id: number) => {
        calls.redo.push(id);
        return reply();
      },
      copy: async (text: string) => {
        calls.copied.push(text);
        return copyWorks;
      },
    },
  };
}

// C27 - o texto mostrado e copiado.
{
  assert.equal(shownText(entry(dictation())), "Ação de amanhã.");
  assert.equal(
    shownText(entry(dictation({ showing: "raw" }))),
    "acao de amanha",
  );
  assert.equal(shownText(entry(null)), "texto do history.db");
  assert.ok(
    /onCopyText=\{\(\) => copyToClipboard\(shownText\(entry\)\)\}/.test(screen),
    "o botão de copiar precisa copiar shownText(entry)",
  );
  assert.ok(
    /:\s*hasTranscription\s*\?\s*shownText\(entry\)/.test(screen),
    "o parágrafo do item precisa mostrar shownText(entry)",
  );
  assert.ok(
    !/\?\s*entry\.transcription_text\s*:/.test(screen),
    "a tela ainda mostra entry.transcription_text",
  );
  ok("C27");
}

// C28 - qual botão de edição aparece.
{
  for (const [d, want] of [
    [dictation({ editor: "llm", showing: "final" }), "undo"],
    [dictation({ editor: "llm", showing: "raw" }), "redo"],
    [dictation({ editor: "rules", showing: "final" }), null],
    [dictation({ editor: "none", showing: "final" }), null],
    [null, null],
  ] as const) {
    assert.equal(editAction(entry(d)), want, JSON.stringify(d));
  }
  assert.ok(
    screen.includes("const action = editAction(entry);"),
    "a tela precisa decidir o botão por editAction(entry)",
  );
  assert.ok(
    /\{action && \(/.test(screen),
    "o botão só aparece quando editAction não é nulo",
  );
  assert.ok(screen.includes('t("settings.history.undoAiEdit")'));
  assert.ok(screen.includes('t("settings.history.applyAiEdit")'));
  ok("C28");
}

// C29 - desfazer e reaplicar trocam a entrada, copiam e avisam; a falha não troca nada.
await (async () => {
  const undone = entry(dictation({ showing: "raw" }));
  const u = deps({ status: "ok", data: undone });
  const outU = await switchText(entry(dictation()), "undo", u.deps);
  assert.deepEqual(u.calls.undo, [7]);
  assert.deepEqual(u.calls.redo, []);
  assert.deepEqual(u.calls.copied, ["acao de amanha"]);
  assert.deepEqual(outU, {
    entry: undone,
    success: true,
    toastKey: "settings.history.originalCopied",
  });

  const redone = entry(dictation({ showing: "final" }));
  const r = deps({ status: "ok", data: redone });
  const outR = await switchText(undone, "redo", r.deps);
  assert.deepEqual(r.calls.redo, [7]);
  assert.deepEqual(r.calls.copied, ["Ação de amanhã."]);
  assert.deepEqual(outR, {
    entry: redone,
    success: true,
    toastKey: "settings.history.editedCopied",
  });

  for (const answer of [
    { status: "error", error: "History entry 7 has no dictation" } as const,
    "throw" as const,
  ]) {
    const f = deps(answer);
    const outF = await switchText(entry(dictation()), "undo", f.deps);
    assert.deepEqual(f.calls.copied, []);
    assert.deepEqual(outF, {
      entry: null,
      success: false,
      toastKey: "settings.history.editToggleError",
    });
  }

  const noCopy = deps({ status: "ok", data: undone }, false);
  const outC = await switchText(entry(dictation()), "undo", noCopy.deps);
  assert.deepEqual(outC, {
    entry: undone,
    success: false,
    toastKey: "settings.history.copyError",
  });

  assert.ok(screen.includes("undo: commands.undoHistoryEntryEdit"));
  assert.ok(screen.includes("redo: commands.redoHistoryEntryEdit"));
  assert.ok(screen.includes("copy: copyToClipboard"));
  assert.ok(
    /if \(outcome\.entry\)[\s\S]{0,200}e\.id === replaced\.id \? replaced : e/.test(
      screen,
    ),
    "a tela precisa trocar a entrada pela devolvida",
  );
  assert.ok(
    /outcome\.success\s*\?\s*toast\.success\(t\(outcome\.toastKey\)\)\s*:\s*toast\.error\(t\(outcome\.toastKey\)\)/.test(
      screen,
    ),
    "a tela precisa avisar com o toast do resultado",
  );

  assert.equal(appName(entry(dictation())), "notepad");
  assert.equal(appName(entry(dictation({ app_name: null }))), null);
  assert.equal(appName(entry(null)), null);
  assert.ok(screen.includes("const app = appName(entry);"));
  assert.ok(
    /\{app && \([\s\S]{0,200}t\("settings\.history\.inApp", \{ app \}\)/.test(
      screen,
    ),
    "o app só aparece quando conhecido, por settings.history.inApp",
  );
  ok("C29");
})();

// C30 - as chaves novas nos dois idiomas.
{
  const want: Record<string, [string, string]> = {
    undoAiEdit: ["Desfazer edição da IA", "Undo AI edit"],
    applyAiEdit: ["Aplicar edição da IA", "Apply AI edit"],
    originalCopied: ["Texto original copiado", "Original text copied"],
    editedCopied: ["Texto editado copiado", "Edited text copied"],
    editToggleError: [
      "Não foi possível trocar o texto",
      "Couldn't switch the text",
    ],
    inApp: ["em {{app}}", "in {{app}}"],
  };
  for (const [key, [ptText, enText]] of Object.entries(want)) {
    assert.equal(pt.settings.history[key], ptText, `pt ${key}`);
    assert.equal(en.settings.history[key], enText, `en ${key}`);
  }
  ok("C30");
}

// C2 - "Desfazer edição da IA" só para llm mostrando o final; nada para rules, none ou sem vínculo.
{
  for (const [d, want] of [
    [dictation({ editor: "llm", showing: "final" }), "undo"],
    [dictation({ editor: "rules", showing: "final" }), null],
    [dictation({ editor: "rules", showing: "raw" }), null],
    [dictation({ editor: "none", showing: "final" }), null],
    [dictation({ editor: "none", showing: "raw" }), null],
    [null, null],
  ] as const) {
    assert.equal(editAction(entry(d)), want, JSON.stringify(d));
  }
  ok("C2");
}

// C8 - apply label for redo: llm mostrando o bruto oferece "Aplicar edição da IA".
{
  assert.equal(
    editAction(entry(dictation({ editor: "llm", showing: "raw" }))),
    "redo",
  );
  assert.ok(
    /action === "undo"\s*\?\s*t\("settings\.history\.undoAiEdit"\)\s*:\s*t\("settings\.history\.applyAiEdit"\)/.test(
      screen,
    ),
    "o botão de redo precisa usar settings.history.applyAiEdit",
  );
  assert.equal(pt.settings.history.applyAiEdit, "Aplicar edição da IA");
  assert.equal(en.settings.history.applyAiEdit, "Apply AI edit");
  ok("C8 apply label for redo");
}

// C9 - aplicar a edição tardia troca a entrada e copia; desfazer volta a oferecer "Aplicar".
await (async () => {
  // A edição tardia chegou: o item mostra o bruto colado e tem o texto do LLM como final.
  const late = entry(
    dictation({
      raw_text: "texto colado",
      final_text: "Texto do LLM.",
      showing: "raw",
    }),
  );
  const applied = entry(
    dictation({
      raw_text: "texto colado",
      final_text: "Texto do LLM.",
      showing: "final",
    }),
  );
  const r = deps({ status: "ok", data: applied });
  const outR = await switchText(late, editAction(late)!, r.deps);
  assert.deepEqual(r.calls.redo, [7]);
  assert.deepEqual(r.calls.undo, []);
  assert.deepEqual(r.calls.copied, ["Texto do LLM."]);
  assert.deepEqual(outR, {
    entry: applied,
    success: true,
    toastKey: "settings.history.editedCopied",
  });
  ok("C9 switchText redo");

  const u = deps({ status: "ok", data: late });
  const outU = await switchText(applied, editAction(applied)!, u.deps);
  assert.deepEqual(u.calls.undo, [7]);
  assert.deepEqual(outU.entry, late);
  assert.equal(editAction(outU.entry!), "redo");
  ok("C9 undo after apply offers apply again");
})();

console.log("history: all assertions passed");
