// Prova dos checks de `.specs/features/qa-dictionary-one-list/checks.md` (Q8 e o Q42 dos termos do
// QA de 2026-10-13).
// Rode com `bun src/components/dictionaryList.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha. Sem instância do i18next, `t` devolve a chave.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { DictionaryList } from "./DictionaryList";
import {
  countByKind,
  draftOf,
  entriesOf,
  filterEntries,
} from "./dictionaryModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);

const entries = entriesOf(
  ["Fala", "Itaú"],
  [{ spoken: "pfv", written: "por favor", uses: 3 }],
  [{ spoken: "minha assinatura", written: "Ana\nana@x.com" }],
);

// D1: uma lista só, com termos, pares e snippets, filtrada por tipo e pela busca (Q8).
{
  assert.deepEqual(
    entries.map((entry) => entry.key),
    ["term:Fala", "term:Itaú", "replacement:pfv", "snippet:minha assinatura"],
  );
  assert.deepEqual(countByKind(entries), {
    all: 4,
    term: 2,
    replacement: 1,
    snippet: 1,
  });
  const keys = (list: typeof entries) => list.map((entry) => entry.key);
  assert.deepEqual(keys(filterEntries(entries, "term", "")), [
    "term:Fala",
    "term:Itaú",
  ]);
  assert.deepEqual(keys(filterEntries(entries, "all", "itau")), ["term:Itaú"]);
  assert.deepEqual(keys(filterEntries(entries, "all", "favor")), [
    "replacement:pfv",
  ]);
  assert.deepEqual(keys(filterEntries(entries, "snippet", "pfv")), []);
  const pair = entries[2];
  assert.ok(pair.kind === "replacement" && pair.uses === 3);
  const snippet = entries[3];
  assert.ok(snippet.kind === "snippet" && snippet.uses === 0);
  ok("D1");
}

// D2: o formulário de "Adicionar" aplica as regras de cada tipo (Q8).
{
  assert.deepEqual(draftOf("term", "  São  Paulo ", "", entries), {
    ok: true,
    kind: "term",
    spoken: "São Paulo",
    written: "",
  });
  assert.deepEqual(draftOf("term", "fala", "", entries), {
    ok: false,
    reason: "duplicate",
    spoken: "fala",
  });
  assert.equal(draftOf("term", "<>", "", entries).ok, false);
  assert.equal(draftOf("replacement", "btw", "", entries).ok, false);
  assert.deepEqual(draftOf("replacement", "btw", " by the way ", entries), {
    ok: true,
    kind: "replacement",
    spoken: "btw",
    written: "by the way",
  });
  assert.deepEqual(draftOf("replacement", "PFV", "x", entries), {
    ok: false,
    reason: "duplicate",
    spoken: "PFV",
  });
  // Um termo e um par podem ter o mesmo texto: são listas diferentes no backend.
  assert.equal(draftOf("replacement", "fala", "Fala", entries).ok, true);
  assert.deepEqual(draftOf("snippet", "assinatura", "x", entries), {
    ok: false,
    reason: "twoWords",
    spoken: "assinatura",
  });
  assert.equal(draftOf("snippet", "meu endereço", "Rua A", entries).ok, true);
  assert.equal(draftOf("replacement", "a".repeat(61), "x", entries).ok, false);
  ok("D2");
}

// D3: ao remover pelo teclado, o foco vai ao item seguinte, senão ao anterior, senão ao
// "Adicionar" (ou ao campo, com o formulário aberto); Enter só adiciona a partir de um campo;
// trocar de tipo limpa o segundo campo; o repetido diz por que "Adicionar" está desligado.
{
  const list = read("./DictionaryList.tsx");
  assert.match(
    list,
    /neighbourAfterRemoval\(\s*visible\.map\(\(e\) => e\.key\),\s*entry\.key,?\s*\)/,
  );
  assert.match(
    list,
    /const fallback = \(\) => addRef\.current \?\? spokenRef\.current;/,
  );
  assert.match(list, /target instanceof HTMLInputElement \|\|/);
  assert.match(
    list,
    /setWritten\(""\);\s*setSpoken\(\(text\) => text\.slice\(0, LIMITS\[next\]\.maxSpoken\)\);/,
  );
  assert.match(list, /draft\.reason === "duplicate"/);
  assert.match(list, /tabIndex=\{checked \? 0 : -1\}/);
  ok("D3");
}

// D4: a página usa a lista única; o vazio convida a adicionar; os textos novos existem em pt e en.
{
  const page = read("./DictionaryPage.tsx");
  assert.match(page, /<DictionaryList filter=\{query\} \/>/);
  assert.doesNotMatch(page, /CustomWords|DictionaryReplacements/);
  const html = renderToStaticMarkup(<DictionaryList filter="" />);
  assert.match(html, /role="radiogroup"/);
  assert.match(html, />dictionary\.list\.add</);
  assert.match(html, /dictionary\.list\.empty/);
  for (const lang of ["pt", "en"]) {
    const list = JSON.parse(read(`../i18n/locales/${lang}/translation.json`))
      .dictionary.list;
    for (const key of ["add", "close", "empty", "noMatch", "kind", "filter"])
      assert.ok(list[key], `${lang} ${key}`);
    for (const kind of ["all", "term", "replacement", "snippet"])
      assert.ok(list.filters[kind].includes("{{count}}"), `${lang} ${kind}`);
  }
  ok("D4");
}

console.log("dictionary list: all assertions passed");
