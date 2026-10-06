// Prova dos checks da pill (`.specs/features/pill-redesign/checks.md`), do vermelho no modo
// padrão de dois toques (`.specs/features/shortcut-gestures/checks.md`, C24) e do estado âmbar
// do limite de sessão (`.specs/features/session-limit/checks.md`, C18).
// Rode com `bun src/overlay/pill.test.tsx`: imprime `<check> ok` e sai com erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { Pill, type PillProps } from "./Pill";
import { isHoldToTalk, pillBars, pillTone, toPillMode } from "./pillModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const css = read("./Pill.css");
const overlaySource = read("./RecordingOverlay.tsx");

const zeros = Array(16).fill(0);
const loud = Array(16).fill(1);

function render(props: Partial<PillProps>): string {
  return renderToStaticMarkup(
    <Pill
      mode="recording"
      holdToTalk={false}
      ready
      levels={zeros}
      label="rótulo"
      {...props}
    />,
  );
}

function bars(html: string): number[] {
  return [...html.matchAll(/<i[^>]*height:\s*([\d.]+)px[^>]*><\/i>/g)].map(
    (m) => Number(m[1]),
  );
}

function classes(html: string): string[] {
  const m = html.match(/^<div[^>]*class="([^"]*)"/);
  assert.ok(m, `sem div raiz: ${html}`);
  return m[1].split(/\s+/).filter(Boolean);
}

/** The root holds exactly ten bars and nothing else. */
function onlyBars(html: string) {
  const inner = html.replace(/^<div[^>]*>/, "").replace(/<\/div>$/, "");
  const parts = inner.match(/<i[^>]*><\/i>/g) ?? [];
  assert.equal(inner, parts.join(""), html);
  assert.equal(parts.length, 10, html);
}

function text(html: string): string {
  return html.replace(/<[^>]*>/g, "");
}

/** Corpo do primeiro bloco cujo seletor é exatamente `selector`. */
function rule(source: string, selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const m = source.match(new RegExp(`(?:^|[}\\s])${escaped}\\s*\\{([^}]*)\\}`));
  assert.ok(m, `sem regra ${selector}`);
  return m[1];
}

function has(body: string, declaration: string, where: string) {
  const [prop, value] = declaration.split(":").map((s) => s.trim());
  const re = new RegExp(
    `(^|;|\\s)${prop}\\s*:\\s*${value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*(;|$)`,
  );
  assert.ok(re.test(body), `${where} sem \`${declaration}\`: ${body}`);
}

function ok(id: string) {
  console.log(`${id} ok`);
}

// C1 - cápsula sem texto, ícone nem botão; 84×30 com raio 15.
{
  const html = render({ levels: loud });
  assert.equal(text(html), "");
  for (const tag of ["<svg", "<button", "<img", "<span", "<p"]) {
    assert.ok(!html.includes(tag), `${tag} na pill: ${html}`);
  }
  assert.equal((html.match(/<div/g) ?? []).length, 1, html);
  assert.ok(classes(html).includes("fpill"));
  const body = rule(css, ".fpill");
  has(body, "width: 84px", ".fpill");
  has(body, "height: 30px", ".fpill");
  has(body, "border-radius: 15px", ".fpill");
  ok("C1");
}

// C2 - exatamente 10 barras numa linha centralizada.
{
  const html = render({ levels: loud });
  onlyBars(html);
  onlyBars(render({ ready: false, levels: loud }));
  const body = rule(css, ".fpill");
  has(body, "display: flex", ".fpill");
  has(body, "align-items: center", ".fpill");
  has(body, "justify-content: center", ".fpill");
  assert.ok(!/flex-wrap/.test(css), "flex-wrap no Pill.css");
  assert.ok(!/flex-direction:\s*column/.test(css), "coluna no Pill.css");
  ok("C2");
}

// C3 - alturas espelhadas a partir de levels[0..4], com levels[0] no centro.
{
  const levels = [1, 0.6, 0.3, 0.1, 0, ...Array(11).fill(0)];
  const h = pillBars(levels);
  assert.equal(h.length, 10);
  for (let i = 0; i < 10; i++) assert.equal(h[i], h[9 - i], `barra ${i}`);
  const one = (v: number) => pillBars([v, 0, 0, 0, 0])[4];
  assert.equal(h[4], 18);
  assert.equal(h[5], 18);
  assert.equal(h[3], one(0.6));
  assert.equal(h[2], one(0.3));
  assert.equal(h[1], one(0.1));
  assert.equal(h[0], 3);
  assert.equal(h[9], 3);
  const noisyTail = [...levels.slice(0, 5), ...Array(11).fill(1)];
  assert.deepEqual(pillBars(noisyTail), h);
  ok("C3");
}

// C4 - 3 px no silêncio, 18 px no máximo, crescente entre os dois.
{
  const center = (v: number) => pillBars([v])[4];
  assert.equal(center(0), 3);
  assert.equal(center(-0.5), 3);
  assert.deepEqual(pillBars([]), Array(10).fill(3));
  assert.equal(center(1), 18);
  assert.equal(center(2.5), 18);
  assert.ok(center(0.1) > 3);
  assert.ok(center(0.1) < center(0.3));
  assert.ok(center(0.3) < center(0.6));
  assert.ok(center(0.6) < 18);
  ok("C4");
}

// C5 - vermelha em push-to-talk e hold-or-toggle, preta em toggle.
{
  assert.equal(isHoldToTalk("push_to_talk"), true);
  assert.equal(isHoldToTalk("hold_or_toggle"), true);
  assert.equal(isHoldToTalk("toggle"), false);
  assert.equal(isHoldToTalk(undefined), false);
  assert.ok(classes(render({ holdToTalk: true })).includes("hold"));
  assert.ok(!classes(render({ holdToTalk: false })).includes("hold"));
  has(rule(css, ".fpill"), "background: #000", ".fpill");
  has(rule(css, ".fpill.hold"), "background: #e5322d", ".fpill.hold");
  ok("C5");
}

// C6 - armando: barras mínimas e apagadas até o recording-ready.
{
  const arming = render({ ready: false, levels: loud });
  assert.deepEqual(bars(arming), Array(10).fill(3));
  assert.ok(classes(arming).includes("arming"));
  has(rule(css, ".fpill.arming i"), "opacity: 0.45", ".fpill.arming i");
  const ready = render({ ready: true, levels: loud });
  assert.ok(!classes(ready).includes("arming"));
  assert.deepEqual(bars(ready), Array(10).fill(18));
  ok("C6");
}

// C7 - processando: preta, forma fixa, sem texto.
{
  const html = render({ mode: "processing", holdToTalk: true, levels: loud });
  const c = classes(html);
  assert.ok(c.includes("processing"));
  assert.ok(!c.includes("hold"));
  assert.equal(text(html), "");
  assert.ok(!html.includes("<svg") && !html.includes("<button"), html);
  onlyBars(html);
  assert.deepEqual(bars(html), [5, 7, 9, 11, 13, 13, 11, 9, 7, 5]);
  assert.equal(toPillMode("transcribing"), "processing");
  assert.equal(toPillMode("processing"), "processing");
  assert.equal(toPillMode("recording"), "recording");
  ok("C7");
}

// C8 - processando ignora o nível e pulsa a 1,2 s entre 1 e 0,55.
{
  assert.equal(
    render({ mode: "processing", levels: loud }),
    render({ mode: "processing", levels: zeros }),
  );
  const body = rule(css, ".fpill.processing");
  assert.ok(
    /animation:\s*fpill-pulse\s+1\.2s\b[^;]*\binfinite\b/.test(body),
    `.fpill.processing sem o pulso: ${body}`,
  );
  const frames = css.match(/@keyframes\s+fpill-pulse\s*\{([\s\S]*?)\n\}/);
  assert.ok(frames, "sem @keyframes fpill-pulse");
  assert.ok(/opacity:\s*1\s*;/.test(frames[1]), frames[1]);
  assert.ok(/opacity:\s*0\.55\s*;/.test(frames[1]), frames[1]);
  ok("C8");
}

// C9 - sem pulso com prefers-reduced-motion.
{
  const media = css.match(
    /@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{([\s\S]*?)\n\}/,
  );
  assert.ok(media, "sem @media (prefers-reduced-motion: reduce)");
  const body = rule(media[1], ".fpill.processing");
  has(body, "animation: none", "reduced-motion");
  has(body, "opacity: 1", "reduced-motion");
  ok("C9");
}

// C10 - o streaming não é pill; oculto não renderiza.
{
  assert.equal(toPillMode("streaming"), null);
  const hidden = overlaySource.indexOf("if (!isVisible) return null;");
  const pill = overlaySource.indexOf("<Pill");
  assert.ok(
    hidden > 0 && pill > hidden,
    "a pill precisa vir depois do retorno oculto",
  );
  assert.ok(
    /listen\("hide-overlay",[\s\S]*?setIsVisible\(false\)/.test(overlaySource),
    "hide-overlay sem setIsVisible(false)",
  );
  ok("C10");
}

// C11 - role="status" e aria-label traduzido.
{
  const html = render({ label: "Gravando" });
  assert.ok(html.includes('role="status"'), html);
  assert.ok(html.includes('aria-label="Gravando"'), html);
  const pt = JSON.parse(read("../i18n/locales/pt/translation.json"));
  const en = JSON.parse(read("../i18n/locales/en/translation.json"));
  assert.equal(pt.overlay.recording, "Gravando");
  assert.equal(en.overlay.recording, "Recording");
  assert.ok(
    /pillMode === "recording"\s*\?\s*t\("overlay\.recording"\)\s*:\s*t\("overlay\.processing"\)/.test(
      overlaySource,
    ),
    "o rótulo precisa ser overlay.recording gravando e overlay.processing processando",
  );
  ok("C11");
}

// C12 - cores literais, iguais nos dois temas.
{
  assert.ok(!css.includes("var("), "var( no Pill.css");
  assert.ok(!css.includes("prefers-color-scheme"), "tema no Pill.css");
  assert.ok(!css.includes("[data-theme"), "data-theme no Pill.css");
  has(rule(css, ".fpill i"), "background: #fff", ".fpill i");
  ok("C12");
}

// C13 - o ramo mínimo é só a pill.
{
  const minimal = overlaySource.slice(
    overlaySource.indexOf("// ---- Minimal overlay"),
  );
  assert.ok(
    minimal.length > 0 && minimal.includes("<Pill"),
    "ramo mínimo sem <Pill",
  );
  assert.ok(minimal.includes("ov-stage"), "pill fora do .ov-stage");
  for (const gone of [
    "scard compact",
    "cancelBtn",
    "workingRow",
    "listeningRow",
    "swork-label",
  ]) {
    assert.ok(!minimal.includes(gone), `${gone} ainda no ramo mínimo`);
  }
  ok("C13");
}

// C14 - a ativação do atalho chega à pill.
{
  assert.ok(overlaySource.includes("shortcut_activation"));
  assert.ok(overlaySource.includes("holdToTalk={holdToTalk}"));
  assert.ok(
    overlaySource.includes(
      "setHoldToTalk(isHoldToTalk(settings.data.shortcut_activation))",
    ),
    "a ativação das settings precisa chegar ao estado holdToTalk",
  );
  ok("C14");
}

// shortcut-gestures C24 - o modo padrão de dois toques começa segurando: a pill fica vermelha.
{
  assert.equal(isHoldToTalk("push_to_talk_double_tap"), true);
  assert.ok(
    classes(
      render({ holdToTalk: isHoldToTalk("push_to_talk_double_tap") }),
    ).includes("hold"),
  );
  ok("shortcut-gestures C24");
}

// session-limit C18 - perto do limite a cápsula gravando fica âmbar e pulsa; processando não.
{
  assert.equal(pillTone("recording", false, true), "limit");
  assert.equal(pillTone("recording", true, true), "limit");
  assert.equal(pillTone("recording", true, false), "hold");
  assert.equal(pillTone("recording", false, false), null);
  assert.equal(pillTone("processing", true, true), null);
  assert.equal(pillTone("processing", false, true), null);

  const toggle = classes(render({ limit: true }));
  assert.ok(toggle.includes("limit"), toggle.join(" "));
  const held = classes(render({ limit: true, holdToTalk: true }));
  assert.ok(held.includes("limit"), held.join(" "));
  assert.ok(!held.includes("hold"), held.join(" "));
  assert.ok(!classes(render({})).includes("limit"));
  assert.ok(!classes(render({ limit: false })).includes("limit"));
  const processing = render({ mode: "processing", limit: true });
  assert.ok(!classes(processing).includes("limit"), processing);
  assert.deepEqual(bars(processing), [5, 7, 9, 11, 13, 13, 11, 9, 7, 5]);

  const amber = render({ limit: true, levels: loud });
  assert.equal(text(amber), "");
  onlyBars(amber);
  assert.deepEqual(bars(amber), Array(10).fill(18));

  const body = rule(css, ".fpill.limit");
  has(body, "background: #d97706", ".fpill.limit");
  assert.ok(
    /animation:\s*fpill-limit-pulse\s+0\.9s\b[^;]*\binfinite\b/.test(body),
    `.fpill.limit sem o pulso: ${body}`,
  );
  const frames = css.match(/@keyframes\s+fpill-limit-pulse\s*\{([\s\S]*?)\n\}/);
  assert.ok(frames, "sem @keyframes fpill-limit-pulse");
  assert.ok(/opacity:\s*1\s*;/.test(frames[1]), frames[1]);
  assert.ok(/opacity:\s*0\.6\s*;/.test(frames[1]), frames[1]);
  const media = css.match(
    /@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{([\s\S]*?)\n\}/,
  );
  assert.ok(media, "sem @media (prefers-reduced-motion: reduce)");
  const still = rule(media[1], ".fpill.limit");
  has(still, "animation: none", "reduced-motion limit");
  has(still, "opacity: 1", "reduced-motion limit");

  assert.ok(
    overlaySource.includes("limit={limitWarning}"),
    "o estado do recording-limit-warning precisa chegar à pill",
  );
  ok("session-limit C18");
}

console.log("pill: all assertions passed");
