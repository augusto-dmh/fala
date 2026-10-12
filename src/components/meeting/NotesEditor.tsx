import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type MeetingError } from "@/bindings";
import { Button } from "../ui/Button";
import { Textarea } from "../ui/Textarea";
import { AnnotationSaver } from "./annotationSaver";
import { errorKey } from "./meetingModel";
import { editableText, mergeEdit } from "./notesModel";

export interface NotesEditorProps {
  id: string;
  /** The stored notes when editing started: unchanged lines keep their sources. */
  markdown: string;
  /** "Done": the last change is stored (`edited`: at least one); reload the session. */
  onDone: (edited: boolean) => void;
}

/** The notes as plain text, saved within two seconds like the typed notes. */
export function NotesEditor({ id, markdown, onDone }: NotesEditorProps) {
  const { t } = useTranslation();
  const [base] = useState(markdown);
  const [text, setText] = useState(() => editableText(markdown));
  // One save at a time, in order: an older text never lands after a newer one.
  const saving = useRef<Promise<boolean>>(Promise.resolve(true));
  const edited = useRef(false);
  const saver = useRef<AnnotationSaver | null>(null);
  const [error, setError] = useState<MeetingError | null>(null);

  useEffect(() => {
    const current = new AnnotationSaver((value) => {
      edited.current = true;
      saving.current = saving.current
        .then(() => commands.saveMeetingNotes(id, mergeEdit(base, value)))
        .then((result) => {
          setError(result.status === "error" ? result.error : null);
          return result.status === "ok";
        });
    }, editableText(base));
    saver.current = current;
    return () => current.flush();
  }, [id, base]);

  const done = async () => {
    saver.current?.flush();
    // A failed save keeps the editor open with the error and the text.
    if (await saving.current) onDone(edited.current);
  };

  return (
    <div className="space-y-2">
      <Textarea
        aria-label={t("meeting.notesEdit.label")}
        className="min-h-64 w-full"
        value={text}
        onChange={(event) => {
          setText(event.target.value);
          saver.current?.change(event.target.value);
        }}
        onBlur={() => saver.current?.flush()}
      />
      <p className="text-caption text-text-2">{t("meeting.notesEdit.hint")}</p>
      {error !== null && (
        <p role="alert" className="text-caption text-danger">
          {t(errorKey(error))}
        </p>
      )}
      <Button variant="secondary" size="sm" onClick={() => void done()}>
        {t("meeting.notesEdit.done")}
      </Button>
    </div>
  );
}
