import { useEffect, useState, type KeyboardEvent } from "react";
import { commands, type MeetingPerson } from "@/bindings";
import { stepOption } from "../ui/Dropdown";
import { fold as unaccent } from "./transcriptSearch";

/** At most this many names under the field. */
const MAX_SUGGESTIONS = 5;

/** A name without case, accents or extra spaces, close to `fala-storage`'s `person_key`. */
export const nameFold = (text: string) =>
  unaccent(text).trim().replace(/\s+/g, " ");

/**
 * The people whose name starts with `typed` (without case or accents), latest meeting first,
 * leaving out the names in `taken` and one typed in full.
 */
export function suggestNames(
  people: MeetingPerson[],
  typed: string,
  taken: string[] = [],
): string[] {
  const start = nameFold(typed);
  if (!start) return [];
  const skip = new Set([...taken.map(nameFold), start]);
  return people
    .filter(
      (p) => nameFold(p.name).startsWith(start) && !skip.has(nameFold(p.name)),
    )
    .slice(0, MAX_SUGGESTIONS)
    .map((p) => p.name);
}

/**
 * Names from People while a field is `active`: the list to show under it and the keys it
 * takes (↑/↓ pick, Enter takes the picked one, and the field keeps Enter otherwise). A click
 * takes a name without blurring the field.
 */
export function useNameSuggest(
  active: boolean,
  typed: string,
  onPick: (name: string) => void,
  taken: string[] = [],
) {
  const [people, setPeople] = useState<MeetingPerson[]>([]);
  const [picked, setPicked] = useState(-1);
  useEffect(() => {
    if (!active) return;
    let current = true;
    void commands.meetingDirectory().then((result) => {
      if (current && result.status === "ok") setPeople(result.data);
    });
    return () => {
      current = false;
    };
  }, [active]);
  useEffect(() => setPicked(-1), [typed]);
  const names = active ? suggestNames(people, typed, taken) : [];

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    // The keys belong to an IME while it composes.
    if (names.length === 0 || event.nativeEvent.isComposing) return;
    const next = stepOption(
      names.map(() => ({})),
      picked,
      event.key,
    );
    if (
      next !== null &&
      (event.key === "ArrowDown" || event.key === "ArrowUp")
    ) {
      event.preventDefault();
      setPicked(next);
    } else if (event.key === "Enter" && names[picked] !== undefined) {
      event.preventDefault();
      onPick(names[picked]);
    }
  };
  const list = names.length > 0 && (
    <SuggestedNames names={names} picked={picked} onPick={onPick} />
  );
  return { list, onKeyDown };
}

export interface SuggestedNamesProps {
  names: string[];
  picked: number;
  onPick: (name: string) => void;
}

export function SuggestedNames({ names, picked, onPick }: SuggestedNamesProps) {
  return (
    <ul role="listbox" className="flex flex-wrap gap-1">
      {names.map((name, i) => (
        <li
          key={name}
          role="option"
          aria-selected={i === picked}
          // Before the blur: the field stays open and keeps what was typed.
          onMouseDown={(event) => {
            event.preventDefault();
            onPick(name);
          }}
          className={`cursor-pointer rounded-md border border-border px-2 py-0.5 text-caption ${
            i === picked
              ? "bg-text/10 text-text"
              : "text-text-2 hover:bg-text/5"
          }`}
        >
          {name}
        </li>
      ))}
    </ul>
  );
}
