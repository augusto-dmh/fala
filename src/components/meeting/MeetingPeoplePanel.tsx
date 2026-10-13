import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type MeetingError,
  type MeetingPeople,
  type SpeakerSuggestion,
} from "@/bindings";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { errorKey, parseParticipants } from "./meetingModel";

export interface MeetingPeoplePanelProps {
  id: string;
  /**
   * Changes whenever the session is reloaded: a generation may bring suggestions, a new
   * transcription removes them.
   */
  reloadOn: unknown;
  /** "Participants" was picked in the menu: the names field shows; suggestions always do. */
  expanded?: boolean;
  /** A name was applied: the transcript labels changed. */
  onNamesChanged?: () => void;
}

/**
 * "Who was there" (names only) and, after the notes, one suggestion per "Person N" with its
 * reason. Nothing is applied without the click (ADR-0016).
 */
export function MeetingPeoplePanel({
  id,
  reloadOn,
  expanded = true,
  onNamesChanged,
}: MeetingPeoplePanelProps) {
  const { t } = useTranslation();
  const [people, setPeople] = useState<MeetingPeople | null>(null);
  const [typed, setTyped] = useState("");
  const [error, setError] = useState<MeetingError | null>(null);
  // While the field has focus, a reload does not overwrite what is being typed.
  const editing = useRef(false);

  useEffect(() => {
    let current = true;
    void commands.meetingPeople(id).then((result) => {
      if (!current) return;
      if (result.status === "ok") {
        setPeople(result.data);
        if (!editing.current) setTyped(result.data.participants.join(", "));
      } else setError(result.error);
    });
    return () => {
      current = false;
    };
  }, [id, reloadOn]);

  const save = async () => {
    editing.current = false;
    // Before the first load there is nothing to compare with: saving would erase the list.
    if (people === null) return;
    const names = parseParticipants(typed);
    if (names.join("\n") === people.participants.join("\n")) return;
    const result = await commands.setMeetingParticipants(id, names);
    if (result.status === "ok") {
      setError(null);
      setPeople(result.data);
      setTyped(result.data.participants.join(", "));
    } else setError(result.error);
  };

  const apply = async (person: number) => {
    const result = await commands.applySpeakerSuggestion(id, person);
    if (result.status === "ok") {
      setError(null);
      setPeople(result.data);
      onNamesChanged?.();
      return;
    }
    setError(result.error);
    // The suggestion may be gone (a new transcription): show what is stored now.
    const fresh = await commands.meetingPeople(id);
    if (fresh.status === "ok") setPeople(fresh.data);
  };

  return (
    <div className="space-y-2">
      {expanded && (
        <label className="flex flex-col gap-1 text-sm">
          <span>{t("meeting.people.participants")}</span>
          <Input
            value={typed}
            placeholder={t("meeting.people.placeholder")}
            onChange={(event) => setTyped(event.target.value)}
            onFocus={() => {
              editing.current = true;
            }}
            onBlur={() => void save()}
          />
          <span className="text-caption text-text-2">
            {t("meeting.people.hint")}
          </span>
        </label>
      )}
      {error !== null && (
        <p role="alert" className="text-sm text-red-500">
          {t(errorKey(error))}
        </p>
      )}
      {people && (
        <SuggestionList
          suggestions={people.suggestions}
          onApply={(person) => void apply(person)}
        />
      )}
    </div>
  );
}

export interface SuggestionListProps {
  suggestions: SpeakerSuggestion[];
  onApply: (person: number) => void;
}

/** One line per "Person N": the suggested name, "Apply" and the reason under it. */
export function SuggestionList({ suggestions, onApply }: SuggestionListProps) {
  const { t } = useTranslation();
  if (suggestions.length === 0) return null;
  return (
    <ul className="space-y-1" aria-label={t("meeting.people.suggestions")}>
      {suggestions.map((suggestion) => (
        <li
          key={suggestion.person}
          className="flex flex-wrap items-center gap-2 text-sm"
        >
          <span>
            {t("meeting.people.suggestion", {
              person: t("meeting.speaker.person", { n: suggestion.person }),
              name: suggestion.name,
            })}
          </span>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => onApply(suggestion.person)}
          >
            {t("meeting.people.apply")}
          </Button>
          {suggestion.reason && (
            <span className="w-full text-caption text-text-2">
              {suggestion.reason}
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}
