// Prova dos checks dos estilos por app (`.specs/features/app-styles/checks.md`). Rode com
// `bun src/components/settings/appStyles.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createInstance } from "i18next";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nextProvider, initReactI18next } from "react-i18next";
import type { AppSettings, WritingStyle } from "@/bindings";
import { useSettingsStore } from "../../stores/settingsStore";
import { AppStyles } from "./AppStyles";
import { newStyleId, setAppStyle } from "./appStylesModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const ok = (id: string) => console.log(`${id} ok`);

const style = (id: string, name: string, instruction = ""): WritingStyle => ({
  id,
  name,
  instruction,
});

// C5: a new id never repeats one in use, and mapping an app twice keeps one entry in place.
{
  assert.equal(newStyleId([]), "style-1");
  assert.equal(newStyleId([style("style-2", "a"), style("x", "b")]), "style-3");
  assert.equal(newStyleId([style("style-2", "a")]), "style-3");
  const once = setAppStyle([], " Chrome ", "email");
  assert.deepEqual(once, [{ app: "chrome", style: "email" }]);
  const twice = setAppStyle(
    [...once, { app: "slack", style: "message" }],
    "CHROME",
    "document",
  );
  assert.deepEqual(twice, [
    { app: "chrome", style: "document" },
    { app: "slack", style: "message" },
  ]);
  assert.deepEqual(setAppStyle(once, "  ", "email"), once);
}
ok("C5");

// C6: the page shows each style with its instruction and each mapping by the style's name; a
// mapping to a removed style is not shown.
{
  const i18n = createInstance();
  void i18n.use(initReactI18next).init({
    lng: "pt",
    resources: { pt: { translation: pt } },
    initAsync: false,
    interpolation: { escapeValue: false },
  });
  useSettingsStore.setState({
    isLoading: false,
    settings: {
      writing_styles: [
        style("email", "E-mail", "Cordial e em parágrafos."),
        style("mine", "Meu estilo", "Assine Augusto."),
      ],
      app_styles: [
        { app: "chrome", style: "mine" },
        { app: "slack", style: "gone" },
      ],
    } as unknown as AppSettings,
  });
  const html = renderToStaticMarkup(
    <I18nextProvider i18n={i18n}>
      <AppStyles />
    </I18nextProvider>,
  );
  assert.equal(html.match(/<li /g)?.length, 2, "one row per style");
  assert.ok(!html.includes("slack"), "a mapping to a removed style is hidden");
  for (const piece of [
    'value="E-mail"',
    "Cordial e em parágrafos.",
    'value="Meu estilo"',
    "Assine Augusto.",
    "chrome → Meu estilo",
    pt.settings.llm.appStyles.title,
    pt.settings.llm.styles.title,
  ]) {
    assert.ok(html.includes(piece), piece);
  }
  assert.ok(
    read("./LlmSettings.tsx").includes("<AppStyles />"),
    "on the AI settings, after the apps without AI",
  );
}
ok("C6");

// C7: the same keys in pt and en, none empty.
{
  const keys = (node: unknown, prefix = ""): string[] =>
    typeof node === "object" && node !== null
      ? Object.entries(node).flatMap(([k, v]) => keys(v, `${prefix}.${k}`))
      : [prefix];
  for (const section of ["appStyles", "styles"]) {
    const ptKeys = keys(pt.settings.llm[section]).sort();
    assert.deepEqual(keys(en.settings.llm[section]).sort(), ptKeys, section);
    for (const tree of [pt, en]) {
      for (const value of Object.values(tree.settings.llm[section])) {
        assert.ok(typeof value === "string" && value.trim(), section);
      }
    }
  }
}
ok("C7");
