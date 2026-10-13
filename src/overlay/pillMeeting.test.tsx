// Prova dos checks de `.specs/features/qa-pill-new-meeting/checks.md` (Q44 e o item "Nova
// reunião" do Q45, do QA de 2026-10-13).
// Rode com `bun src/overlay/pillMeeting.test.tsx`: imprime `<check> ok` e sai com erro na primeira
// falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { IdlePill } from "./IdlePill";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};

const pill = (hover: boolean, withMeeting = true) =>
  renderToStaticMarkup(
    <IdlePill
      hover={hover}
      keys={["Ctrl", "Space"]}
      dictateLabel="Ditar"
      doubleTapLabel={null}
      label="Fala em repouso"
      newMeetingLabel={withMeeting ? "Nova reunião" : undefined}
      onNewMeeting={withMeeting ? noop : undefined}
    />,
  );

// M1: o botão "Nova reunião" aparece só no hover, cinza (nunca vermelho), e o clique nele não abre
// a janela principal: só ele começa a reunião (ADR-0005).
{
  assert.ok(!pill(false).includes("Nova reunião"), "em repouso, nada");
  assert.match(
    pill(true),
    /<button type="button" class="idle-meeting">Nova reunião<\/button>/,
  );
  assert.ok(!pill(true, false).includes("idle-meeting"));
  // Com o botão dentro, o cartão é um grupo, não um botão com outro dentro.
  assert.match(pill(true), /^<div class="fpill idle hover" role="group"/);
  assert.match(pill(false), /^<div class="fpill idle" role="button"/);
  assert.match(pill(true, false), /role="button"/);
  const source = read("./IdlePill.tsx");
  assert.match(source, /event\.stopPropagation\(\);\s*onNewMeeting\?\.\(\);/);
  // Apertar o botão nunca começa o arrasto da pílula.
  assert.match(
    source,
    /onPointerDown=\{\(event\) => event\.stopPropagation\(\)\}/,
  );
  const css = read("./IdlePill.css");
  const rule = /\.fpill\.idle \.idle-meeting \{[^}]*\}/.exec(css)?.[0] ?? "";
  assert.ok(rule, "regra do botão");
  assert.ok(
    !/#e5322d|#d97706|red/i.test(rule),
    "o botão não usa cor de gravação",
  );
  const overlay = read("./RecordingOverlay.tsx");
  assert.match(
    overlay,
    /onNewMeeting=\{\(\) => void commands\.startMeetingFromPill\(\)\}/,
  );
  ok("M1");
}

// M2: o comando segue o caminho do item do tray (consentimento antes, quando falta), só vale com a
// pílula em repouso, abre Reuniões se a reunião não começou, e a janela do hover cresce.
{
  const rust = read("../../apps/desktop/src/overlay.rs");
  assert.match(
    rust,
    /pub fn start_meeting_from_pill\(app: AppHandle\) \{[\s\S]*?meetings\.start_from_tray\(\);/,
  );
  assert.match(rust, /const OVERLAY_IDLE_HOVER_HEIGHT: f64 = 96\.0;/);
  const command = rust.slice(
    rust.indexOf("pub fn start_meeting_from_pill"),
    rust.indexOf("/// A click on the meeting pill"),
  );
  // Só a pílula em repouso oferece; sem reunião depois da tentativa, abre Reuniões.
  assert.match(
    command,
    /matches!\(Shown::load\(\), Shown::Idle \| Shown::IdleHover\)/,
  );
  assert.match(
    command,
    /if meetings\.status\(\)\.id\.is_none\(\) \{\s*crate::show_main_window\(&app\);\s*let _ = app\.emit_to\("main", "open-meeting", None::<String>\);/,
  );
  assert.match(
    read("../../apps/desktop/src/lib.rs"),
    /overlay::start_meeting_from_pill,/,
  );
  assert.match(read("../bindings.ts"), /async startMeetingFromPill\(\)/);
  ok("M2");
}

// M3: o menu do clique direito ganha "Gravar reunião", com o id e o handler do tray (Q45).
{
  const tray = read("../../apps/desktop/src/tray.rs");
  assert.match(
    tray,
    /Some\("copy_last_transcript"\),\s*None,\s*Some\("meeting_start"\),\s*None,/,
  );
  const pillMenu = tray.slice(
    tray.indexOf("pub fn build_pill_menu"),
    tray.indexOf("Ok(menu)", tray.indexOf("pub fn build_pill_menu")),
  );
  assert.match(pillMenu, /"meeting_start" => &strings\.record_meeting,/);
  for (const lang of ["pt", "en"]) {
    const idle = JSON.parse(read(`../i18n/locales/${lang}/translation.json`))
      .overlay.idle;
    assert.ok(idle.newMeeting, lang);
  }
  ok("M3");
}

console.log("pill meeting: all assertions passed");
