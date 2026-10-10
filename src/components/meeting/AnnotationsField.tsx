import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { Textarea } from "../ui/Textarea";
import { AnnotationSaver } from "./annotationSaver";

export interface AnnotationsFieldProps {
  id: string;
  /** The text already stored for the session (the agenda typed in a draft, for instance). */
  initial: string;
  label: string;
}

/** The notes typed before and during the call, saved to disk within two seconds. */
export function AnnotationsField({
  id,
  initial,
  label,
}: AnnotationsFieldProps) {
  const { t } = useTranslation();
  const [text, setText] = useState(initial);
  const saver = useRef<AnnotationSaver | null>(null);

  useEffect(() => {
    setText(initial);
    const current = new AnnotationSaver((value) => {
      void commands.saveMeetingAnnotations(id, value);
    }, initial);
    saver.current = current;
    return () => current.flush();
  }, [id, initial]);

  return (
    <label className="flex flex-col gap-1 text-sm">
      <span>{label}</span>
      <Textarea
        value={text}
        placeholder={t("meeting.annotations.placeholder")}
        onChange={(event) => {
          setText(event.target.value);
          saver.current?.change(event.target.value);
        }}
        onBlur={() => saver.current?.flush()}
      />
    </label>
  );
}
