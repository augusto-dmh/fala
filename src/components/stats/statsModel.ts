import type {
  DictationDay,
  DictationStats,
  LatencyPercentiles,
} from "@/bindings";

export type Translate = (
  key: string,
  options?: Record<string, unknown>,
) => string;

/** The periods the page offers, in days. */
export const PERIODS = [7, 30, 90] as const;
export type Period = (typeof PERIODS)[number];
export const DEFAULT_PERIOD: Period = 7;
export const PERIOD_STORAGE_KEY = "fala.stats.days";

export interface Budget {
  /** At or below: ok. */
  target: number;
  /** Above: past the maximum, a phase bug. */
  max: number;
}

/** Release → text, from the latency budget in `ARCHITECTURE.md`. The other stages have none. */
export const BUDGETS = {
  e2e: { target: 700, max: 1000 },
  e2e_llm: { target: 1200, max: 2000 },
} as const satisfies Record<string, Budget>;

/** Table order, top to bottom. */
export const STAGES = [
  "e2e",
  "e2e_llm",
  "asr",
  "llm",
  "paste",
  "speech",
] as const;
export type Stage = (typeof STAGES)[number];

/** `ok` up to the target, `warn` above it, `over` past the maximum; `none` without a budget or a value. */
export type BudgetState = "ok" | "warn" | "over" | "none";

export const NO_VALUE = "—";

export function budgetOf(stage: Stage): Budget | null {
  return stage === "e2e" || stage === "e2e_llm" ? BUDGETS[stage] : null;
}

export function budgetState(
  ms: number | null,
  budget: Budget | null,
): BudgetState {
  if (ms === null || budget === null) return "none";
  if (ms <= budget.target) return "ok";
  return ms <= budget.max ? "warn" : "over";
}

/** "380 ms" below a second, "1,2 s" (pt) / "1.2 s" (en) from a second on, "—" without a value. */
export function formatMs(ms: number | null, lang: string): string {
  if (ms === null) return NO_VALUE;
  if (ms < 1000) return `${Math.round(ms)} ms`;
  const seconds = new Intl.NumberFormat(lang, {
    minimumFractionDigits: 1,
    maximumFractionDigits: 1,
  }).format(ms / 1000);
  return `${seconds} s`;
}

export function formatCount(n: number, lang: string): string {
  return new Intl.NumberFormat(lang).format(n);
}

export function formatBudget(budget: Budget | null, lang: string): string {
  if (budget === null) return NO_VALUE;
  return `${formatMs(budget.target, lang)} / ${formatMs(budget.max, lang)}`;
}

export interface LatencyCell {
  text: string;
  state: BudgetState;
}

export interface LatencyRow {
  stage: Stage;
  p50: LatencyCell;
  p90: LatencyCell;
  budget: string;
}

export function latencyRows(stats: DictationStats, lang: string): LatencyRow[] {
  return STAGES.map((stage) => {
    const budget = budgetOf(stage);
    const percentiles: LatencyPercentiles = stats[stage];
    const cell = (ms: number | null): LatencyCell => ({
      text: formatMs(ms, lang),
      state: budgetState(ms, budget),
    });
    return {
      stage,
      p50: cell(percentiles.p50),
      p90: cell(percentiles.p90),
      budget: formatBudget(budget, lang),
    };
  });
}

/** Translation key of a stage's row label. */
export const stageKey = (stage: Stage) => `stats.rows.${stage}`;

/** The words of a cell's state, so colour is never the only cue. */
export function stateNote(state: BudgetState, t: Translate): string | null {
  if (state === "warn") return t("stats.latency.aboveTarget");
  if (state === "over") return t("stats.latency.aboveMax");
  return null;
}

/** Words divided by the days that had a dictation, rounded; null with none. */
export function averageWordsPerDay(stats: DictationStats): number | null {
  const days = stats.per_day.length;
  return days === 0 ? null : Math.round(stats.words / days);
}

/** Fallbacks as a whole percentage of AI attempts, "—" with no attempts. */
export function fallbackShare(stats: DictationStats, lang: string): string {
  if (stats.llm_attempts === 0) return NO_VALUE;
  return new Intl.NumberFormat(lang, {
    style: "percent",
    maximumFractionDigits: 0,
  }).format(stats.fallbacks / stats.llm_attempts);
}

const pad = (n: number) => String(n).padStart(2, "0");

/** `AAAA-MM-DD` of a local date, the shape of `DictationDay.day`. */
export function localDay(date: Date): string {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

const parseDay = (day: string): Date => {
  const [y, m, d] = day.split("-").map(Number);
  return new Date(y, m - 1, d);
};

export interface DayBar {
  day: string;
  words: number;
  /** Bar height as a share of the busiest day, 0–100. */
  height: number;
}

/**
 * One bar per local day from `today − (days − 1)` to today, zero where nothing was dictated. The
 * summary covers `days × 24 h`, so it can hold one day older than that: the range then starts there.
 */
export function dayBars(
  perDay: readonly DictationDay[],
  days: number,
  today: Date,
): DayBar[] {
  const end = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  let start = new Date(
    end.getFullYear(),
    end.getMonth(),
    end.getDate() - (days - 1),
  );
  if (perDay.length > 0 && perDay[0].day < localDay(start)) {
    start = parseDay(perDay[0].day);
  }
  const words = new Map(perDay.map((d) => [d.day, d.words]));
  const out: { day: string; words: number }[] = [];
  for (
    let date = start;
    date <= end;
    date = new Date(date.getFullYear(), date.getMonth(), date.getDate() + 1)
  ) {
    const day = localDay(date);
    out.push({ day, words: words.get(day) ?? 0 });
  }
  const busiest = Math.max(0, ...out.map((d) => d.words));
  return out.map((d) => ({
    ...d,
    height: busiest === 0 ? 0 : Math.round((d.words / busiest) * 100),
  }));
}

/** The longest period whose bars each get the weekday under them and the words above. */
export const LABELLED_PERIOD = 7;
/** In longer periods, a date under one bar in this many, counted back from today. */
export const DATE_STRIDE = 7;

/** What the chart writes around one bar, in the UI language. */
export interface BarText {
  /** Under the bar: the weekday in a week; in longer periods the date, once a week counted
   *  back from today, and null on the other bars. */
  axis: string | null;
  /** Above the bar: its words, only in a week and only when there are some. */
  value: string | null;
  /** The full date, for the tooltip and the screen reader. */
  date: string;
}

/** `days` is the period, not the bar count: a week can show an eighth, older day (`dayBars`)
 *  and keeps its labels. The full date carries the year only when it is not the latest bar's. */
export function barText(
  bars: readonly DayBar[],
  days: number,
  lang: string,
): BarText[] {
  const few = days <= LABELLED_PERIOD;
  const lastYear =
    bars.length > 0 ? parseDay(bars[bars.length - 1].day).getFullYear() : 0;
  const weekday = new Intl.DateTimeFormat(lang, { weekday: "short" });
  const short = new Intl.DateTimeFormat(lang, {
    day: "numeric",
    month: "short",
  });
  const long = new Intl.DateTimeFormat(lang, {
    weekday: "long",
    day: "numeric",
    month: "long",
  });
  const longWithYear = new Intl.DateTimeFormat(lang, {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  return bars.map((bar, i) => {
    const date = parseDay(bar.day);
    const fromEnd = bars.length - 1 - i;
    return {
      axis: few
        ? weekday.format(date)
        : fromEnd % DATE_STRIDE === 0
          ? short.format(date)
          : null,
      value: few && bar.words > 0 ? formatCount(bar.words, lang) : null,
      date: (date.getFullYear() === lastYear ? long : longWithYear).format(
        date,
      ),
    };
  });
}

/** The summary as Markdown, ready for a `verification.md` or a phase handoff. */
export function summaryMarkdown(
  stats: DictationStats,
  lang: string,
  t: Translate,
): string {
  const n = (v: number) => formatCount(v, lang);
  const cell = (c: LatencyCell) => {
    const note = stateNote(c.state, t);
    return note ? `${c.text} (${note})` : c.text;
  };
  const lines = [
    `## ${t("stats.markdown.title", { days: stats.days })}`,
    "",
    `${t("stats.counts.dictations")}: ${n(stats.dictations)} · ${t("stats.counts.words")}: ${n(stats.words)}`,
    `${t("stats.ai.attempts")}: ${n(stats.llm_attempts)} · ${t("stats.ai.fallbacks")}: ${n(stats.fallbacks)} (${fallbackShare(stats, lang)})`,
    "",
    `| ${t("stats.latency.stage")} | ${t("stats.latency.p50")} | ${t("stats.latency.p90")} | ${t("stats.latency.budget")} |`,
    "| --- | --- | --- | --- |",
    ...latencyRows(stats, lang).map(
      (row) =>
        `| ${t(stageKey(row.stage))} | ${cell(row.p50)} | ${cell(row.p90)} | ${row.budget} |`,
    ),
  ];
  return `${lines.join("\n")}\n`;
}

type ReadStorage = Pick<Storage, "getItem">;
type WriteStorage = Pick<Storage, "setItem">;

const isPeriod = (n: number): n is Period =>
  (PERIODS as readonly number[]).includes(n);

/** The remembered period, or 7 when storage is empty, invalid or unavailable. */
export function readPeriod(storage?: ReadStorage): Period {
  try {
    const value = Number((storage ?? localStorage).getItem(PERIOD_STORAGE_KEY));
    return isPeriod(value) ? value : DEFAULT_PERIOD;
  } catch {
    return DEFAULT_PERIOD;
  }
}

/** Remembers the period; a storage that throws only loses the convenience. */
export function writePeriod(period: Period, storage?: WriteStorage): void {
  try {
    (storage ?? localStorage).setItem(PERIOD_STORAGE_KEY, String(period));
  } catch {
    // Private mode or blocked storage: the page still works with the default.
  }
}
