// Templates do usuário na UI (`meeting-user-templates`). Rode com
// `bun src/components/meeting/userTemplates.test.tsx`: imprime `<check> ok`.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { MeetingTemplate } from "@/bindings";
import {
  filterTemplates,
  shownTemplate,
  TEMPLATE_SEARCH_ABOVE,
  TemplateMenu,
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
  const groups = (html: string) =>
    [...html.matchAll(/role="group" aria-label="([^"]+)"/g)].map((m) => m[1]);
  const plain = renderToStaticMarkup(
    <TemplateMenu templates={builtin} value="geral" onChange={noop} />,
  );
  assert.deepEqual(groups(plain), ["meeting.view.template"], plain);
  const grouped = renderToStaticMarkup(
    <TemplateMenu
      templates={[...builtin, vendas]}
      value="vendas"
      onChange={noop}
    />,
  );
  assert.deepEqual(
    groups(grouped),
    ["meeting.templates.builtin", "meeting.templates.user"],
    grouped,
  );
  const second = grouped.indexOf('aria-label="meeting.templates.user"');
  assert.ok(grouped.indexOf("Um a um") < second, grouped);
  assert.ok(second < grouped.indexOf("Vendas"), grouped);
  // Grupo vazio não aparece; um template que sumiu da pasta dá lugar ao primeiro.
  const onlyUser = renderToStaticMarkup(
    <TemplateMenu templates={[vendas]} value="vendas" onChange={noop} />,
  );
  assert.deepEqual(groups(onlyUser), ["meeting.templates.user"], onlyUser);
  // Without templates the menu says so instead of a lone heading.
  assert.ok(
    renderToStaticMarkup(
      <TemplateMenu templates={[]} value="" onChange={noop} />,
    ).includes("common.noOptionsFound"),
  );
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
    <TemplateMenu templates={many} value="geral" onChange={noop} />,
  );
  assert.ok(searchable.includes('type="search"'), searchable);
  const few = renderToStaticMarkup(
    <TemplateMenu
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
  // The list lives in Settings > Meetings; the page and Settings each read the catalog.
  assert.match(
    read("./TemplatesSettings.tsx"),
    /useMeetingTemplates\(\);[\s\S]*<TemplatesSettings\s+catalog=\{catalog\}/,
  );
  assert.match(
    read("../settings/general/GeneralSettings.tsx"),
    /<MeetingTemplatesSettings \/>/,
  );
  assert.match(page, /const \[catalog\] = useMeetingTemplates\(\);/);
  const hook = read("./useMeetingTemplates.ts");
  assert.match(hook, /addEventListener\("focus", load\)/);
  assert.match(hook, /removeEventListener\("focus", load\)/);
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

// QA Q29: the templates are the items of the "Generate notes" arrow, not a select in a popover:
// one checked, the groups as headings, the search above five, and a pick closes the menu.
{
  const html = renderToStaticMarkup(
    <TemplateMenu
      templates={[...builtin, vendas]}
      value="vendas"
      onChange={noop}
    />,
  );
  assert.ok(!html.includes("<select"), html);
  assert.equal(html.match(/role="menuitemradio"/g)?.length, 3, html);
  assert.equal(html.match(/aria-checked="true"/g)?.length, 1, html);
  assert.match(html, /aria-checked="true"[^>]*>[\s\S]*?Vendas</);
  const headings = [
    ...html.matchAll(/<p aria-hidden="true" class="px-3[^"]*">([^<]+)<\/p>/g),
  ].map((m) => m[1]);
  assert.deepEqual(headings, [
    "meeting.templates.builtin",
    "meeting.templates.user",
  ]);
  const many = Array.from({ length: TEMPLATE_SEARCH_ABOVE + 1 }, (_, i) =>
    template(`t${i}`, `T${i}`, "builtin"),
  );
  assert.ok(
    renderToStaticMarkup(
      <TemplateMenu templates={many} value="t0" onChange={noop} />,
    ).includes('type="search"'),
  );
  const menu = readFileSync(
    new URL("./TemplateSelect.tsx", import.meta.url),
    "utf8",
  );
  assert.match(
    menu,
    /props\.onChange\(template\.id\);\s*props\.onPicked\?\.\(\);/,
  );
  const bar = readFileSync(
    new URL("./SessionBar.tsx", import.meta.url),
    "utf8",
  );
  assert.match(bar, /\{props\.templateMenu\(close\)\}/);
  // A pick or Esc closes from inside and gives the focus back to the button.
  assert.match(
    menu,
    /if \(inside\) \{\s*box\.current\s*\?\.querySelector<HTMLButtonElement>\('\[aria-haspopup="menu"\]'\)\s*\?\.focus\(\);/,
  );
  // The draft and the live recorder use the same menu behind a button: no native select left.
  for (const file of ["./TemplateSelect.tsx", "./RecorderPanel.tsx"]) {
    assert.ok(
      !readFileSync(new URL(file, import.meta.url), "utf8").includes("<select"),
      file,
    );
  }
  assert.match(bar, /aria-haspopup="menu"/);
  ok("Q29");
}
