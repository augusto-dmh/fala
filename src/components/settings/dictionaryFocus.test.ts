// Prova dos checks de `.specs/features/qa-dictionary-focus/checks.md` (Q7 do QA de 2026-10-13).
// Rode com `bun src/components/settings/dictionaryFocus.test.ts`: imprime `<check> ok` e sai com
// erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  entryButton,
  neighbourAfterRemoval,
  settleFocus,
  type FocusTarget,
} from "../../hooks/useFocusWhenIdle";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);

// F1: o foco pedido espera a gravação acabar e só então vai ao alvo, e só se o foco se perdeu
// (ficou no body); se a pessoa já foi para outro campo, fica lá.
{
  const focused: string[] = [];
  const target: FocusTarget = () =>
    ({ focus: () => focused.push("vizinho") }) as unknown as HTMLElement;
  const body = {} as Element;
  const search = {} as Element;
  assert.equal(settleFocus(target, true, body, body), target, "ocupado");
  assert.deepEqual(focused, []);
  assert.equal(settleFocus(target, false, search, body), null);
  assert.deepEqual(focused, [], "a busca tem o foco: não rouba");
  assert.equal(settleFocus(target, false, body, body), null);
  assert.deepEqual(focused, ["vizinho"]);
  assert.equal(settleFocus(target, false, null, body), null);
  assert.deepEqual(focused, ["vizinho", "vizinho"]);
  assert.equal(settleFocus(null, false, body, body), null);
  assert.equal(
    settleFocus(() => null, false, body, body),
    null,
  );
  ok("F1");
}

// F2: ao remover, o alvo é o item que toma o lugar, senão o anterior, senão o campo.
{
  assert.equal(neighbourAfterRemoval(["a", "b", "c"], "b"), "c");
  assert.equal(neighbourAfterRemoval(["a", "b", "c"], "c"), "b");
  assert.equal(neighbourAfterRemoval(["a"], "a"), null);
  assert.equal(neighbourAfterRemoval(["a"], "x"), null);
  const buttons = ["café", 'diz "oi"'].map((entry) => ({ dataset: { entry } }));
  const list = {
    querySelectorAll: (selector: string) => {
      assert.equal(selector, "[data-entry]");
      return buttons;
    },
  } as unknown as HTMLElement;
  assert.equal(entryButton(list, 'diz "oi"'), buttons[1]);
  assert.equal(entryButton(list, "chá"), undefined);
  assert.equal(entryButton(null, "café"), undefined);
  ok("F2");
}

// F3: os campos ficam habilitados durante a gravação (o foco e as teclas seguintes ficam neles);
// adicionar espera a gravação; a remoção pelo teclado leva o foco ao vizinho, com o campo de
// reserva; o clique não mexe no foco.
{
  const words = read("./CustomWords.tsx");
  assert.match(words, /if \(busy\) return;/);
  const field = /<Input\s+ref=\{inputRef\}[\s\S]*?\/>/.exec(words)?.[0] ?? "";
  assert.ok(field && !field.includes("disabled"), field);
  assert.match(words, /neighbourAfterRemoval\(visibleWords, wordToRemove\)/);
  assert.match(
    words,
    /entryButton\(listRef\.current, next\) \?\? inputRef\.current/,
  );
  assert.match(words, /handleRemoveWord\(word, event\.detail === 0\)/);
  assert.match(words, /data-entry=\{word\}/);

  const pairs = read("./DictionaryReplacements.tsx");
  assert.match(pairs, /!busy;/, "canAdd espera a gravação");
  assert.doesNotMatch(pairs, /variant="compact"\s+disabled=\{busy\}/);
  assert.match(pairs, /spokenRef\.current\?\.focus\(\);/);
  assert.match(
    pairs,
    /entryButton\(listRef\.current, next\) \?\? spokenRef\.current/,
  );
  assert.match(pairs, /handleRemove\(pair\.spoken, event\.detail === 0\)/);
  assert.match(pairs, /data-entry=\{pair\.spoken\}/);
  assert.match(read("../ui/Input.tsx"), /React\.forwardRef<HTMLInputElement/);
  ok("F3");
}

console.log("dictionary focus: all assertions passed");
