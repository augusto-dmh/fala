// Prova do QA Q41: "Recomendado" e a ordem dos downloads seguem o idioma do ditado.
// Rode com `bun src/components/settings/models/modelsModel.test.ts`.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fitsLanguage, recommendedFor, splitByLanguage } from "./modelsModel";

const ok = (id: string) => console.log(`${id} ok`);
const model = (id: string, langs: string[], recommended = true) => ({
  id,
  supported_languages: langs,
  is_recommended: recommended,
});
const canary = model("canary-180m-flash", ["en", "de", "es", "fr"]);
const parakeet = model("parakeet-v3", ["en", "pt", "es", "de", "fr"]);
const whisper = model("whisper-small", [], false);

// Canary (en/de/es/fr) is not recommended for Brazilian Portuguese; Parakeet is.
{
  assert.equal(recommendedFor(canary, "pt-BR"), false);
  assert.equal(recommendedFor(canary, "pt"), false);
  assert.equal(recommendedFor(parakeet, "pt-BR"), true);
  assert.equal(recommendedFor(canary, "en"), true);
  // "auto" or no language: every recommended model keeps its badge.
  assert.equal(recommendedFor(canary, "auto"), true);
  assert.equal(recommendedFor(canary, null), true);
  // Not recommended stays not recommended.
  assert.equal(recommendedFor(whisper, "pt"), false);
  // A model without a language list is not pushed away.
  assert.equal(fitsLanguage(whisper, "pt"), true);
  ok("Q41-recommended");
}

// The downloads split: the dictation language first, the rest behind "Other models".
{
  const { fit, other } = splitByLanguage([canary, parakeet, whisper], "pt");
  assert.deepEqual(
    fit.map((m) => m.id),
    ["parakeet-v3", "whisper-small"],
  );
  assert.deepEqual(
    other.map((m) => m.id),
    ["canary-180m-flash"],
  );
  assert.equal(splitByLanguage([canary, parakeet], "auto").other.length, 0);
  const page = readFileSync(
    new URL("./ModelsSettings.tsx", import.meta.url),
    "utf8",
  );
  assert.match(
    page,
    /showRecommended=\{recommendedFor\(model, targetLanguage\)\}/,
  );
  // With a language filter on, the badge is about that language.
  assert.match(
    page,
    /languageFilter === "all" \? asrLanguage : languageFilter/,
  );
  assert.match(page, /\{showOther && hiddenAvailable\.map\(availableCard\)\}/);
  // A search or any filter shows every match; only the plain list folds.
  for (const part of [
    'languageFilter !== "all"',
    'searchQuery.trim() !== ""',
    "filterStreaming",
    "filterTranslation",
  ]) {
    assert.ok(page.slice(page.indexOf("const narrowed")).includes(part), part);
  }
  assert.match(page, /narrowed\s*\?\s*\{ fit: availableModels, other: \[\] \}/);
  ok("Q41-split");
}
