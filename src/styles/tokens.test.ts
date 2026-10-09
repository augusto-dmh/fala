// Prova dos checks dos tokens "Tinta" (`.specs/features/ui-tinta/checks.md`, S1).
// Rode com `bun src/styles/tokens.test.ts`: imprime `<check> ok` e sai com erro na primeira falha.
import assert from "node:assert/strict";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const theme = read("./theme.css");
const app = read("../App.css");
const overlayCss = read("../overlay/RecordingOverlay.css");
const appTsx = read("../App.tsx");
const srcDir = new URL("..", import.meta.url).pathname;

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}
const sources = walk(srcDir).filter(
  (p) => /\.(tsx?|css|html)$/.test(p) && !/\.test\.tsx?$/.test(p),
);
assert.ok(sources.length > 100, "src walk found the sources");

/** Body of the first rule `selector {` at or after `from` (no nested braces). */
function block(css: string, selector: string, from = 0): string {
  const start = css.indexOf(`${selector} {`, from);
  assert.ok(start >= 0, `rule ${selector} exists`);
  const open = css.indexOf("{", start);
  const close = css.indexOf("}", open);
  return css.slice(open + 1, close);
}
function decls(body: string): Map<string, string> {
  const map = new Map<string, string>();
  for (const m of body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) {
    map.set(m[1], m[2].replace(/\s+/g, " ").trim());
  }
  return map;
}
const ok = (id: string) => console.log(`${id} ok`);

// The table of section 3.7: token -> [light, dark].
const PALETTE: Record<string, [string, string]> = {
  "surface-0": ["#f3f3f3", "#202020"],
  "surface-1": ["#ffffff", "#2b2b2b"],
  "surface-2": ["#f7f7f7", "#323232"],
  border: ["#e5e5e5", "#3a3a3a"],
  text: ["#1a1a1a", "#f5f5f5"],
  "text-2": ["#5c5c5c", "#c4c4c4"],
  "text-3": ["#8a8a8a", "#8f8f8f"],
  accent: ["#1a1a1a", "#f5f5f5"],
  "on-accent": ["#ffffff", "#1a1a1a"],
  warn: ["#b45309", "#f5a524"],
  ok: ["#1a7f4b", "#4cc38a"],
  them: ["#8a8a8a", "#8f8f8f"],
  danger: ["#c42b1c", "#ff99a4"],
};
const THEMED = Object.keys(PALETTE);

const rootBlock = decls(block(theme, ":root"));
const darkMediaStart = theme.indexOf("@media (prefers-color-scheme: dark)");
assert.ok(darkMediaStart >= 0, "dark media query exists");
const darkMedia = decls(block(theme, ":root", darkMediaStart));
const forcedLight = decls(block(theme, ':root[data-theme="light"]'));
const forcedDark = decls(block(theme, ':root[data-theme="dark"]'));

// C1: every themed token has its light/dark pair; rec is #e5322d in both; me = ok.
for (const [token, [light, dark]] of Object.entries(PALETTE)) {
  assert.equal(
    rootBlock.get(`--light-color-${token}`),
    light,
    `${token} light`,
  );
  assert.equal(rootBlock.get(`--dark-color-${token}`), dark, `${token} dark`);
}
assert.equal(rootBlock.get("--color-rec"), "#e5322d");
assert.equal(rootBlock.get("--color-me"), "var(--color-ok)");
ok("C1");

// C2: the four theme blocks point every themed token at the right pair.
const expectations: [string, Map<string, string>, "light" | "dark"][] = [
  [":root (default)", rootBlock, "light"],
  ["prefers-color-scheme: dark", darkMedia, "dark"],
  ['data-theme="light"', forcedLight, "light"],
  ['data-theme="dark"', forcedDark, "dark"],
];
for (const [name, map, mode] of expectations) {
  for (const token of THEMED) {
    assert.equal(
      map.get(`--color-${token}`),
      `var(--${mode}-color-${token})`,
      `${name}: --color-${token}`,
    );
  }
}
ok("C2");

// C3: the inherited names are aliases of the ink tokens; no pink is left in src.
const ALIASES: Record<string, string> = {
  background: "surface-1",
  "background-ui": "accent",
  "logo-primary": "accent",
  "logo-stroke": "text",
  "text-stroke": "surface-1",
  "mid-gray": "text-2",
  warning: "warn",
  error: "danger",
};
for (const [alias, target] of Object.entries(ALIASES)) {
  assert.equal(
    rootBlock.get(`--color-${alias}`),
    `var(--color-${target})`,
    `alias ${alias}`,
  );
  for (const map of [darkMedia, forcedLight, forcedDark]) {
    assert.equal(
      map.get(`--color-${alias}`),
      undefined,
      `${alias} not re-pointed`,
    );
  }
}
const PINK = /#(faa2ca|da5893|f28cbb|382731|fad1ed)\b/i;
for (const file of sources) {
  assert.ok(!PINK.test(readFileSync(file, "utf8")), `no pink in ${file}`);
}
ok("C3");

// C4: WCAG contrast of the text pairs, in both themes.
function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}
for (const mode of [0, 1] as const) {
  for (const fg of ["text", "text-2"]) {
    for (const bg of ["surface-0", "surface-1", "surface-2"]) {
      const ratio = contrast(PALETTE[fg][mode], PALETTE[bg][mode]);
      assert.ok(ratio >= 4.5, `${fg} on ${bg} (${mode}) = ${ratio.toFixed(2)}`);
    }
  }
  const onAccent = contrast(PALETTE["on-accent"][mode], PALETTE.accent[mode]);
  assert.ok(onAccent >= 4.5, `on-accent on accent (${mode})`);
}
ok("C4");

// C5: root type and the font stack; no bundled font.
const appRootStart = app.indexOf(":root {\n  /* Typography */");
assert.ok(appRootStart >= 0, "App.css typography root exists");
const appRoot = block(app, ":root", appRootStart);
assert.match(appRoot, /font-size: 14px;/);
assert.match(appRoot, /line-height: 20px;/);
assert.match(appRoot, /font-family: var\(--font-sans\);/);
const themeAt = decls(block(app, "@theme"));
assert.equal(
  themeAt.get("--font-sans"),
  '"Segoe UI Variable Text", "Segoe UI Variable", "Segoe UI", system-ui, sans-serif',
);
assert.equal(
  themeAt.get("--font-display"),
  '"Segoe UI Variable Display", "Segoe UI Variable", "Segoe UI", system-ui, sans-serif',
);
for (const file of sources) {
  const text = readFileSync(file, "utf8");
  assert.ok(!/@font-face/.test(text), `no @font-face in ${file}`);
  assert.ok(
    !/fonts\.googleapis|\.woff2?\b/.test(text),
    `no font file in ${file}`,
  );
}
ok("C5");

// C6: the Windows 11 ramp and the Tailwind sizes mapped onto it.
const RAMP: Record<string, [string, string]> = {
  caption: ["12px", "16px"],
  body: ["14px", "20px"],
  "body-lg": ["18px", "24px"],
  subtitle: ["20px", "28px"],
  title: ["28px", "36px"],
  xs: ["12px", "16px"],
  sm: ["14px", "20px"],
  base: ["14px", "20px"],
  lg: ["18px", "24px"],
  xl: ["20px", "28px"],
  "2xl": ["28px", "36px"],
};
for (const [step, [size, line]] of Object.entries(RAMP)) {
  assert.equal(themeAt.get(`--text-${step}`), size, `text-${step}`);
  assert.equal(
    themeAt.get(`--text-${step}--line-height`),
    line,
    `text-${step} line`,
  );
}
ok("C6");

// C7: radius 4 px on controls (sm, md, lg) and 8 px on overlays (xl).
assert.equal(themeAt.get("--radius-sm"), "4px");
assert.equal(themeAt.get("--radius-md"), "4px");
assert.equal(themeAt.get("--radius-lg"), "4px");
assert.equal(themeAt.get("--radius-xl"), "8px");
ok("C7");

// C8: the Fluent double focus ring, as a base rule and as a utility.
const RING = [
  "outline: 2px solid var(--color-focus-outer);",
  "outline-offset: 1px;",
  "box-shadow: 0 0 0 1px var(--color-focus-inner);",
];
const baseLayerStart = app.indexOf("@layer base");
assert.ok(baseLayerStart >= 0, "@layer base exists");
const focusVisible = block(app, ":focus-visible", baseLayerStart);
const focusUtility = block(app, "@utility focus-ring");
for (const decl of RING) {
  assert.ok(focusVisible.includes(decl), `:focus-visible ${decl}`);
  assert.ok(focusUtility.includes(decl), `focus-ring ${decl}`);
}
assert.equal(rootBlock.get("--color-focus-outer"), "var(--color-text)");
assert.equal(rootBlock.get("--color-focus-inner"), "var(--color-surface-1)");
ok("C8");

// C9: transitions default to 100 ms ease-out; reduced motion stops them.
assert.equal(themeAt.get("--default-transition-duration"), "100ms");
assert.equal(themeAt.get("--ease-fluent"), "cubic-bezier(0, 0, 0, 1)");
const reducedStart = app.indexOf("@media (prefers-reduced-motion: reduce)");
assert.ok(reducedStart >= 0, "reduced-motion block exists");
const reduced = app.slice(reducedStart);
assert.match(reduced, /transition-duration: 0\.01ms !important;/);
assert.match(reduced, /animation-duration: 0\.01ms !important;/);
ok("C9");

// C10: the window sits on surface-0 and the wordmark is ink.
assert.match(appRoot, /background-color: var\(--color-surface-0\);/);
assert.match(block(app, ".logo-primary"), /fill: var\(--color-text\);/);
assert.match(
  read("../components/icons/FalaTextLogo.tsx"),
  /className="logo-primary"/,
);
ok("C10");

// C11: the Live panel's accent is ink and it no longer names the old tokens.
assert.match(overlayCss, /--s-accent: var\(--color-accent\);/);
assert.doesNotMatch(overlayCss, /logo-primary|background-ui|mid-gray/);
ok("C11");

// C12: the Toaster uses the new tokens only.
const toasterStart = appTsx.indexOf("<Toaster");
assert.ok(toasterStart >= 0, "Toaster exists");
const toaster = appTsx.slice(toasterStart, appTsx.indexOf("/>", toasterStart));
for (const cls of [
  "bg-surface-1",
  "text-text",
  "border-border",
  "rounded-xl",
]) {
  assert.ok(toaster.includes(cls), `toast has ${cls}`);
}
assert.match(toaster, /description: "text-text-2"/);
assert.doesNotMatch(
  toaster,
  /logo-primary|background-ui|mid-gray|bg-background\b/,
);
ok("C12");

// C13: no hard-coded white text sits on a solid ink fill.
for (const file of sources.filter((p) => p.endsWith(".tsx"))) {
  for (const line of readFileSync(file, "utf8").split("\n")) {
    if (/\bbg-(logo-primary|background-ui|accent)(?![\w/-])/.test(line)) {
      assert.ok(
        !/\btext-white\b/.test(line),
        `text-white on ink in ${file}: ${line.trim()}`,
      );
    }
  }
}
assert.match(
  read("../components/ui/Badge.tsx"),
  /primary: "bg-accent text-on-accent"/,
);
assert.match(
  read("../components/ui/ToggleSwitch.tsx"),
  /peer-checked:after:bg-on-accent/,
);
assert.doesNotMatch(read("../components/Sidebar.tsx"), /bg-logo-primary\/80/);
ok("C13");
