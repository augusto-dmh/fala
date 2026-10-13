import { useEffect, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { Textarea } from "../ui/Textarea";
import { AnnotationSaver } from "./annotationSaver";

export interface AnnotationsFieldProps {
  id: string;
  /** The text already stored for the session (the agenda typed in a draft, for instance). */
  initial: string;
  label: string;
  /** The tab already names the field: the label is for screen readers only. */
  labelHidden?: boolean;
  /** Shown while the field holds just "/"; `clear` empties the field. */
  slashMenu?: (clear: () => void) => ReactNode;
  /** Every change of the text, before it is saved. */
  onTextChange?: (text: string) => void;
}

/** The notes typed before and during the call, saved to disk within two seconds. */
export function AnnotationsField({
  id,
  initial,
  label,
  labelHidden = false,
  slashMenu,
  onTextChange,
}: AnnotationsFieldProps) {
  const { t } = useTranslation();
  const [text, setText] = useState(initial);
  const saver = useRef<AnnotationSaver | null>(null);
  const stored = useRef(initial);
  stored.current = initial;

  // Only another session resets the field: a reload of this one (a pause, the stop) brings
  // back what this field saved, maybe older than the last two seconds of typing.
  useEffect(() => {
    setText(stored.current);
    const current = new AnnotationSaver((value) => {
      void commands.saveMeetingAnnotations(id, value);
    }, stored.current);
    saver.current = current;
    return () => current.flush();
  }, [id]);
  const change = (value: string) => {
    setText(value);
    onTextChange?.(value);
    saver.current?.change(value);
  };

  return (
    <div className="flex flex-col gap-1">
      <label className="flex flex-col gap-1 text-sm">
        <span className={labelHidden ? "sr-only" : undefined}>{label}</span>
        <Textarea
          value={text}
          placeholder={t("meeting.annotations.placeholder")}
          onChange={(event) => change(event.target.value)}
          onBlur={() => saver.current?.flush()}
        />
      </label>
      {slashMenu && text === "/" && slashMenu(() => change(""))}
    </div>
  );
}
