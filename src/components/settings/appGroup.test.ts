// Prova do check de `.specs/features/qa-language-theme-general/checks.md` (Q32 do QA de
// 2026-10-13): idioma do app e tema saem de Sobre e vão para Geral, num grupo "Aplicativo".
// Rode com `bun src/components/settings/appGroup.test.ts`: imprime `<check> ok` e sai com erro na
// primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");

const general = read("./general/GeneralSettings.tsx");
const about = read("./about/AboutSettings.tsx");

assert.match(
  general,
  /<SettingsGroup title=\{t\("settings\.general\.app"\)\}>\s*<AppLanguageSelector[^>]*\/>\s*<ThemeSelector[^>]*\/>\s*<\/SettingsGroup>/,
);
assert.doesNotMatch(about, /AppLanguageSelector|ThemeSelector/);
for (const lang of ["pt", "en"]) {
  const locale = JSON.parse(
    read(`../../i18n/locales/${lang}/translation.json`),
  );
  assert.ok(locale.settings.general.app, lang);
}
console.log("A1 ok");
