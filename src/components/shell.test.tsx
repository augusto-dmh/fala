// Prova dos checks da casca com rail (`.specs/features/ui-tinta/checks.md`, S3).
// Rode com `bun src/components/shell.test.tsx`: imprime `<check> ok` e sai com erro na primeira
// falha.
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { RAIL_ITEMS, Rail } from "./Rail";
import {
  resolveSettingsView,
  visibleAdvancedPages,
  type SettingsView,
} from "./settingsNav";
import { UPDATER_ENABLED } from "../lib/updater";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};
const classes = (html: string): string[] =>
  [...html.matchAll(/class="([^"]*)"/g)].flatMap((m) =>
    m[1]
      .replace(/&gt;/g, ">")
      .replace(/&lt;/g, "<")
      .replace(/&amp;/g, "&")
      .split(/\s+/),
  );
const has = (html: string, cls: string) => classes(html).includes(cls);
/** The opening tag of the first <button> whose aria-label is `label`. */
const button = (html: string, label: string): string => {
  const m = html.match(new RegExp(`<button[^>]*aria-label="${label}"[^>]*>`));
  assert.ok(m, `button ${label}`);
  return m[0];
};

const pt = JSON.parse(read("../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../i18n/locales/en/translation.json"));
const appTsx = read("../App.tsx");

// i18next is not initialised here, so `t` returns the key: labels show as keys.
const render = (active: "home" | "dictionary" | "settings") =>
  renderToStaticMarkup(
    <Rail active={active} onSelect={noop} onOpenModels={noop} />,
  );

// C25: the rail has Início and Dicionário on top and Configurações in its
// footer, 220 px wide collapsing to 48 px of icons below 840 px, with the
// wordmark in ink beside the pill in miniature.
{
  assert.deepEqual(
    RAIL_ITEMS.map((item) => item.id),
    ["home", "dictionary"],
  );
  const html = render("home");
  assert.match(html, /<nav[^>]*aria-label="rail.navigation"/);
  for (const cls of ["w-[220px]", "max-[840px]:w-[48px]"]) {
    assert.ok(has(html, cls), `nav ${cls}`);
  }
  const home = html.indexOf('aria-label="rail.home"');
  const dict = html.indexOf('aria-label="rail.dictionary"');
  const settings = html.indexOf('aria-label="rail.settings"');
  const footer = html.indexOf("mt-auto");
  assert.ok(home > 0 && home < dict && dict < footer && footer < settings);
  // The classes are read on the element itself, not anywhere in the rail.
  const wordmark = html.match(/<span class="([^"]*)">Fala<\/span>/);
  assert.ok(wordmark, "wordmark span");
  const wordmarkClasses = classes(wordmark[0]);
  for (const cls of [
    "text-subtitle",
    "font-semibold",
    "text-text",
    "max-[840px]:hidden",
  ]) {
    assert.ok(wordmarkClasses.includes(cls), `wordmark ${cls}`);
  }
  assert.ok(has(html, "fill-black") && has(html, "fill-white"), "pill mark");
  for (const id of ["home", "dictionary", "settings"]) {
    const label = html.match(
      new RegExp(`<span class="([^"]*)">rail\\.${id}</span>`),
    );
    assert.ok(label, `label ${id}`);
    assert.ok(
      classes(label[0]).includes("max-[840px]:hidden"),
      `label ${id} hides when collapsed`,
    );
    const tag = button(html, `rail.${id}`);
    const after = html.slice(html.indexOf(tag) + tag.length);
    const icon = after.slice(0, after.indexOf("</svg>"));
    assert.match(icon, /^<svg[^>]*width="20"/, `icon ${id} 20 px`);
    assert.match(icon, /stroke-width="1.5"/, `icon ${id} stroke 1.5`);
  }
}
ok("C25");

// C26: exactly the active destination is marked: aria-current, subtle fill and
// a 3 px ink bar; the others are not.
for (const active of ["home", "dictionary", "settings"] as const) {
  const html = render(active);
  for (const id of ["home", "dictionary", "settings"]) {
    const tag = button(html, `rail.${id}`);
    const tagClasses = classes(tag);
    const isActive = id === active;
    assert.equal(
      tag.includes('aria-current="page"'),
      isActive,
      `${active}/${id} aria-current`,
    );
    for (const cls of ["bg-text/5", "before:w-[3px]", "before:bg-accent"]) {
      assert.equal(
        tagClasses.includes(cls),
        isActive,
        `${active}/${id} ${cls}`,
      );
    }
    assert.ok(!tag.includes("logo-primary"), "no pink");
  }
}
ok("C26");

// C27: the app opens on Início; the old sidebar and footer are gone.
assert.match(appTsx, /useState<RailDestination>\("home"\)/);
assert.match(appTsx, /destination === "home" && <HomePage \/>/);
assert.match(appTsx, /destination === "dictionary" && <DictionaryPage \/>/);
assert.doesNotMatch(appTsx, /Sidebar|Footer|SECTIONS_CONFIG/);
assert.ok(
  !existsSync(new URL("./Sidebar.tsx", import.meta.url)),
  "Sidebar.tsx removed",
);
assert.ok(!existsSync(new URL("./footer", import.meta.url)), "footer removed");
assert.match(read("./HomePage.tsx"), /<HistorySettings \/>/);
ok("C27");

// C28: Settings navigation: Models always under Advanced, post-processing and
// Debug only when switched on, and an unreachable page falls back to Advanced.
{
  const combos: [boolean, boolean, string[]][] = [
    [false, false, ["models"]],
    [true, false, ["models", "postprocessing"]],
    [false, true, ["models", "debug"]],
    [true, true, ["models", "postprocessing", "debug"]],
  ];
  for (const [post, debug, pages] of combos) {
    assert.deepEqual(
      visibleAdvancedPages({ post_process_enabled: post, debug_mode: debug }),
      pages,
      `post=${post} debug=${debug}`,
    );
  }
  assert.deepEqual(visibleAdvancedPages(null), ["models"]);
  const off = { post_process_enabled: false, debug_mode: false };
  const on = { post_process_enabled: true, debug_mode: true };
  const cases: [SettingsView, typeof off, string, string, string | null][] = [
    ["general", off, "general", "general", null],
    ["advanced", off, "advanced", "advanced", null],
    ["about", off, "about", "about", null],
    ["models", off, "models", "advanced", "models"],
    ["postprocessing", on, "postprocessing", "advanced", "postprocessing"],
    ["debug", on, "debug", "advanced", "debug"],
    ["postprocessing", off, "advanced", "advanced", null],
    ["debug", off, "advanced", "advanced", null],
  ];
  for (const [view, settings, current, tab, subPage] of cases) {
    assert.deepEqual(
      resolveSettingsView(view, settings),
      { current, tab, subPage },
      view,
    );
  }
  const page = read("./SettingsPage.tsx");
  assert.match(page, /role="tablist"/);
  assert.match(page, /role="tab"/);
  assert.match(page, /aria-selected=\{selected\}/);
  for (const key of [
    "settingsPage.tabs.general",
    "settingsPage.tabs.advanced",
    "settingsPage.tabs.about",
  ]) {
    assert.ok(page.includes(key), key);
  }
  assert.match(page, /current === "models" && <ModelsSettings \/>/);
  assert.match(
    page,
    /current === "postprocessing" && <PostProcessingSettings \/>/,
  );
  assert.match(page, /current === "debug" && \(\s*<DebugSettings/);
  assert.match(page, /current === "about" && <AboutSettings \/>/);
  assert.match(page, /current === "general" && <GeneralSettings \/>/);
}
ok("C28");

// C29: the dictionary is its own destination and CustomWords lives only there.
assert.match(read("./DictionaryPage.tsx"), /<CustomWords grouped \/>/);
assert.doesNotMatch(
  read("./settings/advanced/AdvancedSettings.tsx"),
  /CustomWords/,
);
ok("C29");

// C30: the rail footer shows the model state only (no dropdown, no chevron), and
// clicking it opens Settings > Advanced > Models.
{
  const selector = read("./model-selector/ModelSelector.tsx");
  assert.match(selector, /variant === "status"/);
  const statusBranch = selector.slice(
    selector.indexOf('if (variant === "status")'),
    selector.indexOf("Model Status and Switcher"),
  );
  assert.ok(statusBranch.includes("onOpen?.()"), "status click opens");
  assert.ok(!statusBranch.includes("ModelDropdown"), "no dropdown in status");
  assert.ok(statusBranch.includes("compact"), "compact button");
  const statusButton = read("./model-selector/ModelStatusButton.tsx");
  assert.match(statusButton, /\{!compact && \(/);
  assert.match(statusButton, /max-\[840px\]:sr-only/);
  assert.doesNotMatch(statusButton, /logo-primary/);
  assert.match(
    read("./Rail.tsx"),
    /<ModelSelector variant="status" onOpen=\{onOpenModels\} \/>/,
  );
  assert.match(
    appTsx,
    /setDestination\("settings"\);\s*setSettingsView\("models"\);/,
  );
}
ok("C30");

// C31: release notes and the update checker stay hidden while the updater is off.
{
  assert.equal(UPDATER_ENABLED, false);
  assert.match(
    appTsx,
    /\{UPDATER_ENABLED && \(\s*<ErrorBoundary context="What's New">/,
  );
  assert.match(
    read("./settings/about/AboutSettings.tsx"),
    /\{UPDATER_ENABLED && \(\s*<ShowWhatsNewOnUpdate/,
  );
  const debug = read("./settings/debug/DebugSettings.tsx");
  assert.match(debug, /\{UPDATER_ENABLED && \(\s*<WhatsNewPreview/);
  assert.match(debug, /\{UPDATER_ENABLED && \(\s*<UpdateChecksToggle/);
}
ok("C31");

// C32: the new strings exist in pt (source) and en; the sidebar block is gone.
{
  const expectPt = {
    "rail.navigation": "Navegação principal",
    "rail.home": "Início",
    "rail.dictionary": "Dicionário",
    "rail.settings": "Configurações",
    "settingsPage.tabs.general": "Geral",
    "settingsPage.tabs.advanced": "Avançado",
    "settingsPage.tabs.about": "Sobre",
    "settingsPage.advancedPages.title": "Mais opções",
    "settingsPage.advancedPages.models.title": "Modelos",
    "settingsPage.advancedPages.postProcessing.title": "Pós-processamento",
    "settingsPage.advancedPages.debug.title": "Depuração",
  };
  const expectEn = {
    "rail.home": "Home",
    "rail.dictionary": "Dictionary",
    "rail.settings": "Settings",
    "settingsPage.tabs.advanced": "Advanced",
  };
  const get = (obj: Record<string, unknown>, key: string) =>
    key
      .split(".")
      .reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], obj);
  for (const [key, value] of Object.entries(expectPt))
    assert.equal(get(pt, key), value, `pt ${key}`);
  for (const [key, value] of Object.entries(expectEn))
    assert.equal(get(en, key), value, `en ${key}`);
  for (const key of [
    "dictionary.description",
    "settingsPage.advancedPages.models.description",
    "settingsPage.advancedPages.postProcessing.description",
    "settingsPage.advancedPages.debug.description",
  ]) {
    assert.equal(typeof get(pt, key), "string", `pt ${key}`);
    assert.equal(typeof get(en, key), "string", `en ${key}`);
  }
  assert.equal(pt.sidebar, undefined);
  assert.equal(en.sidebar, undefined);
}
ok("C32");
