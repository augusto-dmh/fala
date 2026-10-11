// Prova dos checks dos cards de nível de limpeza (`.specs/features/cleanup-level/checks.md`,
// S2). Rode com `bun src/components/settings/cleanupLevel.test.tsx`: imprime `<check> ok` e sai
// com erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createInstance } from "i18next";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nextProvider, initReactI18next } from "react-i18next";
import type { CleanupLevel } from "@/bindings";
import { CleanupLevelCards } from "./CleanupLevelCards";
import { CLEANUP_LEVELS, levelForKey } from "./cleanupLevelModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};

const i18n = createInstance();
void i18n.use(initReactI18next).init({
  lng: "pt",
  resources: { pt: { translation: pt }, en: { translation: en } },
  initAsync: false,
  interpolation: { escapeValue: false },
});

const decode = (html: string) =>
  html
    .replace(/&quot;/g, '"')
    .replace(/&#x27;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");

const render = (value: CleanupLevel, dimmed: boolean) =>
  decode(
    renderToStaticMarkup(
      <I18nextProvider i18n={i18n}>
        <CleanupLevelCards value={value} onChange={noop} dimmed={dimmed} />
      </I18nextProvider>,
    ),
  );

/** Each `<button ...>...</button>` radio, keyed by its `data-level`. */
const radios = (html: string) =>
  [...html.matchAll(/<button([^>]*)>(.*?)<\/button>/g)].map((m) => ({
    attrs: m[1],
    inner: m[2],
    level: m[1].match(/data-level="(\w+)"/)?.[1],
  }));
const text = (html: string) => html.replace(/<[^>]+>/g, "");
const cleanup = pt.settings.llm.cleanup;

// C6: one radiogroup, four radios in order, each with label, description and example; the raw
// dictation is shown once.
{
  const html = render("light", false);
  assert.equal(html.match(/role="radiogroup"/g)?.length, 1, "one radiogroup");
  const cards = radios(html);
  assert.deepEqual(
    cards.map((c) => c.level),
    ["verbatim", "light", "medium", "heavy"],
  );
  assert.deepEqual(CLEANUP_LEVELS, ["verbatim", "light", "medium", "heavy"]);
  for (const card of cards) {
    assert.match(card.attrs, /role="radio"/, `${card.level} role`);
    const level = cleanup.levels[card.level!];
    const body = text(card.inner);
    for (const field of ["label", "description", "example"]) {
      assert.ok(body.includes(level[field]), `${card.level} ${field}`);
    }
  }
  assert.equal(
    [...cards.map((c) => text(c.inner))].join("").includes(cleanup.raw),
    false,
    "the raw dictation is not repeated inside the cards",
  );
  assert.ok(text(html).includes(cleanup.raw), "raw dictation shown");
  assert.ok(text(html).includes(cleanup.title), "title");
  assert.match(html, /aria-labelledby="([^"]+)"[^>]*>/);
  const labelledBy = html.match(/aria-labelledby="([^"]+)"/)![1];
  assert.ok(html.includes(`id="${labelledBy}"`), "group is labelled");
}
ok("C6");

// C7: only the stored level is checked, in the tab order and drawn with the accent border.
for (const value of CLEANUP_LEVELS) {
  for (const card of radios(render(value, false))) {
    const checked = card.level === value;
    assert.match(
      card.attrs,
      new RegExp(`aria-checked="${checked}"`),
      `${value}: ${card.level} aria-checked`,
    );
    assert.match(
      card.attrs,
      new RegExp(`tabindex="${checked ? 0 : -1}"`),
      `${value}: ${card.level} tabindex`,
    );
    const classes = card.attrs.match(/class="([^"]*)"/)![1].split(/\s+/);
    assert.equal(
      classes.includes("border-accent"),
      checked,
      `${value}: ${card.level} border-accent`,
    );
  }
}
ok("C7");

// C8: arrows move and wrap, Home/End jump, other keys do nothing.
{
  const table: [CleanupLevel, string, CleanupLevel | null][] = [
    ["verbatim", "ArrowRight", "light"],
    ["light", "ArrowDown", "medium"],
    ["heavy", "ArrowRight", "verbatim"],
    ["heavy", "ArrowDown", "verbatim"],
    ["medium", "ArrowLeft", "light"],
    ["light", "ArrowUp", "verbatim"],
    ["verbatim", "ArrowLeft", "heavy"],
    ["verbatim", "ArrowUp", "heavy"],
    ["medium", "Home", "verbatim"],
    ["light", "End", "heavy"],
    ["light", "Enter", null],
    ["light", " ", null],
    ["light", "Tab", null],
  ];
  for (const [current, key, expected] of table) {
    assert.equal(levelForKey(current, key), expected, `${current} ${key}`);
  }
}
ok("C8");

// C9: dimmed shows the inactive line and lowers the opacity, but the radios stay usable.
{
  const dimmed = render("light", true);
  const active = render("light", false);
  const groupClasses = (html: string) =>
    html.match(/role="radiogroup"[^>]*class="([^"]*)"/)![1].split(/\s+/);
  assert.ok(groupClasses(dimmed).includes("opacity-60"), "dimmed opacity");
  assert.ok(!groupClasses(active).includes("opacity-60"), "active opacity");
  assert.ok(text(dimmed).includes(cleanup.inactive), "inactive line");
  assert.ok(!text(active).includes(cleanup.inactive), "no inactive line");
  for (const card of radios(dimmed)) {
    assert.doesNotMatch(card.attrs, /disabled/, `${card.level} not disabled`);
  }
}
ok("C9");

// C10: the footer, the pt copy from the definition, and the cards right below the AI toggle.
{
  assert.ok(text(render("light", false)).includes(cleanup.footer), "footer");
  assert.equal(
    cleanup.footer,
    "O original nunca se perde: Desfazer edição da IA, no Início.",
  );
  assert.ok(
    cleanup.inactive.includes("ale quando a IA formata o ditado"),
    cleanup.inactive,
  );
  const settings = read("./LlmSettings.tsx");
  assert.match(
    settings,
    /<ToggleSwitch[\s\S]*?\/>\s*<CleanupLevelCards\b[\s\S]*?dimmed=\{!enabled \|\| !apiKey\}[\s\S]*?\/>\s*<SettingContainer/,
  );
  assert.match(
    settings,
    /onChange=\{\(level\) => updateSetting\("cleanup_level", level\)\}/,
  );
}
ok("C10");

// C11: the same keys in pt and en, and the pt examples from the definition.
{
  const keys = (node: unknown, prefix = ""): string[] =>
    typeof node === "object" && node !== null
      ? Object.entries(node).flatMap(([k, v]) => keys(v, `${prefix}.${k}`))
      : [prefix];
  assert.deepEqual(
    keys(en.settings.llm.cleanup).sort(),
    keys(pt.settings.llm.cleanup).sort(),
  );
  for (const [locale, tree] of [
    ["pt", pt],
    ["en", en],
  ] as const) {
    for (const key of keys(tree.settings.llm.cleanup)) {
      const value = key
        .split(".")
        .slice(1)
        .reduce((n: any, k) => n[k], tree.settings.llm.cleanup);
      assert.ok(typeof value === "string" && value.trim(), `${locale}${key}`);
    }
  }
  const levels = cleanup.levels;
  assert.equal(
    cleanup.raw,
    "é... então, tipo, a reunião vai ser na terça, não, na quarta às três, e eu queria que vocês trouxessem os números né",
  );
  assert.deepEqual(
    [levels.verbatim, levels.light, levels.medium, levels.heavy].map(
      (l) => l.label,
    ),
    ["Literal", "Leve", "Média", "Forte"],
  );
  assert.equal(
    levels.verbatim.example,
    "É... então, tipo, a reunião vai ser na terça, não, na quarta às três, e eu queria que vocês trouxessem os números, né?",
  );
  assert.equal(
    levels.light.example,
    "A reunião vai ser na quarta às três, e eu queria que vocês trouxessem os números.",
  );
  assert.equal(
    levels.medium.example,
    "A reunião será na quarta, às três. Queria que vocês trouxessem os números.",
  );
  assert.equal(
    levels.heavy.example,
    "Reunião na quarta às 15h; tragam os números.",
  );
}
ok("C11");
