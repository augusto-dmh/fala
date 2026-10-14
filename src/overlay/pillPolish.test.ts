// Prova dos checks de `.specs/features/qa-pill-polish/checks.md` (Q45, Q46 e Q47 do QA de
// 2026-10-13).
// Rode com `bun src/overlay/pillPolish.test.ts`: imprime `<check> ok` e sai com erro na primeira
// falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { noticeTextKey } from "./noticeModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);

// P1: o aviso de mudo diz qual microfone gravou; o backend manda o nome do que estava aberto, e
// a pergunta vem antes do nome, para a reticência cortar só o nome (Q47).
{
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
  const actions = read("../../apps/desktop/src/actions.rs");
  assert.match(
    actions,
    /let microphone = rm\.microphone_name\(\);[\s\S]*?tauri::async_runtime::spawn/,
    "lido antes do stop, com o stream aberto",
  );
  assert.match(
    actions,
    /show_notice\(&ah, NoticeKind::MicMuted, microphone, true\)/,
  );
  assert.doesNotMatch(
    read("./RecordingOverlay.tsx"),
    /getAppSettings\(\)\.then/,
  );
  for (const lang of ["pt", "en"]) {
    const text: string = JSON.parse(
      read(`../i18n/locales/${lang}/translation.json`),
    ).overlay.notice.micMutedNamed;
    assert.ok(
      text.indexOf("?") < text.indexOf("{{device}}"),
      `${lang}: ${text}`,
    );
  }
  ok("P1");
}

// P2: em repouso, 10 px acima da borda dentro da janela de 20 px (8 de cápsula, 2 de borda);
// aberta no hover, a margem de 6 px de antes (Q46). As cores ficam no pill.test.tsx.
{
  const css = read("./IdlePill.css");
  assert.match(css, /\.fpill\.idle \{[^}]*margin: 0 0 10px;/);
  assert.match(css, /\.ov-stage\.top \.fpill\.idle \{\s*margin: 10px 0 0;/);
  assert.match(css, /\.fpill\.idle\.hover \{[^}]*margin: 0 0 6px;/);
  assert.match(
    css,
    /\.ov-stage\.top \.fpill\.idle\.hover \{\s*margin: 6px 0 0;/,
  );
  ok("P2");
}

console.log("pill polish: all assertions passed");
