// Prova dos checks dos controles Fluent (`.specs/features/ui-tinta/checks.md`, S2).
// Rode com `bun src/components/ui/controls.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha.
/* eslint-disable i18next/no-literal-string -- fixtures, not UI copy */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { Button } from "./Button";
import { Input } from "./Input";
import { SettingContainer } from "./SettingContainer";
import { SettingsGroup } from "./SettingsGroup";
import { selectStyles } from "./Select";
import { Slider } from "./Slider";
import { ToggleSwitch } from "./ToggleSwitch";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};

/** Every class attribute in the markup, split into tokens. */
const classes = (html: string): string[] =>
  [...html.matchAll(/class="([^"]*)"/g)].flatMap((m) =>
    m[1]
      .replace(/&gt;/g, ">")
      .replace(/&lt;/g, "<")
      .replace(/&amp;/g, "&")
      .split(/\s+/),
  );
const has = (html: string, cls: string) => classes(html).includes(cls);
const text = (html: string) => html.replace(/<[^>]+>/g, "");

const ROW_FILES = [
  "./SettingsGroup.tsx",
  "./SettingContainer.tsx",
  "./Button.tsx",
  "./ToggleSwitch.tsx",
  "./Input.tsx",
  "./Select.tsx",
  "./Dropdown.tsx",
  "./Slider.tsx",
];

// C15: SettingsGroup draws a sentence-case subtitle and one 4 px card per row.
{
  const html = renderToStaticMarkup(
    <SettingsGroup title="Som" description="Microfone e saída">
      <div>a</div>
      <div>b</div>
    </SettingsGroup>,
  );
  assert.match(html, /<h2 class="[^"]*">Som<\/h2>/);
  for (const cls of ["text-body", "font-semibold", "text-text"]) {
    assert.ok(has(html, cls), `subtitle ${cls}`);
  }
  assert.ok(!has(html, "uppercase"), "no uppercase");
  assert.ok(!has(html, "tracking-wide"), "no tracking");
  assert.match(
    html,
    /<p class="[^"]*text-caption[^"]*text-text-2[^"]*">Microfone e saída<\/p>/,
  );
  for (const cls of [
    "gap-1",
    "[&>*]:bg-surface-1",
    "[&>*]:border",
    "[&>*]:border-border",
    "[&>*]:rounded-lg",
  ]) {
    assert.ok(has(html, cls), `rows container ${cls}`);
  }
  const bare = renderToStaticMarkup(
    <SettingsGroup>
      <div>a</div>
    </SettingsGroup>,
  );
  assert.ok(!bare.includes("<h2"), "no subtitle without title");
}
ok("C15");

// C16: the description is always visible below the title, in text-2, in both
// description modes, and there is no "?" tooltip trigger any more.
for (const mode of ["tooltip", "inline"] as const) {
  for (const layout of ["horizontal", "stacked"] as const) {
    const html = renderToStaticMarkup(
      <SettingContainer
        title="Atalho"
        description="Segure para falar"
        descriptionMode={mode}
        layout={layout}
        grouped
      >
        <span>ctl</span>
      </SettingContainer>,
    );
    assert.match(
      html,
      /<h3 class="[^"]*text-body[^"]*">Atalho<\/h3>/,
      `${mode}/${layout} title`,
    );
    assert.match(
      html,
      /<p class="[^"]*text-caption[^"]*text-text-2[^"]*">Segure para falar<\/p>/,
      `${mode}/${layout} description`,
    );
    assert.ok(!html.includes("cursor-help"), `${mode}/${layout} no ? icon`);
    assert.ok(
      !html.includes('role="button"'),
      `${mode}/${layout} no tooltip trigger`,
    );
    assert.ok(text(html).includes("ctl"), `${mode}/${layout} control rendered`);
  }
}
assert.doesNotMatch(read("./SettingContainer.tsx"), /Tooltip/);
ok("C16");

// C17: row heights 56 px without a description and 68 px with one; an optional
// 20 px icon sits left of the title; alone (not grouped) the row is its own card.
{
  const withDesc = renderToStaticMarkup(
    <SettingContainer title="T" description="D" grouped>
      <span />
    </SettingContainer>,
  );
  assert.ok(has(withDesc, "min-h-[68px]") && !has(withDesc, "min-h-[56px]"));
  const noDesc = renderToStaticMarkup(
    <SettingContainer title="T" description="" grouped>
      <span />
    </SettingContainer>,
  );
  assert.ok(has(noDesc, "min-h-[56px]") && !has(noDesc, "min-h-[68px]"));
  assert.ok(!noDesc.includes("<p"), "no empty description paragraph");
  const withIcon = renderToStaticMarkup(
    <SettingContainer
      title="T"
      description="D"
      icon={<svg data-icon="x" />}
      grouped
    >
      <span />
    </SettingContainer>,
  );
  const iconAt = withIcon.indexOf('data-icon="x"');
  assert.ok(
    iconAt >= 0 && iconAt < withIcon.indexOf("<h3"),
    "icon before title",
  );
  assert.ok(has(withIcon, "w-5") && has(withIcon, "h-5"), "20 px icon box");
  assert.ok(!has(withDesc, "border"), "grouped row draws no card");
  const alone = renderToStaticMarkup(
    <SettingContainer title="T" description="D">
      <span />
    </SettingContainer>,
  );
  for (const cls of ["bg-surface-1", "border", "border-border", "rounded-lg"]) {
    assert.ok(has(alone, cls), `standalone card ${cls}`);
  }
}
ok("C17");

// C18: Button keeps its 7 variants and 3 sizes; ink primary, 4 px radius, double
// focus ring, 32 px default height, no old token.
{
  const variants = [
    "primary",
    "primary-soft",
    "secondary",
    "warning",
    "danger",
    "danger-ghost",
    "ghost",
  ] as const;
  for (const variant of variants) {
    const html = renderToStaticMarkup(<Button variant={variant}>x</Button>);
    for (const cls of [
      "rounded-lg",
      "focus-visible:focus-ring",
      "min-h-[32px]",
    ]) {
      assert.ok(has(html, cls), `${variant} ${cls}`);
    }
    assert.doesNotMatch(
      html,
      /logo-primary|background-ui|mid-gray|text-white/,
      variant,
    );
  }
  const primary = renderToStaticMarkup(<Button>x</Button>);
  assert.ok(has(primary, "bg-accent") && has(primary, "text-on-accent"));
  const sizes: Record<string, string> = {
    sm: "min-h-[24px]",
    md: "min-h-[32px]",
    lg: "min-h-[40px]",
  };
  for (const [size, cls] of Object.entries(sizes)) {
    const html = renderToStaticMarkup(
      <Button size={size as "sm" | "md" | "lg"}>x</Button>,
    );
    assert.ok(has(html, cls), `size ${size}`);
  }
  const passthrough = renderToStaticMarkup(
    <Button className="extra" type="submit" aria-label="lbl">
      x
    </Button>,
  );
  assert.ok(has(passthrough, "extra") && passthrough.includes('type="submit"'));
}
ok("C18");

// C19: ToggleSwitch is the Fluent 40x20 toggle: outlined when off, ink when on,
// with the double focus ring on the hidden checkbox's peer.
{
  const html = renderToStaticMarkup(
    <ToggleSwitch
      checked
      onChange={noop}
      label="Ligar"
      description="Liga a IA"
      grouped
    />,
  );
  for (const cls of [
    "w-[40px]",
    "h-[20px]",
    "rounded-full",
    "border-text-2",
    "peer-checked:bg-accent",
    "peer-checked:after:bg-on-accent",
    "after:bg-text-2",
    "peer-focus-visible:focus-ring",
    "peer-checked:after:translate-x-[20px]",
    "rtl:peer-checked:after:-translate-x-[20px]",
  ]) {
    assert.ok(has(html, cls), `toggle ${cls}`);
  }
  assert.match(html, /<p class="[^"]*text-text-2[^"]*">Liga a IA<\/p>/);
  assert.ok(html.includes('type="checkbox"') && html.includes("checked"));
}
ok("C19");

// C20: Input: 4 px radius, ink underline on focus, regular weight, same variants.
{
  for (const variant of ["default", "compact"] as const) {
    const html = renderToStaticMarkup(
      <Input variant={variant} placeholder="p" />,
    );
    for (const cls of [
      "rounded-md",
      "bg-surface-1",
      "border-border",
      "text-body",
      "focus:border-b-accent",
    ]) {
      assert.ok(has(html, cls), `${variant} ${cls}`);
    }
    assert.ok(!has(html, "font-semibold"), `${variant} regular weight`);
  }
  const disabled = renderToStaticMarkup(<Input disabled />);
  assert.ok(
    has(disabled, "opacity-50") && !has(disabled, "focus:border-b-accent"),
  );
}
ok("C20");

// C21: Select (react-select) styles: 32 px, 4 px radius, ink underline when
// focused, 8 px menu on surface-1, no old token anywhere in the style table.
{
  const base = {} as never;
  const control = (focused: boolean) =>
    (selectStyles.control as any)(base, { isFocused: focused });
  assert.equal(control(false).minHeight, 32);
  assert.equal(control(false).borderRadius, 4);
  assert.equal(control(true).borderBottomColor, "var(--color-accent)");
  assert.equal(control(false).borderBottomColor, "var(--color-text-3)");
  assert.equal(control(false).backgroundColor, "var(--color-surface-1)");
  const menu = (selectStyles.menu as any)(base, {});
  assert.equal(menu.borderRadius, 8);
  assert.equal(menu.backgroundColor, "var(--color-surface-1)");
  assert.doesNotMatch(
    read("./Select.tsx"),
    /logo-primary|mid-gray|--color-background\b/,
  );
}
ok("C21");

// C22: Dropdown: trigger like a Fluent combo box (32 px, 4 px, double focus
// ring), 8 px menu on surface-1 with a 3 px ink marker on the selected option.
{
  const src = read("./Dropdown.tsx");
  for (const cls of [
    "min-h-[32px]",
    "rounded-md",
    "focus-visible:focus-ring",
    "rounded-xl",
    "bg-surface-1",
    "before:w-[3px]",
    "before:bg-accent",
  ]) {
    assert.ok(src.includes(cls), `dropdown ${cls}`);
  }
  assert.doesNotMatch(src, /logo-primary|mid-gray|bg-background\b/);
}
ok("C22");

// C23: Slider: 4 px rail with the ink fill, 20 px thumb, double focus ring.
{
  const html = renderToStaticMarkup(
    <Slider
      value={0.5}
      onChange={noop}
      min={0}
      max={1}
      label="Volume"
      description="Do som"
      grouped
    />,
  );
  for (const cls of [
    "h-[4px]",
    "rounded-full",
    "focus-visible:focus-ring",
    "[&::-webkit-slider-thumb]:w-[20px]",
    "[&::-webkit-slider-thumb]:h-[20px]",
    "[&::-webkit-slider-thumb]:bg-accent",
    "tabular-nums",
  ]) {
    assert.ok(has(html, cls), `slider ${cls}`);
  }
  assert.match(html, /linear-gradient\(to right, var\(--color-accent\) 50%/);
  assert.match(html, /<p class="[^"]*text-text-2[^"]*">Do som<\/p>/);
}
ok("C23");

// C24: none of the eight restyled files names an old token any more.
for (const file of ROW_FILES) {
  assert.doesNotMatch(
    read(file),
    /logo-primary|background-ui|mid-gray|bg-background\b|--color-background\b/,
    file,
  );
}
ok("C24");
