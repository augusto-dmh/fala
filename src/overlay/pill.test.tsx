// Prova dos checks da pill (`.specs/features/pill-redesign/checks.md`), do vermelho no modo
// padrão de dois toques (`.specs/features/shortcut-gestures/checks.md`, C24) e do estado âmbar
// do limite de sessão (`.specs/features/session-limit/checks.md`, C18) e do estado de carga do
// modelo (`.specs/features/upstream-catchup/checks-pr2-pill-loading.md`) e da pílula em repouso
// (`.specs/features/pill-idle/checks.md`, C6 a C9) e da falha de cola e do clique na reunião
// (`.specs/features/pill-polish/checks.md`, C3 a C5).
// Rode com `bun src/overlay/pill.test.tsx`: imprime `<check> ok` e sai com erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { Pill, type PillProps } from "./Pill";
import {
  SLOW_LOAD_MS,
  formatElapsed,
  isHoldToTalk,
  nextModelLoadStart,
  pillBars,
  pillTone,
  showsModelLoading,
  toPillMode,
} from "./pillModel";
import { MeetingPill } from "./MeetingPill";
import { IdlePill, type IdlePillProps } from "./IdlePill";
import {
  PASTE_FAILED_MS,
  idleKeys,
  pasteFailedHint,
  pasteFailedTimeout,
  pasteLastShortcut,
  showsDoubleTapHint,
} from "./pillModel";
import { PasteFailedPill } from "./PasteFailedPill";
import {
  requestOpenMeeting,
  takeOpenMeetingRequest,
} from "../components/meeting/meetingModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const css = read("./Pill.css");
const overlaySource = read("./RecordingOverlay.tsx");
const locale = (lang: string) =>
  JSON.parse(read(`../i18n/locales/${lang}/translation.json`));

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

// upstream-catchup PR2 C1 - só completed/failed encerram a carga; started repetido mantém o início.
{
  assert.equal(nextModelLoadStart(null, "loading_started", 1000), 1000);
  assert.equal(nextModelLoadStart(500, "loading_started", 1000), 500);
  assert.equal(nextModelLoadStart(500, "loading_completed", 1000), null);
  assert.equal(nextModelLoadStart(500, "loading_failed", 1000), null);
  assert.equal(nextModelLoadStart(500, "unloaded", 1000), 500);
  assert.equal(nextModelLoadStart(500, "selection_changed", 1000), 500);
  assert.equal(nextModelLoadStart(null, "unloaded", 1000), null);
  ok("upstream-catchup PR2 C1");
}

// upstream-catchup PR2 C2 - processando mostra a carga na hora; gravando, só a partir de 2 s.
{
  assert.equal(SLOW_LOAD_MS, 2000);
  assert.equal(showsModelLoading("recording", null, 10_000), false);
  assert.equal(showsModelLoading("processing", null, 10_000), false);
  assert.equal(showsModelLoading("processing", 1000, 1000), true);
  assert.equal(showsModelLoading("recording", 1000, 2999), false);
  assert.equal(showsModelLoading("recording", 1000, 3000), true);
  ok("upstream-catchup PR2 C2");
}

// upstream-catchup PR2 C3 - classe `loading`, sem texto, barras ao vivo gravando e paradas processando.
{
  const recording = render({ modelLoading: true, levels: loud });
  assert.ok(classes(recording).includes("loading"), recording);
  assert.equal(text(recording), "");
  onlyBars(recording);
  assert.deepEqual(bars(recording), Array(10).fill(18));

  const processing = render({
    mode: "processing",
    modelLoading: true,
    levels: loud,
  });
  assert.ok(classes(processing).includes("loading"), processing);
  assert.equal(text(processing), "");
  onlyBars(processing);
  assert.deepEqual(bars(processing), [5, 7, 9, 11, 13, 13, 11, 9, 7, 5]);

  assert.ok(!classes(render({})).includes("loading"));
  assert.ok(!classes(render({ mode: "processing" })).includes("loading"));
  assert.ok(!classes(render({ modelLoading: false })).includes("loading"));
  ok("upstream-catchup PR2 C3");
}

// upstream-catchup PR2 C4 - anel e brilho; processando troca o pulso pelo brilho; sem movimento, só o anel.
{
  has(
    rule(css, ".fpill.loading"),
    "box-shadow: inset 0 0 0 1.5px rgba(255, 255, 255, 0.6)",
    ".fpill.loading",
  );
  const sweep = rule(css, ".fpill.loading::after");
  assert.ok(
    /animation:\s*fpill-loading-sweep\b[^;]*\binfinite\b/.test(sweep),
    `.fpill.loading::after sem o brilho: ${sweep}`,
  );
  assert.ok(/@keyframes\s+fpill-loading-sweep\s*\{/.test(css));
  has(
    rule(css, ".fpill.processing.loading"),
    "animation: none",
    ".fpill.processing.loading",
  );
  const media = css.match(
    /@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{([\s\S]*?)\n\}/,
  );
  assert.ok(media, "sem @media (prefers-reduced-motion: reduce)");
  const still = rule(media[1], ".fpill.loading::after");
  has(still, "animation: none", "reduced-motion loading");
  has(still, "display: none", "reduced-motion loading");
  ok("upstream-catchup PR2 C4");
}

// upstream-catchup PR2 C5 - strings pt e en, usadas no aria-label da pill.
{
  const pt = locale("pt").overlay;
  const en = locale("en").overlay;
  assert.equal(pt.loadingModel, "Carregando modelo…");
  assert.equal(pt.recordingLoadingModel, "Gravando, carregando modelo…");
  assert.equal(en.loadingModel, "Loading model…");
  assert.equal(en.recordingLoadingModel, "Recording, loading model…");
  assert.ok(overlaySource.includes('t("overlay.loadingModel")'));
  assert.ok(overlaySource.includes('t("overlay.recordingLoadingModel")'));
  assert.ok(
    /const pillLabel = pillLoading\s*\?\s*loadingLabel/.test(overlaySource),
    "o rótulo da pill precisa trocar para o de carga",
  );
  assert.ok(overlaySource.includes("label={pillLabel}"));
  ok("upstream-catchup PR2 C5");
}

// upstream-catchup PR2 C6 - o overlay segue model-state-changed e liga a pill e o rótulo ao vivo.
{
  assert.ok(
    /listen<ModelStateEvent>\(\s*"model-state-changed"/.test(overlaySource),
    "o overlay precisa escutar model-state-changed",
  );
  assert.ok(overlaySource.includes("nextModelLoadStart("));
  assert.ok(overlaySource.includes("showsModelLoading("));
  assert.ok(overlaySource.includes("modelLoading={pillLoading}"));
  assert.ok(
    /workKind === "polishing"\s*\?\s*t\("overlay\.processing"\)\s*:\s*transcribingLabel/.test(
      overlaySource,
    ),
    "o rótulo de trabalho ao vivo precisa usar transcribingLabel",
  );
  assert.ok(
    /const transcribingLabel =\s*loadStart !== null\s*\?\s*t\("overlay\.loadingModel"\)/.test(
      overlaySource,
    ),
  );
  ok("upstream-catchup PR2 C6");
}

// upstream-catchup PR2 C7 - nenhuma espera nova antes de a pill aparecer.
{
  const handler = overlaySource.match(
    /listen\("show-overlay", async \(event\) => \{([\s\S]*?)\n {6}\}\);/,
  );
  assert.ok(handler, "sem o handler de show-overlay");
  assert.equal((handler[1].match(/\bawait\b/g) ?? []).length, 2, handler[1]);
  ok("upstream-catchup PR2 C7");
}

// Pill da reunião (`.specs/features/meeting-panel/checks.md`, C15 e C25).
{
  const meeting = (paused: boolean, muted: boolean, recordedMs = 725_000) =>
    renderToStaticMarkup(
      <MeetingPill
        paused={paused}
        recordedMs={recordedMs}
        muted={muted}
        label="reunião"
      />,
    );
  const live = meeting(false, false);
  assert.equal(text(live), "12:05", live);
  assert.ok(classes(live).includes("meeting"), live);
  assert.ok(classes(live).includes("live"), live);
  assert.ok(live.includes('class="dot"'), live);
  assert.ok(live.includes('role="status"'), live);
  const paused = meeting(true, true);
  assert.ok(classes(paused).includes("paused"), paused);
  assert.equal(text(paused), "12:05");
  assert.equal(formatElapsed(3_723_000), "1:02:03");
  assert.equal(formatElapsed(0), "00:00");
  assert.equal(toPillMode("meeting"), null);
  assert.equal(toPillMode("meeting_paused"), null);
  has(rule(css, ".fpill.meeting .dot"), "background: #e5322d", "dot");
  has(rule(css, ".fpill.meeting.paused .dot"), "background: #9ca3af", "paused");
  assert.ok(
    overlaySource.includes('state === "meeting" || state === "meeting_paused"'),
    "o overlay precisa desenhar a pill da reunião nos dois estados",
  );
  ok("meeting-pill");

  const muted = meeting(false, true);
  assert.ok(classes(muted).includes("muted"), muted);
  has(rule(css, ".fpill.meeting.muted .dot"), "background: #d97706", "muted");
  assert.ok(
    overlaySource.includes(
      "event.payload.muted_mic || event.payload.muted_system",
    ),
    "o aviso de canal mudo precisa chegar à pill",
  );
  ok("meeting-muted");
}

// pill-idle C6 - em repouso: só a cápsula, sem barras, ponto, cor nem texto; o CSS não anima nem
// usa vermelho.
function renderIdle(props: Partial<IdlePillProps>): string {
  return renderToStaticMarkup(
    <IdlePill
      hover={false}
      keys={["Ctrl", "Meta"]}
      dictateLabel="Ditar"
      doubleTapLabel="toque duplo para travar"
      label="repouso"
      {...props}
    />,
  );
}
const idleCss = read("./IdlePill.css");
{
  const html = renderIdle({});
  assert.equal(text(html), "", html);
  assert.equal((html.match(/<div/g) ?? []).length, 1, html);
  for (const tag of ["<i", "<span", "<kbd", "<svg"]) {
    assert.ok(!html.includes(tag), `${tag} no repouso: ${html}`);
  }
  assert.deepEqual(classes(html), ["fpill", "idle"]);
  const body = rule(idleCss, ".fpill.idle");
  has(body, "width: 40px", ".fpill.idle");
  has(body, "height: 8px", ".fpill.idle");
  has(body, "background: rgba(0, 0, 0, 0.5)", ".fpill.idle");
  has(body, "border: 1px solid rgba(255, 255, 255, 0.5)", ".fpill.idle");
  assert.ok(!/animation|@keyframes/.test(idleCss), "o repouso não anima");
  assert.ok(
    !/#e5322d|#d97706/i.test(idleCss),
    "o repouso não usa vermelho nem âmbar",
  );
  // Nenhuma regra do Pill.css com cor de gravação vale para .idle.
  assert.ok(!/\.fpill\.idle/.test(css), "o repouso fica fora do Pill.css");
  ok("pill-idle C6");
}

// pill-idle C7 - hover: "Ditar" e um <kbd> por tecla; a dica do toque duplo só no modo padrão.
{
  assert.deepEqual(idleKeys("ctrl+meta"), ["Ctrl", "Meta"]);
  assert.deepEqual(idleKeys("shift_left+space"), ["Left Shift", "Space"]);
  assert.deepEqual(idleKeys(""), []);
  assert.deepEqual(idleKeys(undefined), []);
  const html = renderIdle({ hover: true });
  assert.ok(classes(html).includes("hover"), html);
  const kbds = [...html.matchAll(/<kbd>([^<]*)<\/kbd>/g)].map((m) => m[1]);
  assert.deepEqual(kbds, ["Ctrl", "Meta"]);
  assert.ok(html.includes(">Ditar<"), html);
  assert.ok(html.includes("toque duplo para travar"), html);
  const noHint = renderIdle({ hover: true, doubleTapLabel: null });
  assert.ok(!noHint.includes("idle-hint"), noHint);
  const modes = [
    "toggle",
    "push_to_talk",
    "hold_or_toggle",
    undefined,
  ] as const;
  assert.ok(showsDoubleTapHint("push_to_talk_double_tap"));
  assert.ok(modes.every((mode) => !showsDoubleTapHint(mode)));
  assert.equal(toPillMode("idle"), null);
  // pt e en têm as mesmas chaves (`bun run check:translations`).
  const { dictate, doubleTap, label } = locale("pt").overlay.idle;
  assert.deepEqual([dictate, doubleTap], ["Ditar", "toque duplo para travar"]);
  assert.ok(label);
  ok("pill-idle C7");
}

// pill-idle C8 - o overlay liga hover e clique aos comandos; Início abre pelo evento open-home.
{
  const at = overlaySource.indexOf('if (state === "idle") {');
  assert.ok(at > 0, "sem o ramo idle");
  const idle = overlaySource.slice(at, overlaySource.indexOf("\n  }\n", at));
  assert.ok(idle.includes("commands.setIdlePillHover(on)"), idle);
  assert.ok(idle.includes("onMouseEnter={() => hover(true)}"), idle);
  assert.ok(idle.includes("onMouseLeave={() => hover(false)}"), idle);
  assert.ok(idle.includes("commands.openHomeFromPill()"), idle);
  assert.ok(overlaySource.includes('if (event.payload === "idle")'));
  assert.ok(overlaySource.includes("commands.pillOverlayReady()"));
  const app = read("../App.tsx");
  assert.match(
    app,
    /listen\("open-home", \(\) => \{\s*setDestination\("home"\);/,
  );
  const rust = read("../../apps/desktop/src/overlay.rs");
  const open = rust.slice(rust.indexOf("pub fn open_home_from_pill"));
  const body = open.slice(0, open.indexOf("\n}\n"));
  assert.ok(body.includes("crate::show_main_window(&app)"), body);
  assert.ok(body.includes('emit_to("main", "open-home", ())'), body);
  ok("pill-idle C8");
}

// pill-idle C9 - o toggle fica logo depois do ShowOverlay, só no Windows e com a pílula ligada.
{
  const advanced = read("../components/settings/advanced/AdvancedSettings.tsx");
  assert.match(
    advanced,
    /<ShowOverlay [^>]*\/>\s*<ShowIdlePill descriptionMode="tooltip" grouped=\{true\} \/>/,
  );
  const toggle = read("../components/settings/ShowIdlePill.tsx");
  assert.ok(
    toggle.includes(
      'if (osType !== "windows" || getSetting("overlay_style") === "none") {\n      return null;',
    ),
    toggle,
  );
  assert.ok(toggle.includes('updateSetting("pill_idle_visible", enabled)'));
  const { label, description } = locale("pt").settings.advanced.idlePill;
  assert.equal(label, "Mostrar a pílula em repouso");
  assert.ok(description);
  ok("pill-idle C9");
}

// pill-polish C3 - falha de cola: a pílula diz o que houve, o atalho de colar o último (ou só o
// clique) e "Copiado" depois do clique; fica 5 s, parada enquanto o mouse está em cima.
{
  assert.equal(toPillMode("paste_failed"), null);
  assert.equal(PASTE_FAILED_MS, 5000);
  assert.equal(pasteFailedTimeout(false), 5000);
  assert.equal(pasteFailedTimeout(true), null);
  assert.equal(pasteLastShortcut("ctrl+shift+v"), "Ctrl + Shift + V");
  assert.equal(pasteLastShortcut(""), null);
  assert.equal(pasteLastShortcut(undefined), null);
  assert.deepEqual(pasteFailedHint(false, "Ctrl + Shift + V"), {
    key: "overlay.pasteFailed.retry",
    keys: "Ctrl + Shift + V",
  });
  assert.deepEqual(pasteFailedHint(false, null), {
    key: "overlay.pasteFailed.click",
  });
  assert.deepEqual(pasteFailedHint(true, "Ctrl + Shift + V"), {
    key: "overlay.pasteFailed.copied",
  });

  const pt = locale("pt").overlay.pasteFailed;
  const en = locale("en").overlay.pasteFailed;
  assert.equal(pt.title, "Não deu para colar.");
  assert.equal(pt.retry, "{{keys}} cola de novo · clique para copiar");
  assert.equal(pt.click, "Clique para copiar");
  assert.equal(pt.copied, "Copiado");
  assert.ok(pt.label && en.label && en.title && en.retry.includes("{{keys}}"));
  assert.equal(en.copied, "Copied");

  const onCopy = () => {};
  const hovers: boolean[] = [];
  const props = {
    title: pt.title,
    hint: pt.retry.replace("{{keys}}", "Ctrl + Shift + V"),
    label: pt.label,
    onCopy,
    onHover: (on: boolean) => hovers.push(on),
  };
  const html = renderToStaticMarkup(<PasteFailedPill {...props} />);
  assert.equal(
    text(html),
    "Não deu para colar.Ctrl + Shift + V cola de novo · clique para copiar",
  );
  assert.ok(classes(html).includes("paste_failed"), html);
  assert.ok(html.includes('role="button"'), html);
  assert.ok(html.includes(`aria-label="${pt.label}"`), html);
  // ADR-0017: the warning tone the notices already use, no new colour.
  assert.ok(html.includes('class="fnotice-dot warn"'), html);
  const noticeCss = read("./Notice.css");
  for (const selector of [".fnotice.paste_failed", ".fnotice-hint"]) {
    assert.ok(!/#[0-9a-f]{3,6}\b/i.test(rule(noticeCss, selector)), selector);
  }
  // The whole card copies, and the pointer on the card (not the window) stops the timer.
  const card = PasteFailedPill(props);
  assert.equal(card.props.onClick, onCopy);
  card.props.onMouseEnter();
  card.props.onMouseLeave();
  assert.deepEqual(hovers, [true, false]);
  ok("pill-polish C3");
}

// pill-polish C4 - o overlay liga o estado: lê o atalho, para o timer no hover, encerra pelo
// comando e copia pelo clique; a janela principal continua com o toast.
{
  const at = overlaySource.indexOf('if (state === "paste_failed") {');
  assert.ok(at > 0, "sem o ramo paste_failed");
  const branch = overlaySource.slice(at, overlaySource.indexOf("\n  }\n", at));
  assert.ok(branch.includes("onHover={setPasteHover}"), branch);
  assert.ok(!branch.includes("onMouseEnter"), "o hover é do cartão");
  assert.ok(branch.includes("pasteFailedHint(pasteCopied, pasteLastKeys)"));
  assert.ok(
    /commands\s*\.copyFailedPaste\(\)\s*\.then\(\(copied\) => copied && setPasteCopied\(true\)\)/.test(
      branch,
    ),
    branch,
  );
  const timer = overlaySource.match(
    /useEffect\(\(\) => \{\s*if \(state !== "paste_failed" \|\| !isVisible \|\| pasteGeneration === null\)\s*return;([\s\S]*?)\}, \[state, isVisible, pasteHover, pasteGeneration\]\);/,
  );
  assert.ok(timer, "sem o timer do paste_failed");
  assert.ok(timer[1].includes("pasteFailedTimeout(pasteHover)"), timer[1]);
  assert.ok(
    timer[1].includes("commands.endPasteFailed(pasteGeneration)"),
    timer[1],
  );
  // Each failure names its show, so a second one restarts the timer.
  assert.ok(
    /listen<\{ generation: number \}>\(\s*"paste-failed",\s*\(event\) => \{\s*setPasteCopied\(false\);\s*setPasteGeneration\(event\.payload\.generation\);/.test(
      overlaySource,
    ),
    "o overlay precisa ouvir paste-failed",
  );
  assert.ok(
    overlaySource.includes(
      'if (state !== "paste_failed") setPasteHover(false);',
    ),
  );
  assert.ok(
    overlaySource.includes(
      "pasteLastShortcut(\n                settings.data.bindings?.paste_last?.current_binding,",
    ),
    "o atalho de colar o último precisa vir das settings",
  );
  // The paste_failed branch comes before the idle one, after the hidden return.
  assert.ok(
    overlaySource.indexOf("if (!isVisible) return null;") < at &&
      at < overlaySource.indexOf('if (state === "idle") {'),
  );
  assert.match(
    read("../App.tsx"),
    /listen\("paste-error", \(\) => \{\s*toast\.error/,
  );
  ok("pill-polish C4");
}

// pill-polish C5 - clique na pílula de reunião abre a sessão gravando na janela principal.
{
  const onOpen = () => {};
  const pill = MeetingPill({
    paused: false,
    recordedMs: 0,
    muted: false,
    label: "reunião",
    onOpen,
  });
  assert.equal(pill.props.onClick, onOpen);
  assert.equal(pill.props.role, "status");
  has(rule(css, ".fpill.meeting"), "cursor: pointer", ".fpill.meeting");
  assert.equal(
    (css.match(/(?:^|\n)\.fpill\.meeting \{/g) ?? []).length,
    1,
    "uma regra .fpill.meeting só",
  );
  const meeting = overlaySource.slice(
    overlaySource.indexOf(
      'if (state === "meeting" || state === "meeting_paused")',
    ),
  );
  assert.ok(
    meeting.includes("onOpen={() => void commands.openMeetingFromPill()}"),
  );
  // The mic switch stays on its own button, outside the pill.
  assert.ok(
    meeting.includes("onSwitch={() => void commands.switchMeetingOutput"),
  );

  requestOpenMeeting("m-1");
  assert.equal(takeOpenMeetingRequest(), "m-1");
  assert.equal(takeOpenMeetingRequest(), null);
  const app = read("../App.tsx");
  assert.match(
    app,
    /listen<string \| null>\("open-meeting", \(event\) => \{\s*if \(sectionRef\.current !== "meetings"\) \{\s*if \(event\.payload\) requestOpenMeeting\(event\.payload\);\s*setDestination\("meetings"\);/,
  );
  const page = read("../components/meeting/MeetingsPage.tsx");
  assert.match(
    page,
    /const requested = takeOpenMeetingRequest\(\);\s*if \(requested\) void open\(requested\);/,
  );
  assert.match(
    page,
    /listen<string \| null>\("open-meeting", \(event\) => \{\s*if \(event\.payload\) void open\(event\.payload\);/,
  );
  ok("pill-polish C5");
}

console.log("pill: all assertions passed");
