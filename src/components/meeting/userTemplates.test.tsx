// Prova dos templates do usuário na UI (`meeting-user-templates`): os dois grupos no seletor, a
// busca acima de 5, a lista em Reuniões com [Abrir a pasta], [Duplicar para editar] só nos
// embutidos e o arquivo quebrado visível, e a releitura ao voltar o foco. Rode com
// `bun src/components/meeting/userTemplates.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingTemplate } from "@/bindings";
import {
  filterTemplates,
  shownTemplate,
  TEMPLATE_SEARCH_ABOVE,
  TemplateSelect,
} from "./TemplateSelect";
import { TemplatesSettings } from "./TemplatesSettings";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const page = read("./MeetingsPage.tsx");
const view = read("./MeetingView.tsx");
const recorder = read("./RecorderPanel.tsx");

function ok(id: string) {
  console.log(`${id} ok`);
}

const noop = () => {};

const template = (
  id: string,
  name: string,
  source: MeetingTemplate["source"],
  purpose = "",
): MeetingTemplate => ({ id, name, purpose, source });

const builtin = [
  template("geral", "Reunião geral", "builtin", "Notas de trabalho"),
  template("um-a-um", "Um a um", "builtin", "Conversa de gestão"),
];
const vendas = template("vendas", "Vendas", "user", "Ligação com cliente");

// Os dois grupos só quando há template do usuário; sem ele, a lista de antes.
{
  const plain = renderToStaticMarkup(
    <TemplateSelect templates={builtin} value="geral" onChange={noop} />,
  );
  assert.ok(!plain.includes("<optgroup"), plain);
  assert.ok(plain.includes('value="um-a-um"'));
  const grouped = renderToStaticMarkup(
    <TemplateSelect
      templates={[...builtin, vendas]}
      value="vendas"
      onChange={noop}
    />,
  );
  const groups = [...grouped.matchAll(/<optgroup label="([^"]+)">/g)].map(
    (m) => m[1],
  );
  assert.equal(groups.length, 2, grouped);
  const first = grouped.indexOf("<optgroup");
  const second = grouped.indexOf("<optgroup", first + 1);
  assert.ok(first < grouped.indexOf('value="geral"'), grouped);
  assert.ok(grouped.indexOf('value="um-a-um"') < second, grouped);
  assert.ok(second < grouped.indexOf('value="vendas"'), grouped);
  // Grupo vazio não aparece; um template que sumiu da pasta dá lugar ao primeiro.
  const onlyUser = renderToStaticMarkup(
    <TemplateSelect templates={[vendas]} value="vendas" onChange={noop} />,
  );
  assert.equal(onlyUser.split("<optgroup").length, 2, onlyUser);
  assert.equal(shownTemplate(builtin, "vendas"), "geral");
  assert.equal(shownTemplate(builtin, "um-a-um"), "um-a-um");
  assert.equal(shownTemplate(builtin, null), "geral");
  assert.equal(shownTemplate([], "vendas"), "vendas");
  assert.match(view, /props\.onGenerate\(picked, false\)/);
  // Os dois seletores (rascunho/nota e gravação) usam o mesmo componente.
  assert.match(view, /<TemplateSelect/);
  assert.match(recorder, /<TemplateSelect/);
  ok("UT-groups");
}

// Busca por nome e propósito, sem acento e sem maiúscula, só acima de 5; o escolhido fica.
{
  const many = [
    ...builtin,
    vendas,
    template("retro", "Retrospectiva", "user"),
    template("diaria", "Diária", "user"),
    template("entrevista", "Entrevista", "user", "Contratação"),
  ];
  assert.equal(many.length, TEMPLATE_SEARCH_ABOVE + 1);
  const ids = (list: MeetingTemplate[]) => list.map((t) => t.id);
  assert.deepEqual(ids(filterTemplates(many, "DIARIA", "geral")), [
    "geral",
    "diaria",
  ]);
  assert.deepEqual(ids(filterTemplates(many, "contratacao", "retro")), [
    "retro",
    "entrevista",
  ]);
  assert.deepEqual(filterTemplates(many, "  ", "geral"), many);
  const searchable = renderToStaticMarkup(
    <TemplateSelect templates={many} value="geral" onChange={noop} />,
  );
  assert.ok(searchable.includes('type="search"'), searchable);
  const few = renderToStaticMarkup(
    <TemplateSelect
      templates={many.slice(0, TEMPLATE_SEARCH_ABOVE)}
      value="geral"
      onChange={noop}
    />,
  );
  assert.ok(!few.includes('type="search"'));
  ok("UT-search");
}

// Reuniões: [Abrir a pasta], [Duplicar para editar] em cada embutido e só neles, o arquivo
// inválido como "arquivo.json: erro"; a página relê os templates ao voltar o foco.
{
  const html = renderToStaticMarkup(
    <TemplatesSettings
      catalog={{
        templates: [...builtin, vendas],
        errors: [{ file: "quebrado.json", error: "EOF while parsing" }],
      }}
      onChange={noop}
    />,
  );
  const buttons = [...html.matchAll(/<button[^>]*>([^<]*)<\/button>/g)].map(
    (m) => m[1],
  );
  assert.equal(buttons.length, 3, html);
  assert.ok(html.includes("Vendas"));
  assert.ok(html.includes('role="alert"'));
  assert.ok(html.includes("quebrado.json"));
  assert.ok(html.includes("EOF while parsing"));
  const clean = renderToStaticMarkup(
    <TemplatesSettings
      catalog={{ templates: builtin, errors: [] }}
      onChange={noop}
    />,
  );
  assert.ok(!clean.includes('role="alert"'));
  assert.match(page, /<TemplatesSettings catalog=\{catalog\}/);
  assert.match(page, /addEventListener\("focus", load\)/);
  assert.match(page, /removeEventListener\("focus", load\)/);
  ok("UT-settings");
}

// Toda string nova em pt e en.
{
  assert.deepEqual(
    Object.keys(en.meeting.templates),
    Object.keys(pt.meeting.templates),
  );
  for (const key of Object.keys(pt.meeting.templates)) {
    for (const locale of [pt, en]) {
      const value = locale.meeting.templates[key];
      assert.equal(typeof value, "string", key);
      assert.ok(value.length > 0, key);
    }
  }
  assert.equal(pt.meeting.templates.duplicate, "Duplicar para editar");
  assert.equal(pt.meeting.templates.open, "Abrir a pasta");
  ok("UT-i18n");
}
