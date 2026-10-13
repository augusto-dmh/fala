// Prova dos checks da página Desempenho (`.specs/features/dictation-stats/checks.md`, S2 a S10).
// Rode com `bun src/components/stats/stats.test.tsx`: imprime `<check> ok` e sai com erro na
// primeira falha.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import type { DictationStats } from "@/bindings";
import { StatsView } from "./StatsPage";
import {
  BUDGETS,
  DEFAULT_PERIOD,
  PERIOD_STORAGE_KEY,
  STAGES,
  averageWordsPerDay,
  budgetOf,
  budgetState,
  barText,
  dayBars,
  fallbackShare,
  formatMs,
  latencyRows,
  readPeriod,
  summaryMarkdown,
  writePeriod,
  type Period,
  type Translate,
} from "./statsModel";

const read = (path: string) =>
  readFileSync(new URL(path, import.meta.url), "utf8");
const pt = JSON.parse(read("../../i18n/locales/pt/translation.json"));
const en = JSON.parse(read("../../i18n/locales/en/translation.json"));
const page = read("./StatsPage.tsx");

const ok = (id: string) => console.log(`${id} ok`);
const noop = () => {};

const lookup = (resources: Record<string, unknown>, key: string) =>
  key
    .split(".")
    .reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], resources);

/** `t` over a locale's resources, with `{{x}}` interpolation; a missing key fails. */
const translator =
  (resources: Record<string, unknown>): Translate =>
  (key, options = {}) => {
    // i18next's plural keys: `count` picks `_one` or `_other`.
    const plural =
      typeof options.count === "number"
        ? `${key}_${options.count === 1 ? "one" : "other"}`
        : null;
    const value =
      (plural && lookup(resources, plural)) ?? lookup(resources, key);
    assert.equal(typeof value, "string", `chave sem texto: ${key}`);
    return (value as string).replace(/\{\{(\w+)\}\}/g, (_, name) =>
      String(options[name]),
    );
  };
const tPt = translator(pt);

const stats = (over: Partial<DictationStats> = {}): DictationStats => ({
  days: 7,
  dictations: 42,
  words: 1234,
  llm_attempts: 8,
  fallbacks: 1,
  per_day: [
    { day: "2026-10-05", dictations: 10, words: 300 },
    { day: "2026-10-11", dictations: 32, words: 934 },
  ],
  e2e: { p50: 380, p90: 1001 },
  e2e_llm: { p50: 1200, p90: 1201 },
  asr: { p50: 210, p90: 340 },
  llm: { p50: 640, p90: null },
  paste: { p50: 40, p90: 90 },
  speech: { p50: 3200, p90: 8100 },
  ...over,
});

const TODAY = new Date(2026, 9, 11, 15, 30);
const STAGE_ORDER = "e2e e2e_llm asr llm paste speech";

const view = (s: DictationStats | null, days: Period = 7) =>
  renderToStaticMarkup(
    <StatsView
      days={days}
      onDays={noop}
      stats={s}
      error={null}
      onCopy={noop}
      lang="pt"
      today={TODAY}
      t={tPt}
    />,
  );

// S2: the period defaults to 7, accepts only 7/30/90, survives a storage that throws, and the
// page asks the backend for that period.
{
  const blocked = () => {
    throw new Error("blocked");
  };
  const throwing = { getItem: blocked, setItem: blocked };
  assert.equal(DEFAULT_PERIOD, 7);
  const stored = [null, "30", "90", "15", "abc"].map((v) =>
    readPeriod({ getItem: () => v }),
  );
  assert.deepEqual(stored, [7, 30, 90, 7, 7]);
  assert.equal(readPeriod(throwing), 7);
  // No localStorage in this runtime at all: the default, no throw.
  assert.equal(readPeriod(), 7);
  const written: [string, string][] = [];
  writePeriod(90, { setItem: (k, v) => written.push([k, v]) });
  assert.deepEqual(written, [[PERIOD_STORAGE_KEY, "90"]]);
  assert.doesNotThrow(() => writePeriod(30, throwing));
  assert.match(page, /commands\s*\.getDictationStats\(days\)/);
  assert.match(page, /useState<Period>\(\(\) => readPeriod\(\)\)/);
  const html = view(stats());
  const pressed = (on: boolean, days: number) =>
    `aria-pressed="${on}"[^>]*>${days} dias<`;
  assert.match(
    html,
    new RegExp(
      [pressed(true, 7), pressed(false, 30), pressed(false, 90)].join(".*"),
    ),
  );
}
ok("S2");

// S3: milliseconds below a second, seconds with one decimal in the UI language, "—" for null.
{
  const ms = [380, 999, 1000, 1234, 2400, null];
  assert.deepEqual(
    ms.map((v) => formatMs(v, "pt")),
    ["380 ms", "999 ms", "1,0 s", "1,2 s", "2,4 s", "—"],
  );
  assert.deepEqual(
    ms.map((v) => formatMs(v, "en")),
    ["380 ms", "999 ms", "1.0 s", "1.2 s", "2.4 s", "—"],
  );
}
ok("S3");

// S4: ok up to the target, warn above it, over past the maximum, at the exact limits.
{
  assert.deepEqual(BUDGETS.e2e, { target: 700, max: 1000 });
  assert.deepEqual(BUDGETS.e2e_llm, { target: 1200, max: 2000 });
  const e2e = budgetOf("e2e");
  const llm = budgetOf("e2e_llm");
  const ms = [700, 701, 1000, 1001, 1200, 1201, 2000, 2001];
  assert.deepEqual(
    ms.map((v) => budgetState(v, e2e)).join(" "),
    "ok warn warn over over over over over",
  );
  assert.deepEqual(
    ms.map((v) => budgetState(v, llm)).join(" "),
    "ok ok ok ok ok warn warn over",
  );
  assert.equal(budgetState(null, e2e), "none");
  for (const stage of ["asr", "llm", "paste", "speech"] as const) {
    assert.equal(budgetOf(stage), null, stage);
    assert.equal(budgetState(5000, budgetOf(stage)), "none", stage);
  }
}
ok("S4");

// S5: six rows in order with p50, p90 and budget; the cells carry the semantic colour and the
// words of their state.
{
  const rows = latencyRows(stats(), "pt");
  assert.equal(rows.map((r) => r.stage).join(" "), STAGE_ORDER);
  assert.deepEqual(rows[0], {
    stage: "e2e",
    p50: { text: "380 ms", state: "ok" },
    p90: { text: "1,0 s", state: "over" },
    budget: "700 ms / 1,0 s",
  });
  assert.deepEqual(rows[1].p90, { text: "1,2 s", state: "warn" });
  assert.equal(rows[1].budget, "1,2 s / 2,0 s");
  assert.deepEqual(rows[3].p90, { text: "—", state: "none" });
  assert.equal(rows[5].budget, "—");

  const html = view(stats());
  const order = [...html.matchAll(/data-stage="(\w+)"/g)].map((m) => m[1]);
  assert.equal(order.join(" "), STAGE_ORDER);
  assert.match(html, />Soltar → texto, sem IA<.*>Duração da fala</);
  const row = (stage: string) => {
    const start = html.indexOf(`data-stage="${stage}"`);
    return html.slice(start, html.indexOf("</tr>", start));
  };
  const e2e = row("e2e");
  assert.match(e2e, /<td class="[^"]*text-ok[^"]*">380 ms<\/td>/);
  assert.match(
    e2e,
    /<td class="[^"]*text-warn[^"]*">1,0 s<span[^>]*>acima do máximo<\/span><\/td>/,
  );
  assert.match(
    row("e2e_llm"),
    /<td class="[^"]*text-warn[^"]*">1,2 s<span[^>]*>acima da meta<\/span><\/td>/,
  );
  assert.ok(
    !row("asr").includes("text-ok") && !row("asr").includes("text-warn"),
  );
}
ok("S5");

// S6: dictations, words and the average per day with dictation; the AI block with its share.
{
  assert.equal(averageWordsPerDay(stats()), 617);
  assert.equal(averageWordsPerDay(stats({ per_day: [] })), null);
  assert.equal(fallbackShare(stats(), "pt"), "13%");
  assert.equal(
    fallbackShare(stats({ llm_attempts: 0, fallbacks: 0 }), "pt"),
    "—",
  );
  assert.match(
    view(stats()),
    />42<.*>1.234<.*>Palavras por dia com ditado<.*>617<.*>Quedas para as regras<.*>13%</,
  );
}
ok("S6");

// S7: one bar per local day up to today, zero where nothing was dictated, the busiest at 100 %,
// and an older day from the summary extends the range.
{
  const bars = dayBars(stats().per_day, 7, TODAY);
  assert.equal(
    bars.map((b) => b.day.slice(8)).join(" "),
    "05 06 07 08 09 10 11",
  );
  assert.equal(bars.map((b) => b.words).join(" "), "300 0 0 0 0 0 934");
  assert.equal(bars.map((b) => b.height).join(" "), "32 0 0 0 0 0 100");
  const older = dayBars(
    [{ day: "2026-10-04", dictations: 1, words: 5 }],
    7,
    TODAY,
  );
  assert.equal(older.length, 8);
  assert.equal(older[0].day, "2026-10-04");
  const across = dayBars([], 7, new Date(2026, 10, 2, 9));
  assert.deepEqual(
    [across[0].day, across[across.length - 1].day, across.length],
    ["2026-10-27", "2026-11-02", 7],
  );
  assert.equal(dayBars([], 90, TODAY).length, 90);
  const html = view(stats());
  assert.equal([...html.matchAll(/role="img"/g)].length, 7);
  assert.ok(html.includes('title="domingo, 11 de outubro: 934 palavras"'));
  assert.ok(html.includes('title="terça-feira, 6 de outubro: 0 palavras"'));
  assert.match(html, /style="height:100%"/);
}
ok("S7");

// S8: the Markdown summary, exactly.
{
  assert.equal(
    summaryMarkdown(stats(), "pt", tPt),
    [
      "## Desempenho do ditado: últimos 7 dias",
      "",
      "Ditados: 42 · Palavras: 1.234",
      "Tentativas: 8 · Quedas para as regras: 1 (13%)",
      "",
      "| Etapa | p50 | p90 | Meta / máximo |",
      "| --- | --- | --- | --- |",
      "| Soltar → texto, sem IA | 380 ms | 1,0 s (acima do máximo) | 700 ms / 1,0 s |",
      "| Soltar → texto, com IA | 1,2 s | 1,2 s (acima da meta) | 1,2 s / 2,0 s |",
      "| Reconhecimento | 210 ms | 340 ms | — |",
      "| IA | 640 ms | — | — |",
      "| Colagem | 40 ms | 90 ms | — |",
      "| Duração da fala | 3,2 s | 8,1 s | — |",
      "",
    ].join("\n"),
  );
  const inEn = summaryMarkdown(stats(), "en", translator(en));
  assert.ok(inEn.startsWith("## Dictation performance: last 7 days\n"));
  assert.ok(
    inEn.includes(
      "| Release → text, without AI | 380 ms | 1.0 s (above maximum) |",
    ),
  );
  assert.match(
    page,
    /copyToClipboard\(\s*summaryMarkdown\(stats, i18n\.language, t\),?\s*\)/,
  );
}
ok("S8");

// S9: with zero dictations, only the empty state: no figures, no table, no copy button.
{
  const none = { dictations: 0, words: 0, llm_attempts: 0, per_day: [] };
  const html = view(stats({ ...none, fallbacks: 0 }));
  assert.ok(html.includes("Ainda não há ditados medidos neste período."));
  assert.ok(!html.includes("<table"));
  assert.ok(!html.includes("Copiar resumo"));
  assert.ok(!html.includes('role="img"'));
  assert.doesNotMatch(view(null), /Ainda não há|<table/);
}
ok("S9");

// S10: every key the page uses exists in pt and en, and colours come only from theme tokens.
{
  const keys = [
    ...new Set(
      [...page.matchAll(/["'`](stats\.[\w.]+)["'`]/g)].map((m) => m[1]),
    ),
    ...STAGES.map((s) => `stats.rows.${s}`),
    "rail.stats",
    "stats.markdown.title",
  ];
  assert.ok(keys.length > 15, `keys ${keys.length}`);
  // A plural key exists as `_one` and `_other`.
  const has = (locale: Record<string, unknown>, key: string) =>
    typeof lookup(locale, key) === "string" ||
    (typeof lookup(locale, `${key}_one`) === "string" &&
      typeof lookup(locale, `${key}_other`) === "string");
  for (const key of keys) {
    assert.ok(has(pt, key), `pt ${key}`);
    assert.ok(has(en, key), `en ${key}`);
  }
  assert.equal(lookup(pt, "rail.stats"), "Desempenho");
  assert.equal(lookup(en, "rail.stats"), "Performance");
  assert.equal(
    lookup(pt, "stats.description"),
    "Medido só neste computador; nada sai da máquina.",
  );
  assert.doesNotMatch(page, /#[0-9a-fA-F]{3,8}\b/);
  assert.doesNotMatch(
    page,
    /\b(?:text|bg|border)-(?:red|green|amber|yellow|orange|blue|gray|logo-primary|mid-gray)\b/,
  );
}
ok("S10");

// QA Q30: the chart says which day each bar is and how many words it has, in the UI language:
// every weekday and its words in a week, a date once a week counted back from today otherwise.
{
  const week = barText(dayBars(stats().per_day, 7, TODAY), 7, "pt");
  assert.deepEqual(
    week.map((b) => b.axis),
    ["seg.", "ter.", "qua.", "qui.", "sex.", "sáb.", "dom."],
  );
  assert.deepEqual(
    week.map((b) => b.value),
    ["300", null, null, null, null, null, "934"],
  );
  assert.equal(week[6].date, "domingo, 11 de outubro");
  const month = barText(dayBars([], 30, TODAY), 30, "en");
  const shown = month.flatMap((b, i) => (b.axis ? [[i, b.axis]] : []));
  assert.deepEqual(shown[shown.length - 1], [29, "Oct 11"]);
  assert.deepEqual(shown[shown.length - 2], [22, "Oct 4"]);
  assert.ok(
    month.every((b) => b.value === null),
    "no numbers on 30 bars",
  );
  const html = view(stats());
  assert.match(html, />dom\.</);
  assert.match(html, />934</);
  assert.ok(!html.includes("2026-10-11"), "no ISO date on the page");
  // An eighth, older day in a week keeps the weekday labels and the words.
  const eight = barText(
    dayBars([{ day: "2026-10-04", dictations: 1, words: 1 }], 7, TODAY),
    7,
    "pt",
  );
  assert.equal(eight.length, 8);
  assert.ok(eight.every((b) => b.axis !== null));
  assert.equal(eight[0].value, "1");
  // One word in the singular; a day from another year carries its year.
  assert.ok(
    view({
      ...stats(),
      per_day: [{ day: "2026-10-11", dictations: 1, words: 1 }],
    }).includes('title="domingo, 11 de outubro: 1 palavra"'),
  );
  const newYear = barText(dayBars([], 7, new Date(2027, 0, 2)), 7, "en");
  assert.match(newYear[0].date, /2026/);
  assert.doesNotMatch(newYear[6].date, /2027/);
  ok("Q30");
}
