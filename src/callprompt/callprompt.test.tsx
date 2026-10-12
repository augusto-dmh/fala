// Prova do check C15 de `meeting-detection`: contagem de 20 s que pausa no hover, o cartão com
// os botões de cada aviso e as chaves nos dois idiomas. Rode com
// `bun src/callprompt/callprompt.test.tsx`: imprime `<check> ok` e sai com erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { CALL_PROMPT_ANSWERS, CallPromptCard } from "./CallPromptCard";
import { advance, expired, fraction, startCountdown } from "./countdown";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const windowSource = read("./CallPromptWindow.tsx");
const rustSource = read("../../apps/desktop/src/call_prompt.rs");
const locales = ["pt", "en"].map((lang) => ({
  lang,
  json: JSON.parse(read(`../i18n/locales/${lang}/translation.json`)),
}));

function lookup(json: unknown, key: string): unknown {
  return key
    .split(".")
    .reduce<unknown>(
      (node, part) =>
        node && typeof node === "object"
          ? (node as Record<string, unknown>)[part]
          : undefined,
      json,
    );
}

// Countdown: 20 s, nothing passes on hover, closes at zero.
let c = startCountdown(20_000);
assert.equal(fraction(c), 1);
c = advance(c, 5_000, false);
assert.equal(c.remainingMs, 15_000);
for (let i = 0; i < 1_000; i++) c = advance(c, 100, true);
assert.equal(c.remainingMs, 15_000, "hover pauses the countdown");
assert.equal(expired(c), false);
c = advance(c, 15_000, false);
assert.equal(expired(c), true);
assert.equal(fraction(c), 0);
assert.equal(advance(c, 100, false).remainingMs, 0, "never below zero");
// The backend sends 20 s and the window starts from what it gets.
assert.match(rustSource, /COUNTDOWN: Duration = Duration::from_secs\(20\)/);
assert.match(windowSource, /startCountdown\(payload\.countdown_ms\)/);
// Running out answers `expired`, which only closes the card (call_prompt.rs tests).
assert.match(windowSource, /expired\(countdown\)\) answer\("expired"\)/);
console.log("C15 countdown ok");

// The card: the offer has three buttons, the stop question two; the first is primary.
assert.deepEqual(CALL_PROMPT_ANSWERS.offer, ["record", "not_now", "mute_app"]);
assert.deepEqual(CALL_PROMPT_ANSWERS.ask_stop, ["stop", "keep_recording"]);
// Quitting while recording: [Parar e sair] first, and running out never quits (call_prompt.rs).
assert.deepEqual(CALL_PROMPT_ANSWERS.quit, ["stop_and_quit", "cancel"]);
const html = renderToStaticMarkup(
  <CallPromptCard
    kind="offer"
    title="titulo"
    body="corpo"
    actions={[
      { answer: "record", label: "gravar", primary: true },
      { answer: "not_now", label: "agora-nao" },
      { answer: "mute_app", label: "nao-avisar" },
    ]}
    remaining={0.5}
    onAnswer={() => {}}
    onHover={() => {}}
  />,
);
assert.match(html, /role="alertdialog"/);
assert.match(html, /class="primary"[^>]*>gravar</);
assert.match(html, />agora-nao</);
assert.match(html, />nao-avisar</);
assert.match(html, /scaleX\(0\.5\)/);
console.log("C15 card ok");

// Every key the window and the settings use exists in pt and en, with {{app}} where needed.
const keys = [
  "callPrompt.offer.title",
  "callPrompt.offer.body",
  "callPrompt.askStop.title",
  "callPrompt.askStop.body",
  "callPrompt.record",
  "callPrompt.notNow",
  "callPrompt.muteApp",
  "callPrompt.stop",
  "callPrompt.keepRecording",
  "callPrompt.quit.title",
  "callPrompt.quit.body",
  "callPrompt.stopAndQuit",
  "callPrompt.cancel",
  "settings.meetings.title",
  "settings.meetings.callPrompt.label",
  "settings.meetings.callPrompt.description",
  "settings.meetings.mutedApps.title",
  "settings.meetings.mutedApps.description",
  "settings.meetings.mutedApps.add",
  "settings.meetings.mutedApps.remove",
];
for (const { lang, json } of locales) {
  for (const key of keys) {
    assert.equal(typeof lookup(json, key), "string", `${lang} ${key}`);
  }
  for (const key of [
    "callPrompt.offer.title",
    "callPrompt.askStop.title",
    "callPrompt.muteApp",
  ]) {
    assert.match(lookup(json, key) as string, /\{\{app\}\}/, `${lang} ${key}`);
  }
}
assert.equal(
  lookup(locales[0].json, "callPrompt.offer.title"),
  "Parece uma chamada no {{app}}",
);
console.log("C15 keys ok");
