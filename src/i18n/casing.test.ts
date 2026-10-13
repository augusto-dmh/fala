// Prova dos checks de `.specs/features/qa-one-casing/checks.md` (Q9, Q12, Q25, Q31, Q38, Q39 e Q43
// do QA de 2026-10-13).
// Rode com `bun src/i18n/casing.test.ts`: imprime `<check> ok` e sai com erro na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { languageName, matchesLanguage } from "../lib/constants/languages";
import { deviceLabel } from "../lib/utils/deviceLabel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const locales = {
  en: JSON.parse(read("./locales/en/translation.json")),
  pt: JSON.parse(read("./locales/pt/translation.json")),
};

// Nomes próprios, siglas e teclas que ficam com maiúscula no meio de um título.
const PROPER = new Set([
  "Fala",
  "AI",
  "IA",
  "API",
  "URL",
  "OpenAI",
  "ONNX",
  "GitHub",
  "English",
  "Portuguese",
  "Brazil",
  "Granola",
  "Whisper",
  "Markdown",
  "Google",
  "Gemini",
  "Anthropic",
  "Claude",
  "PC",
  "MCP",
  "Secure",
  "Input",
  "Ctrl",
  "Shift",
  "Insert",
  "Cmd",
  "Super",
  "Enter",
  "Option",
  "ElevenLabs",
  "Windows",
  "YouTube",
  "CSV",
  "ZIP",
]);
// Nomes de modelo, exemplos e frases com nome próprio, fora da regra.
const EXEMPT = [
  "onboarding.models.",
  "stats.rows.",
  "meeting.people.placeholder",
  "dictionary.snippets.writtenPlaceholder",
  "tray.languagePtBr",
  "meeting.context.language",
  "settings.about.privacy.channels",
  "meeting.import.urlPlaceholder",
  "settings.debug.keyboardDiagnostic.running",
  "settings.debug.updateChecks.lockedDescription",
];

/** Short labels (titles, buttons, options) with a capital after the first word. */
function titleCased(tree: unknown): string[] {
  const found: string[] = [];
  const walk = (node: unknown, path: string[]) => {
    if (node && typeof node === "object") {
      for (const [key, value] of Object.entries(node))
        walk(value, [...path, key]);
      return;
    }
    if (typeof node !== "string") return;
    const key = path.join(".");
    if (EXEMPT.some((prefix) => key.startsWith(prefix))) return;
    // A label may end in an ellipsis or carry a value; a sentence is not a label.
    const label = node
      .replace(/\{\{[^}]*\}\}/g, "")
      .replace(/(\.\.\.|…)\s*$/, "");
    if (label.length >= 70 || /\.\s*$|\. |\? /.test(label)) return;
    const words = label.match(/[\p{L}][\p{L}\p{N}'-]*/gu) ?? [];
    for (const word of words.slice(1)) {
      const parts = word.split("-");
      if (
        parts.some(
          (part, i) =>
            (i > 0 || part === word) &&
            /^\p{Lu}/u.test(part) &&
            !PROPER.has(part) &&
            !/^\p{Lu}+$/u.test(part),
        )
      ) {
        found.push(`${key}: ${node}`);
        return;
      }
    }
  };
  walk(tree, []);
  return found;
}

// C1: títulos, rótulos e botões em sentence case, em en e em pt (Q31, Q45).
for (const [lang, tree] of Object.entries(locales)) {
  assert.deepEqual(titleCased(tree), [], lang);
}
assert.equal(locales.en.tray.pasteLastTranscript, "Paste last dictation");
assert.equal(
  locales.en.settings.debug.muteWhileRecording.label,
  "Mute while recording",
);
assert.equal(
  locales.pt.settings.advanced.autostart.label,
  "Iniciar na inicialização",
);
ok("C1");

// C2: sem macOS nem a dica de Linux em texto que o Windows mostra; o título de seção do modelo e
// a grafia en (Q12, Q38).
{
  const en = locales.en.settings.advanced;
  const pt = locales.pt.settings.advanced;
  for (const text of [
    en.acceleration.transcribe.description,
    en.autoSubmit.description,
    en.overlay.style.description,
    pt.acceleration.transcribe.description,
    pt.autoSubmit.description,
    pt.overlay.style.description,
    pt.overlay.position.description,
  ]) {
    assert.doesNotMatch(text, /macOS|Linux/, text);
  }
  assert.equal(
    pt.overlay.position.description,
    "Onde a sobreposição aparece na tela durante a gravação e a transcrição.",
  );
  assert.equal(
    locales.en.settings.modelSettings.title,
    "Settings for {{model}}",
  );
  assert.doesNotMatch(JSON.stringify(locales.en), /recognis/);
  ok("C2");
}

// C3: os tokens dos snippets explicados nos dois idiomas; a privacidade sem jargão de config
// (Q9, Q43).
{
  assert.match(locales.en.dictionary.snippets.description, /Portuguese names/);
  assert.match(locales.pt.dictionary.snippets.description, /data por extenso/);
  for (const tree of Object.values(locales)) {
    const sends = tree.settings.about.privacy.channels.mcp.sends;
    // Plain words, but it still says where the switch is: there is none in the app.
    assert.doesNotMatch(sends, /= true/, sends);
    assert.match(sends, /mcp\.toml/, sends);
  }
  ok("C3");
}

// C4: abas "Minhas notas | Notas da IA"; "Idioma do aplicativo" ao lado de "Tema do aplicativo";
// "chamada" em pt (Q25, Q39).
{
  assert.equal(locales.pt.meeting.screen.myNotes, "Minhas notas");
  assert.equal(locales.pt.meeting.view.notes, "Notas da IA");
  assert.equal(locales.en.meeting.view.notes, "AI notes");
  assert.equal(locales.pt.appLanguage.title, "Idioma do aplicativo");
  assert.equal(locales.pt.theme.title, "Tema do aplicativo");
  assert.doesNotMatch(JSON.stringify(locales.pt), /\bcall\b/);
  ok("C4");
}

// C5: idiomas do ASR, dispositivo padrão e o placeholder do dropdown no idioma da UI (Q39).
{
  assert.equal(languageName("pt", "pt"), "Português");
  assert.equal(languageName("pt", "en"), "Portuguese");
  assert.equal(languageName("jw", "pt"), "Javanês");
  assert.equal(
    languageName("auto", "pt"),
    "Auto Detect",
    "o seletor troca pelo t()",
  );
  // O padrão do sistema com a palavra da bandeja ("Automático").
  assert.equal(
    locales.pt.settings.sound.defaultDevice,
    locales.pt.tray.microphoneAuto,
  );
  assert.equal(
    deviceLabel("Default", (key) => key),
    "settings.sound.defaultDevice",
  );
  assert.equal(
    deviceLabel("Fone USB", (key) => key),
    "Fone USB",
  );
  // A busca ignora acento e acha pelo nome em inglês ou pelo código.
  const pt = { value: "pt", label: "Português" };
  assert.ok(matchesLanguage(pt, "portugues"));
  assert.ok(matchesLanguage(pt, "Portuguese"));
  assert.ok(matchesLanguage(pt, "pt"));
  assert.ok(!matchesLanguage(pt, "ingl"));
  assert.ok(locales.pt.common.selectOption);
  for (const file of [
    "../components/settings/MicrophoneSelector.tsx",
    "../components/settings/OutputDeviceSelector.tsx",
    "../components/settings/ClamshellMicrophoneSelector.tsx",
  ]) {
    assert.match(read(file), /deviceLabel\(device\.name, t\)/, file);
  }
  assert.match(
    read("../components/ui/Dropdown.tsx"),
    /t\("common\.selectOption"\)/,
  );
  assert.match(
    read("../components/settings/LanguageSelector.tsx"),
    /localizedLanguages\(SELECTABLE_LANGUAGES, i18n\.language\)/,
  );
  ok("C5");
}

console.log("casing: all assertions passed");
