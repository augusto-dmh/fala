// Prova dos checks de `.specs/features/dictionary-import/checks.md` (importar o dicionário de um
// arquivo, com prévia).
// Rode com `bun src/components/dictionaryImport.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha. Sem instância do i18next, `t` devolve a chave.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { entriesOf } from "./dictionaryModel";
import {
  importedLists,
  MAX_LINES,
  pickSeparator,
  previewImport,
  splitFields,
} from "./dictionaryImport";
import { ImportPreviewBody } from "./ImportDictionaryDialog";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);

const stored = {
  terms: ["Fala", "Itaú"],
  replacements: [{ spoken: "pfv", written: "por favor", uses: 3 }],
};
const entries = entriesOf(stored.terms, stored.replacements, [
  { spoken: "minha assinatura", written: "Ana" },
]);

// I1: uma entrada por linha; os três separadores, aspas, BOM, cabeçalho, comentários e as linhas
// inválidas com o número.
{
  for (const separator of [",", ";", "\t"]) {
    const preview = previewImport(
      `Supabase\nbtw${separator} by the way \n  fala   sql ${separator}Fala.sqlite`,
      entries,
    );
    assert.deepEqual(
      preview.added,
      [
        { kind: "term", text: "Supabase" },
        { kind: "replacement", spoken: "btw", written: "by the way" },
        { kind: "replacement", spoken: "fala sql", written: "Fala.sqlite" },
      ],
      JSON.stringify(separator),
    );
    assert.deepEqual(preview.invalid, []);
  }
  // O separador é o do arquivo: um `;` no escrito de um CSV com vírgulas é texto.
  assert.equal(pickSeparator(["a,b", "c,d; e", "f"]), ",");
  assert.equal(pickSeparator(["a;b", "c;d"]), ";");
  assert.equal(pickSeparator(["termo"]), ",");
  assert.deepEqual(previewImport("a,b\nc,d; e", []).added[1], {
    kind: "replacement",
    spoken: "c",
    written: "d; e",
  });
  // Aspas CSV simples.
  assert.deepEqual(splitFields('"a, b",c', ","), ["a, b", "c"]);
  assert.deepEqual(splitFields('x, "diz ""oi""" ', ","), ["x", 'diz "oi"']);
  assert.deepEqual(splitFields("termo,", ","), ["termo"]);
  assert.equal(splitFields('"aberto,c', ","), null);
  assert.equal(splitFields('"a"b,c', ","), null);
  assert.deepEqual(previewImport('"Itaú, S.A."\n"a, b",c', []).added, [
    { kind: "term", text: "Itaú, S.A." },
    { kind: "replacement", spoken: "a, b", written: "c" },
  ]);
  // BOM, CRLF, linhas vazias, comentários e cabeçalho.
  const file =
    "\uFEFFfalado;escrito\r\n# meus pares\r\n\r\n   \r\nobg;obrigado\r\n";
  assert.deepEqual(previewImport(file, []), {
    error: null,
    added: [{ kind: "replacement", spoken: "obg", written: "obrigado" }],
    duplicates: 0,
    invalid: [],
  });
  for (const header of ["term", "Termo", "spoken,written", "Falado\tEscrito"])
    assert.equal(previewImport(`${header}\nAna`, []).added.length, 1, header);
  // O cabeçalho só vale na primeira linha de entrada.
  assert.equal(previewImport("Ana\ntermo", []).added.length, 2);
  // Inválidas: campos demais, aspas abertas, campo vazio, acima do limite do tipo.
  const bad = previewImport(
    [
      "a,b,c",
      '"aberto',
      ",x",
      "x,",
      "<>",
      "y".repeat(51),
      `${"s".repeat(61)},w`,
      "ok",
    ].join("\n"),
    [],
  );
  assert.deepEqual(bad.invalid, [
    { line: 1, reason: "fields" },
    { line: 2, reason: "quote" },
    { line: 3, reason: "empty" },
    { line: 5, reason: "empty" },
    { line: 6, reason: "tooLong" },
    { line: 7, reason: "tooLong" },
  ]);
  // `x,` perde o campo vazio do fim e vira termo.
  assert.deepEqual(bad.added, [
    { kind: "term", text: "x" },
    { kind: "term", text: "ok" },
  ]);
  ok("I1");
}

// I2: duplicados sem caixa; tetos de linhas e de campo; gravar acrescenta só os novos.
{
  const preview = previewImport(
    "fala\nITAÚ\nPFV,por favor\nNovo\nnovo\nPFV2,x\npfv2,y\nfala,Fala",
    entries,
  );
  assert.equal(preview.duplicates, 5);
  assert.deepEqual(preview.added, [
    { kind: "term", text: "Novo" },
    { kind: "replacement", spoken: "PFV2", written: "x" },
    // Um par pode ter o falado igual a um termo: são listas diferentes.
    { kind: "replacement", spoken: "fala", written: "Fala" },
  ]);
  // O snippet não conta como par repetido.
  assert.equal(previewImport("minha assinatura,Ana", entries).added.length, 1);

  const many = Array.from({ length: MAX_LINES }, (_, i) => `t${i}`);
  assert.equal(previewImport(many.join("\n"), []).added.length, MAX_LINES);
  const over = previewImport(
    ["# só comentário", ...many, "mais"].join("\n"),
    [],
  );
  assert.deepEqual(over.error, { kind: "lines", count: MAX_LINES + 1 });
  assert.deepEqual(over.added, []);
  const long = previewImport(`ok\nx,${"w".repeat(201)}\nmais`, []);
  assert.deepEqual(long.error, { kind: "field", line: 2 });
  assert.deepEqual(long.added, []);
  assert.equal(previewImport(`x,${"w".repeat(200)}`, []).error, null);

  assert.deepEqual(importedLists(preview, stored.terms, stored.replacements), {
    terms: ["Fala", "Itaú", "Novo"],
    replacements: [
      { spoken: "pfv", written: "por favor", uses: 3 },
      { spoken: "PFV2", written: "x", uses: 0 },
      { spoken: "fala", written: "Fala", uses: 0 },
    ],
  });
  assert.deepEqual(
    importedLists(previewImport("Fala", entries), stored.terms, []),
    { terms: null, replacements: null },
  );
  assert.deepEqual(importedLists(long, stored.terms, stored.replacements), {
    terms: null,
    replacements: null,
  });
  ok("I2");
}

// I3: a prévia mostra as contagens, as inválidas e as primeiras entradas; Cancelar só fecha;
// Importar grava e avisa.
{
  const preview = previewImport(
    "Fala\nNovo\nobg,obrigado\na,b,c\n" +
      Array.from({ length: 6 }, (_, i) => `x${i},y,z`).join("\n"),
    entries,
  );
  const html = renderToStaticMarkup(<ImportPreviewBody preview={preview} />);
  for (const key of ["terms", "replacements", "duplicates", "invalid"])
    assert.match(html, new RegExp(`dictionary\\.import\\.counts\\.${key}`));
  assert.equal(html.match(/dictionary\.import\.invalidLine/g)?.length, 5);
  assert.match(html, /dictionary\.import\.first/);
  assert.match(html, />Novo</);
  assert.match(html, />obg → obrigado</);

  const error = renderToStaticMarkup(
    <ImportPreviewBody preview={previewImport(`${"w".repeat(201)}`, [])} />,
  );
  assert.match(error, /dictionary\.import\.fieldTooLong/);
  assert.doesNotMatch(error, /counts/);
  const nothing = renderToStaticMarkup(
    <ImportPreviewBody preview={previewImport("Fala", entries)} />,
  );
  assert.match(nothing, /dictionary\.import\.nothing/);

  const dialog = read("./ImportDictionaryDialog.tsx");
  assert.match(dialog, /const cancel = \(\) => setFile\(null\);/);
  assert.match(dialog, /onClick=\{cancel\}/);
  assert.match(dialog, /disabled=\{count === 0 \|\| saving \|\| disabled\}/);
  assert.match(
    dialog,
    /if \(lists\.terms\) await updateSetting\("custom_words", lists\.terms\);/,
  );
  assert.match(
    dialog,
    /await updateSetting\("dictionary_replacements", lists\.replacements\);/,
  );
  // O toast conta o que o store guardou: uma gravação que falhou volta atrás sem lançar.
  assert.match(dialog, /const count = stored\(\) - before;/);
  assert.match(dialog, /toast\.success\(t\("dictionary\.import\.done"/);
  assert.match(dialog, /toast\.error\(t\("dictionary\.import\.saveError"/);
  assert.match(
    read("../stores/settingsStore.ts"),
    /const result = await commands\.updateCustomWords\(value as string\[\]\);\s*if \(result\.status === "error"\) \{\s*throw/,
  );
  // Arquivo que não é UTF-8 ou grande demais não vira prévia.
  assert.match(dialog, /new TextDecoder\("utf-8", \{ fatal: true \}\)/);
  assert.match(dialog, /\(await stat\(path\)\)\.size > MAX_BYTES/);
  ok("I3");
}

// I4: "Importar…" na lista; o seletor filtra `.txt`, `.csv` e `.tsv`; textos em pt e en.
{
  assert.match(
    read("./DictionaryList.tsx"),
    /<ImportDictionary entries=\{entries\} disabled=\{busy\} \/>/,
  );
  assert.match(
    read("./ImportDictionaryDialog.tsx"),
    /extensions: \["txt", "csv", "tsv"\]/,
  );
  for (const lang of ["pt", "en"]) {
    const keys = JSON.parse(read(`../i18n/locales/${lang}/translation.json`))
      .dictionary.import;
    for (const key of [
      "open",
      "title",
      "files",
      "format",
      "readError",
      "tooBig",
      "saveError",
      "invalidLine",
      "tooManyLines",
      "fieldTooLong",
      "first",
      "nothing",
      "cancel",
      "confirm",
      "done_one",
      "done_other",
    ])
      assert.ok(keys[key], `${lang} ${key}`);
    for (const kind of ["terms", "replacements", "duplicates", "invalid"])
      assert.ok(keys.counts[`${kind}_other`].includes("{{count}}"));
    for (const reason of ["fields", "quote", "empty", "tooLong"])
      assert.ok(keys.reasons[reason], `${lang} ${reason}`);
  }
  ok("I4");
}

console.log("dictionary import: all assertions passed");
