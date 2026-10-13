// Prova dos checks de `.specs/features/qa-steady-text-areas/checks.md` (Q10, Q11 e o gutter
// do Q54 do QA de 2026-10-13).
// Rode com `bun src/components/ui/steadyLayout.test.tsx`: imprime `<check> ok` e sai com erro
// na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { SearchField } from "./SearchField";
import { Textarea } from "./Textarea";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};
const classes = (html: string, tag: string): string[] =>
  (new RegExp(`<${tag}[^>]*class="([^"]*)"`).exec(html)?.[1] ?? "").split(
    /\s+/,
  );

// S1: o texto de um Textarea tem o peso normal, como o dos inputs.
{
  const area = classes(
    renderToStaticMarkup(<Textarea value="" onChange={noop} />),
    "textarea",
  );
  assert.ok(area.includes("font-normal"), area.join(" "));
  assert.ok(!area.includes("font-semibold"));
  ok("S1");
}

// S2: as buscas têm a lupa, num campo de busca com rótulo.
{
  const html = renderToStaticMarkup(
    <SearchField
      className="w-full"
      value=""
      onChange={noop}
      aria-label="Buscar"
    />,
  );
  assert.match(html, /^<div class="relative w-full">/);
  assert.match(html, /<svg[^>]*aria-hidden="true"/);
  assert.match(html, /<input[^>]*type="search"/);
  assert.ok(classes(html, "input").includes("ps-8"));
  assert.match(html, /aria-label="Buscar"/);
  // Desabilitado, a lupa apaga junto com o campo.
  const off = renderToStaticMarkup(<SearchField disabled aria-label="x" />);
  assert.match(off, /<svg[^>]*class="[^"]*opacity-50/);
  // Toda busca que filtra uma lista, menos a do Início (o PR do Início mexe nela).
  for (const page of [
    "../DictionaryPage.tsx",
    "../meeting/MeetingsPage.tsx",
    "../meeting/MeetingView.tsx",
    "../meeting/TemplateSelect.tsx",
    "../settings/models/ModelsSettings.tsx",
  ]) {
    const source = read(page);
    assert.match(source, /<SearchField\b/, page);
    assert.doesNotMatch(source, /<(Input|input)\b[^>]*type="search"/, page);
  }
  ok("S2");
}

// S3: o `main` reserva a calha nos dois lados; páginas curtas e longas ficam alinhadas e
// centradas, como no onboarding.
{
  assert.match(
    read("../../App.css"),
    /main \{\s*scrollbar-gutter: stable both-edges;\s*\}/,
  );
  ok("S3");
}

console.log("steady layout: all assertions passed");
