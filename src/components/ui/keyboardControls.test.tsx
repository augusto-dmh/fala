// Prova dos checks de `.specs/features/qa-keyboard-controls/checks.md` (Q33, Q34 e Q35 do QA de
// 2026-10-13).
// Rode com `bun src/components/ui/keyboardControls.test.tsx`: imprime `<check> ok` e sai com erro
// na primeira falha. Sem instância do i18next, `t` devolve a chave.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { Dropdown, stepOption } from "./Dropdown";
import { ResetButton } from "./ResetButton";
import { SettingContainer } from "./SettingContainer";
import { ToggleSwitch } from "./ToggleSwitch";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};

// K1: as setas andam pelas opções habilitadas e dão a volta; Home/End vão às pontas.
{
  const options = [
    { disabled: false },
    { disabled: true },
    { disabled: false },
    { disabled: false },
  ];
  assert.equal(stepOption(options, -1, "ArrowDown"), 0);
  assert.equal(stepOption(options, -1, "ArrowUp"), 3);
  assert.equal(stepOption(options, 0, "ArrowDown"), 2, "pula a desabilitada");
  assert.equal(stepOption(options, 2, "ArrowUp"), 0);
  assert.equal(stepOption(options, 3, "ArrowDown"), 0, "volta ao começo");
  assert.equal(stepOption(options, 0, "ArrowUp"), 3, "volta ao fim");
  assert.equal(stepOption(options, 2, "Home"), 0);
  assert.equal(stepOption(options, 0, "End"), 3);
  assert.equal(stepOption(options, 0, "a"), null);
  assert.equal(stepOption([{ disabled: true }], -1, "ArrowDown"), null);
  assert.equal(stepOption([], -1, "End"), null);
  ok("K1");
}

// K2: dentro de uma linha de configuração, o botão se chama pelo título da linha e pelo valor e
// anuncia a lista (fechada); Esc, setas, Tab, Enter e Espaço estão ligados.
{
  const html = renderToStaticMarkup(
    <SettingContainer title="Microfone" description="">
      <Dropdown
        options={[{ value: "a", label: "Fone" }]}
        selectedValue="a"
        onSelect={noop}
      />
    </SettingContainer>,
  );
  const titleId = /<h3 class="[^"]*" id="([^"]+)"/.exec(html)?.[1];
  const valueId = /<span id="([^"]+)" class="truncate">Fone</.exec(html)?.[1];
  assert.ok(titleId && valueId, html);
  assert.ok(html.includes(`aria-labelledby="${titleId} ${valueId}"`), html);
  assert.match(html, /aria-haspopup="listbox"/);
  assert.match(html, /aria-expanded="false"/);
  assert.doesNotMatch(html, /aria-controls/, "fechada, não aponta a lista");
  const alone = renderToStaticMarkup(
    <Dropdown options={[]} selectedValue={null} onSelect={noop} />,
  );
  assert.doesNotMatch(alone, /aria-labelledby/, "fora de uma linha, o valor");

  const src = read("./Dropdown.tsx");
  assert.match(src, /role="listbox"/);
  assert.match(src, /role="option"/);
  assert.match(src, /aria-selected=\{index === selectedIndex\}/);
  assert.match(
    src,
    /event\.key === "Escape" && isOpen\) \{\s*event\.preventDefault\(\);\s*event\.stopPropagation\(\);\s*close\(true\)/,
    "Esc no botão fecha só a lista",
  );
  assert.match(src, /if \(refocus\) triggerRef\.current\?\.focus\(\)/);
  assert.match(src, /event\.key === "Tab" && isOpen\) \{\s*close\(false\)/);
  assert.match(src, /event\.key === "Tab"\) \{\s*close\(false\)/);
  assert.match(src, /event\.detail === 0 \? startValue\("ArrowDown"\)/);
  assert.match(src, /setFocusValue\(start\);\s*focusOption\(start\);/);
  assert.match(src, /\[isOpen, focusValue, options\]/);
  ok("K2");
}

// K3: o ↻ tem nome e dica ("Restaurar o padrão"), ou o rótulo que o uso der.
{
  const plain = renderToStaticMarkup(<ResetButton onClick={noop} />);
  assert.match(plain, /aria-label="common\.resetToDefault"/);
  assert.match(plain, /title="common\.resetToDefault"/);
  const named = renderToStaticMarkup(
    <ResetButton onClick={noop} ariaLabel="Restaurar atalho" />,
  );
  assert.match(named, /aria-label="Restaurar atalho"/);
  for (const lang of ["pt", "en"]) {
    const locale = JSON.parse(
      read(`../../i18n/locales/${lang}/translation.json`),
    );
    assert.ok(locale.common.resetToDefault, lang);
  }
  ok("K3");
}

// K4: o toggle é um switch com o nome do título.
{
  const html = renderToStaticMarkup(
    <ToggleSwitch
      checked
      onChange={noop}
      label="Silenciar durante o ditado"
      description="d"
    />,
  );
  assert.match(
    html,
    /<input[^>]*type="checkbox"[^>]*role="switch"[^>]*aria-label="Silenciar durante o ditado"/,
  );
  ok("K4");
}

console.log("keyboard controls: all assertions passed");
