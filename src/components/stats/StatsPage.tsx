import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Copy } from "lucide-react";
import { commands, type DictationStats } from "@/bindings";
import { copyToClipboard } from "../settings/history/clipboard";
import { PageHeader } from "../PageHeader";
import { Button } from "../ui/Button";
import {
  PERIODS,
  averageWordsPerDay,
  dayBars,
  fallbackShare,
  formatCount,
  latencyRows,
  readPeriod,
  stageKey,
  stateNote,
  summaryMarkdown,
  writePeriod,
  type BudgetState,
  type LatencyCell,
  type Period,
  type Translate,
} from "./statsModel";

const STATE_CLASS: Record<BudgetState, string> = {
  ok: "text-ok",
  warn: "text-warn",
  over: "text-warn font-semibold",
  none: "text-text",
};

const Card: React.FC<{ title?: string; children: React.ReactNode }> = ({
  title,
  children,
}) => (
  <section className="rounded-lg border border-border bg-surface-1 p-4">
    {title && (
      <h2 className="mb-3 text-body font-semibold text-text">{title}</h2>
    )}
    {children}
  </section>
);

const Figure: React.FC<{ label: string; value: string }> = ({
  label,
  value,
}) => (
  <div className="flex flex-col gap-1 rounded-lg border border-border bg-surface-1 p-4">
    <span className="text-caption text-text-2">{label}</span>
    <span className="font-display text-title font-semibold text-text tabular-nums">
      {value}
    </span>
  </div>
);

const Cell: React.FC<{ cell: LatencyCell; t: Translate }> = ({ cell, t }) => {
  const note = stateNote(cell.state, t);
  return (
    <td className={`py-2 pe-3 tabular-nums ${STATE_CLASS[cell.state]}`}>
      {cell.text}
      {note && <span className="block text-caption">{note}</span>}
    </td>
  );
};

interface PeriodPickerProps {
  days: Period;
  onDays: (days: Period) => void;
  t: Translate;
}

const PeriodPicker: React.FC<PeriodPickerProps> = ({ days, onDays, t }) => (
  <div
    role="group"
    aria-label={t("stats.period.label")}
    className="inline-flex rounded-lg border border-border bg-surface-1 p-0.5"
  >
    {PERIODS.map((period) => (
      <button
        key={period}
        type="button"
        aria-pressed={period === days}
        onClick={() => onDays(period)}
        className={`min-h-[28px] rounded-md px-3 text-body tabular-nums transition-colors cursor-pointer focus-visible:focus-ring ${
          period === days
            ? "bg-text/10 font-semibold text-text"
            : "text-text-2 hover:bg-text/5"
        }`}
      >
        {t("stats.period.days", { count: period })}
      </button>
    ))}
  </div>
);

export interface StatsViewProps {
  days: Period;
  onDays: (days: Period) => void;
  /** null while loading. */
  stats: DictationStats | null;
  error: string | null;
  onCopy: () => void;
  lang: string;
  today: Date;
  t: Translate;
}

/** The page body, without effects, so it renders the same in tests. */
export const StatsView: React.FC<StatsViewProps> = ({
  days,
  onDays,
  stats,
  error,
  onCopy,
  lang,
  today,
  t,
}) => {
  const n = (v: number) => formatCount(v, lang);
  const empty = stats !== null && stats.dictations === 0;
  const average = stats ? averageWordsPerDay(stats) : null;
  return (
    <div className="w-full space-y-6">
      <PageHeader
        title={t("rail.stats")}
        description={t("stats.description")}
        actions={
          stats &&
          !empty && (
            <Button variant="secondary" onClick={onCopy}>
              <span className="flex items-center gap-2">
                <Copy size={16} strokeWidth={1.5} aria-hidden="true" />
                {t("stats.copy")}
              </span>
            </Button>
          )
        }
      />
      <PeriodPicker days={days} onDays={onDays} t={t} />
      {error && (
        <p role="alert" className="text-body text-danger">
          {t("stats.error", { error })}
        </p>
      )}
      {empty && (
        <p className="rounded-lg border border-border bg-surface-1 p-4 text-body text-text-2">
          {t("stats.empty")}
        </p>
      )}
      {stats && !empty && (
        <>
          <div className="grid grid-cols-3 gap-1 max-[840px]:grid-cols-1">
            <Figure
              label={t("stats.counts.dictations")}
              value={n(stats.dictations)}
            />
            <Figure label={t("stats.counts.words")} value={n(stats.words)} />
            <Figure
              label={t("stats.counts.perDay")}
              value={average === null ? "—" : n(average)}
            />
          </div>
          <Card title={t("stats.latency.title")}>
            <table className="w-full text-body text-text">
              <thead>
                <tr className="text-start text-caption text-text-2">
                  <th scope="col" className="pb-2 pe-3 text-start font-normal">
                    {t("stats.latency.stage")}
                  </th>
                  <th scope="col" className="pb-2 pe-3 text-start font-normal">
                    {t("stats.latency.p50")}
                  </th>
                  <th scope="col" className="pb-2 pe-3 text-start font-normal">
                    {t("stats.latency.p90")}
                  </th>
                  <th scope="col" className="pb-2 text-start font-normal">
                    {t("stats.latency.budget")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {latencyRows(stats, lang).map((row) => (
                  <tr
                    key={row.stage}
                    data-stage={row.stage}
                    className="border-t border-border align-top"
                  >
                    <th
                      scope="row"
                      className="py-2 pe-3 text-start font-normal"
                    >
                      {t(stageKey(row.stage))}
                    </th>
                    <Cell cell={row.p50} t={t} />
                    <Cell cell={row.p90} t={t} />
                    <td className="py-2 tabular-nums text-text-2">
                      {row.budget}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Card>
          <Card title={t("stats.ai.title")}>
            <dl className="grid grid-cols-3 gap-4 text-body">
              {(
                [
                  ["stats.ai.attempts", n(stats.llm_attempts)],
                  ["stats.ai.fallbacks", n(stats.fallbacks)],
                  ["stats.ai.share", fallbackShare(stats, lang)],
                ] as const
              ).map(([key, value]) => (
                <div key={key}>
                  <dt className="text-caption text-text-2">{t(key)}</dt>
                  <dd className="tabular-nums text-text">{value}</dd>
                </div>
              ))}
            </dl>
          </Card>
          <Card title={t("stats.perDay.title")}>
            <div className="flex h-32 items-end gap-px">
              {dayBars(stats.per_day, days, today).map((bar) => {
                const label = t("stats.perDay.bar", {
                  day: bar.day,
                  words: n(bar.words),
                });
                return (
                  <div
                    key={bar.day}
                    title={label}
                    aria-label={label}
                    role="img"
                    className="flex h-full flex-1 items-end"
                  >
                    <div
                      className="w-full rounded-t-sm bg-text/60"
                      style={{ height: `${Math.max(bar.height, 1)}%` }}
                    />
                  </div>
                );
              })}
            </div>
          </Card>
        </>
      )}
    </div>
  );
};

/** Desempenho: how fast dictation has been on this computer, from local metrics only. */
export const StatsPage: React.FC = () => {
  const { t, i18n } = useTranslation();
  const [days, setDays] = useState<Period>(() => readPeriod());
  const [stats, setStats] = useState<DictationStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    commands
      .getDictationStats(days)
      .then((result) => {
        if (!current) return;
        if (result.status === "ok") {
          setStats(result.data);
          setError(null);
        } else {
          setStats(null);
          setError(result.error);
        }
      })
      .catch((e) => {
        if (current) setError(String(e));
      });
    return () => {
      current = false;
    };
  }, [days]);

  const choose = (next: Period) => {
    writePeriod(next);
    setDays(next);
  };

  const copy = async () => {
    if (!stats) return;
    const copied = await copyToClipboard(
      summaryMarkdown(stats, i18n.language, t),
    );
    if (copied) toast.success(t("stats.copied"));
    else toast.error(t("stats.copyFailed"));
  };

  return (
    <StatsView
      days={days}
      onDays={choose}
      stats={stats}
      error={error}
      onCopy={copy}
      lang={i18n.language}
      today={new Date()}
      t={t}
    />
  );
};
