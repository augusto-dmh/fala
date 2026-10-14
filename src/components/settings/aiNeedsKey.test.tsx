// Prova dos checks de `.specs/features/qa-ai-needs-key/checks.md` (Q37 e Q42 do QA de
// 2026-10-13).
// Rode com `bun src/components/settings/aiNeedsKey.test.tsx`: imprime `<check> ok` e sai com erro
// na primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import type { AppSettings } from "@/bindings";
import { privacyInputs, privacyRows } from "./about/privacyModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const ok = (id: string) => console.log(`${id} ok`);
const llm = read("./LlmSettings.tsx");
const dictation = (settings: Partial<AppSettings> | null) =>
  privacyRows(privacyInputs(settings as AppSettings | null, null, null)).find(
    (row) => row.channel === "dictation",
  )?.statuses;

// K1: ligada sem a chave do Gemini, a IA "aguarda a chave" num estado só, na linha e na
// Privacidade; o interruptor continua da pessoa (dá para desligar antes de salvar a chave) e o
// campo da chave nunca trava (Q37).
{
  assert.deepEqual(dictation({ llm_enabled: true }), ["waitingForKey"]);
  assert.deepEqual(
    dictation({ llm_enabled: true, post_process_api_keys: { gemini: "k" } }),
    ["on", "keySet"],
  );
  assert.deepEqual(dictation({ llm_enabled: false }), ["off", "keyMissing"]);
  // Sem as configurações, a Privacidade não diz menos do que pode sair.
  assert.deepEqual(dictation(null), ["on", "unknown"]);
  assert.match(llm, /<ToggleSwitch\s+checked=\{enabled\}\s+onChange=/);
  assert.match(
    llm,
    /enabled && !hasKey\s*\?\s*t\("settings\.llm\.enabled\.waitingKey"\)/,
  );
  const keyField = llm.slice(
    llm.indexOf("<KeyField"),
    llm.indexOf("/>", llm.indexOf("<KeyField")),
  );
  assert.doesNotMatch(
    keyField,
    /disabled=/,
    "a chave sempre pode ser trocada ou removida",
  );
  for (const lang of ["pt", "en"]) {
    const locale = JSON.parse(
      read(`../../i18n/locales/${lang}/translation.json`),
    );
    assert.ok(locale.settings.llm.enabled.waitingKey, lang);
    assert.ok(locale.settings.about.privacy.status.waitingForKey, lang);
  }
  ok("K1");
}

// K2: os apps sem IA ficam no mesmo card do campo que os adiciona (Q42).
{
  const card = llm.slice(
    llm.indexOf('title={t("settings.llm.disabledApps.title")}'),
    llm.indexOf("<AppStyles />"),
  );
  assert.match(card, /layout="stacked"/);
  assert.ok(
    card.indexOf("disabledApps.map(") < card.lastIndexOf("</SettingContainer>"),
    "os chips ficam dentro do SettingContainer",
  );
  ok("K2");
}

console.log("ai needs key: all assertions passed");
