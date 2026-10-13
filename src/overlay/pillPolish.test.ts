// Prova dos checks de `.specs/features/qa-pill-polish/checks.md` (Q45, Q46 e Q47 do QA de
// 2026-10-13).
// Rode com `bun src/overlay/pillPolish.test.ts`: imprime `<check> ok` e sai com erro na primeira
// falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { namedMicrophone, noticeTextKey } from "./noticeModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);

// P1: o aviso de mudo diz qual microfone, quando o escolhido em Configurações tem nome (Q47).
{
  assert.equal(namedMicrophone("Fone USB"), "Fone USB");
  for (const fallback of [null, undefined, "", "  ", "default", "Default"]) {
    assert.equal(namedMicrophone(fallback), null, String(fallback));
  }
  assert.equal(
    noticeTextKey({ kind: "mic_muted", device: "Fone USB" }),
    "overlay.notice.micMutedNamed",
  );
  assert.equal(
    noticeTextKey({ kind: "mic_muted", device: null }),
    "overlay.notice.micMuted",
  );
  assert.equal(
    noticeTextKey({ kind: "mic_in_use", device: "Fone USB" }),
    "overlay.notice.micInUse",
  );
  const overlay = read("./RecordingOverlay.tsx");
  assert.match(overlay, /namedMicrophone\(result\.data\.selected_microphone\)/);
  assert.match(
    overlay,
    /current === shown \? \{ \.\.\.shown, device \} : current/,
  );
  for (const lang of ["pt", "en"]) {
    const notice = JSON.parse(read(`../i18n/locales/${lang}/translation.json`))
      .overlay.notice;
    assert.ok(notice.micMutedNamed.includes("{{device}}"), lang);
  }
  ok("P1");
}

// P2: a pílula em repouso tem preenchimento cinza e fica 10 px acima da borda, dentro da janela de
// 20 px (8 px de cápsula + 2 de borda + 10 de margem) (Q46).
{
  const css = read("./IdlePill.css");
  assert.match(css, /background: rgba\(64, 64, 64, 0\.85\);/);
  assert.match(css, /margin: 0 0 10px;/);
  assert.match(css, /\.ov-stage\.top \.fpill\.idle \{\s*margin: 10px 0 0;/);
  ok("P2");
}

// P3: o menu do clique direito tem "Copiar o último ditado", tratado pelo handler do tray (Q45).
{
  const tray = read("../../apps/desktop/src/tray.rs");
  assert.match(
    tray,
    /const PILL_MENU: \[Option<&str>; 8\] = \[\s*Some\("paste_last_transcript"\),\s*Some\("copy_last_transcript"\),/,
  );
  assert.match(
    tray,
    /"copy_last_transcript" => &strings\.copy_last_transcript,/,
  );
  ok("P3");
}

console.log("pill polish: all assertions passed");
