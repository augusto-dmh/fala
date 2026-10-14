import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  events,
  type GranolaImported,
  type GranolaImportError,
  type GranolaPreview,
  type GranolaRejected,
} from "@/bindings";
import { Button } from "../ui/Button";
import { formatIsoDay } from "@/utils/dateFormat";
import type { Translate } from "./meetingModel";

/** Rejected rows listed under the preview; the rest is a count. */
const SHOWN_REJECTED = 5;

export type GranolaPhase =
  | { kind: "reading" }
  | { kind: "preview"; preview: GranolaPreview }
  | { kind: "running"; done: number; total: number }
  | { kind: "done"; imported: GranolaImported }
  | { kind: "error"; error: GranolaImportError };

/** The day Granola recorded (`YYYY-MM-DD` in its offset), as a date in the UI language. */
export const day = (rfc3339: string | null, locale: string): string =>
  formatIsoDay(rfc3339, locale);

export function rejectedLine(row: GranolaRejected, t: Translate): string {
  const reason = t(`meeting.granola.reasons.${row.reason.kind}`, {
    detail: "detail" in row.reason ? row.reason.detail : "",
  });
  return row.file === null
    ? t("meeting.granola.row", { line: row.line, reason })
    : t("meeting.granola.rowIn", { file: row.file, line: row.line, reason });
}

export function granolaError(error: GranolaImportError, t: Translate): string {
  return t(`meeting.granola.errors.${error.kind}`, {
    detail: "detail" in error ? error.detail : "",
  });
}

export interface GranolaImportViewProps {
  phase: GranolaPhase;
  onRun: () => void;
  onCancel: () => void;
  onClose: () => void;
  /** The UI language, for the dates. */
  locale: string;
  t: Translate;
}

/** What the Granola import shows in each phase: preview, progress, the end or the error. */
export function GranolaImportView({
  phase,
  onRun,
  onCancel,
  onClose,
  locale,
  t,
}: GranolaImportViewProps) {
  const close = (
    <Button size="sm" variant="secondary" onClick={onClose}>
      {t("meeting.granola.close")}
    </Button>
  );
  if (phase.kind === "reading") {
    return <p role="status">{t("meeting.granola.reading")}</p>;
  }
  if (phase.kind === "running") {
    return (
      <div className="flex items-center gap-2">
        <span role="status">
          {t("meeting.granola.importing", {
            done: phase.done,
            total: phase.total,
          })}
        </span>
        <Button size="sm" variant="secondary" onClick={onCancel}>
          {t("meeting.granola.cancel")}
        </Button>
      </div>
    );
  }
  if (phase.kind === "done") {
    return (
      <div className="flex flex-wrap items-center gap-2">
        <span role="status">
          {t("meeting.granola.done", { ...phase.imported })}
        </span>
        {close}
      </div>
    );
  }
  if (phase.kind === "error") {
    return (
      <div className="flex flex-wrap items-center gap-2">
        <span role="alert" className="text-danger">
          {granolaError(phase.error, t)}
        </span>
        {close}
      </div>
    );
  }
  const { preview } = phase;
  const hidden = preview.rejected.length - SHOWN_REJECTED;
  return (
    <div className="space-y-1">
      <p>
        {preview.meetings === 0
          ? t("meeting.granola.none")
          : t("meeting.granola.found", {
              count: preview.meetings,
              first: day(preview.first, locale),
              last: day(preview.last, locale),
            })}
      </p>
      {preview.duplicates > 0 && (
        <p className="text-caption text-text-2">
          {t("meeting.granola.duplicates", { count: preview.duplicates })}
        </p>
      )}
      {preview.rejected.length > 0 && (
        <div className="text-caption text-text-2">
          <p>
            {t("meeting.granola.skipped", { count: preview.rejected.length })}
          </p>
          <ul className="list-disc ps-5">
            {preview.rejected.slice(0, SHOWN_REJECTED).map((row) => (
              <li key={`${row.file ?? ""}:${row.line}`}>
                {rejectedLine(row, t)}
              </li>
            ))}
            {hidden > 0 && (
              <li>{t("meeting.granola.more", { count: hidden })}</li>
            )}
          </ul>
        </div>
      )}
      <p className="text-caption text-text-2">{t("meeting.granola.local")}</p>
      <div className="flex gap-2">
        <Button size="sm" disabled={preview.meetings === 0} onClick={onRun}>
          {t("meeting.granola.import")}
        </Button>
        {close}
      </div>
    </div>
  );
}

export interface GranolaImportProps {
  /** The export the person picked: a CSV or the `.zip` from the email. */
  path: string;
  /** After a run: `finished` when it ended without an error or a cancel. */
  onImported: (finished: boolean) => void;
  onClose: () => void;
}

/** "Granola history": preview the export, then import it with progress. */
export function GranolaImport({
  path,
  onImported,
  onClose,
}: GranolaImportProps) {
  const { t, i18n } = useTranslation();
  const [phase, setPhase] = useState<GranolaPhase>({ kind: "reading" });

  useEffect(() => {
    let current = true;
    setPhase({ kind: "reading" });
    void commands.previewGranolaImport(path).then((result) => {
      if (!current) return;
      setPhase(
        result.status === "ok"
          ? { kind: "preview", preview: result.data }
          : { kind: "error", error: result.error },
      );
    });
    return () => {
      current = false;
    };
  }, [path]);

  const run = async () => {
    if (phase.kind !== "preview") return;
    setPhase({ kind: "running", done: 0, total: phase.preview.meetings });
    const unlisten = await events.granolaProgress.listen(({ payload }) =>
      setPhase({ kind: "running", ...payload }),
    );
    const result = await commands.runGranolaImport(path);
    unlisten();
    setPhase(
      result.status === "ok"
        ? { kind: "done", imported: result.data }
        : { kind: "error", error: result.error },
    );
    // Even a cancelled or failed run may have saved some meetings; only a finished one may close
    // its card on its own.
    onImported(result.status === "ok");
  };

  return (
    <div className="px-4 pb-4 text-sm">
      <GranolaImportView
        phase={phase}
        onRun={() => void run()}
        onCancel={() => void commands.cancelMeetingImport()}
        onClose={onClose}
        locale={i18n.language}
        t={t}
      />
    </div>
  );
}
