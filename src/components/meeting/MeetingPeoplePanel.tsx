import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type MeetingPeople,
  type SpeakerSuggestion,
} from "@/bindings";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { parseParticipants } from "./meetingModel";

export interface MeetingPeoplePanelProps {
  id: string;
  /** The stored notes: a new generation may bring new suggestions. */
  notes: string | null;
  /** A name was applied: the transcript labels changed. */
  onNamesChanged?: () => void;
}

/**
 * "Who was there" (names only) and, after the notes, one suggestion per "Person N" with its
 * reason. Nothing is applied without the click (ADR-0016).
 */
export function MeetingPeoplePanel({
  id,
  notes,
  onNamesChanged,
}: MeetingPeoplePanelProps) {
  const { t } = useTranslation();
  const [people, setPeople] = useState<MeetingPeople | null>(null);
  const [typed, setTyped] = useState("");

  useEffect(() => {
    let current = true;
    void commands.meetingPeople(id).then((result) => {
      if (current && result.status === "ok") {
        setPeople(result.data);
        setTyped(result.data.participants.join(", "));
      }
    });
    return () => {
      current = false;
    };
  }, [id, notes]);

  const save = async () => {
    const names = parseParticipants(typed);
    if (people && names.join("\n") === people.participants.join("\n")) return;
    const result = await commands.setMeetingParticipants(id, names);
    if (result.status === "ok") {
      setPeople(result.data);
      setTyped(result.data.participants.join(", "));
    }
  };

  const apply = async (person: number) => {
    const result = await commands.applySpeakerSuggestion(id, person);
    if (result.status === "ok") {
      setPeople(result.data);
      onNamesChanged?.();
    }
  };

  return (
    <div className="space-y-2">
      <label className="flex flex-col gap-1 text-sm">
        <span>{t("meeting.people.participants")}</span>
        <Input
          value={typed}
          placeholder={t("meeting.people.placeholder")}
          onChange={(event) => setTyped(event.target.value)}
          onBlur={() => void save()}
        />
        <span className="text-caption text-text-2">
          {t("meeting.people.hint")}
        </span>
      </label>
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
