import { useState } from "react";
import { Search } from "lucide-react";
import type { MeetingLine } from "@/bindings";
import type { Translate } from "./meetingModel";
import {
  granolaImportDate,
  inlinePieces,
  type ParsedNotes,
} from "./notesModel";
import { SourcePopover } from "./SourcePopover";

export interface NotesViewProps {
  notes: ParsedNotes;
  segments: Map<number, MeetingLine>;
  audioAvailable: boolean;
  playing: number | null;
  onPlay: (line: MeetingLine) => void;
  onReveal: (seq: number) => void;
  /** The UI language, for the date of an import footer. */
  locale?: string;
  t: Translate;
}

/** A line's text with its inline Markdown drawn, not shown as `_` and `*`. */
function Inline({ text }: { text: string }) {
  return (
    <>
      {inlinePieces(text).map((piece, i) =>
        piece.strong ? (
          <strong key={i} className="font-semibold">
            {piece.text}
          </strong>
        ) : piece.em ? (
          <em key={i}>{piece.text}</em>
        ) : (
          piece.text
        ),
      )}
    </>
  );
}

/** "2026-10-12" as a local calendar date in the UI language. */
function importDate(iso: string, locale: string | undefined): string {
  const [year, month, day] = iso.split("-").map(Number);
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(
    new Date(year, month - 1, day),
  );
}

/**
 * The notes as a document: typed lines in `text`, generated ones in `text-2` (ADR-0017), and a
 * magnifier on each generated line that opens its sources.
 */
export function NotesView(props: NotesViewProps) {
  const { t } = props;
  const [open, setOpen] = useState<number | null>(null);
  return (
    <div className="space-y-1">
      {props.notes.blocks.map((block, index) => {
        if (block.kind === "heading") {
          return (
            <p
              key={index}
              className={`pt-2 font-semibold text-text ${block.level <= 2 ? "text-base" : "text-sm"}`}
            >
              <Inline text={block.text} />
            </p>
          );
        }
        const imported = block.generated ? null : granolaImportDate(block.text);
        if (imported !== null) {
          return (
            <p key={index} className="pt-2 text-caption text-text-2 italic">
              {t("meeting.granola.importedOn", {
                date: importDate(imported, props.locale),
              })}
            </p>
          );
        }
        return (
          <div key={index} className="group relative flex items-start gap-1">
            <p
              data-generated={block.generated}
              className={`text-sm ${block.generated ? "text-text-2" : "text-text"} ${block.bullet ? "pl-3 -indent-3" : ""}`}
            >
              {block.bullet && <span aria-hidden="true">• </span>}
              {block.literal ? block.text : <Inline text={block.text} />}
            </p>
            {block.generated && (
              <button
                type="button"
                className="shrink-0 cursor-pointer rounded p-1 text-text-2 opacity-0 group-hover:opacity-100 hover:text-text focus-visible:opacity-100 focus-visible:focus-ring aria-expanded:opacity-100"
                aria-label={t("meeting.source.open")}
                aria-expanded={open === index}
                onClick={() => setOpen(open === index ? null : index)}
              >
                <Search size={14} />
              </button>
            )}
            {open === index && (
              <SourcePopover
                sources={block.sources}
                segments={props.segments}
                annotations={props.notes.annotations}
                audioAvailable={props.audioAvailable}
                playing={props.playing}
                onPlay={props.onPlay}
                onReveal={props.onReveal}
                onClose={() => setOpen(null)}
                t={t}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}
