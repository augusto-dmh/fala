import { useState } from "react";
import type { MeetingPerson } from "@/bindings";
import { Button } from "../ui/Button";
import { Dropdown } from "../ui/Dropdown";
import { Input } from "../ui/Input";
import type { Translate } from "./meetingModel";
import { nameFold } from "./nameSuggest";

export interface PersonEditProps {
  person: MeetingPerson;
  /** Everyone in People, for "Merge with…". */
  people: MeetingPerson[];
  /** Every name of `person` becomes `name` in all meetings. */
  onRename: (key: string, name: string) => void;
  onClose: () => void;
  /** Opens on this step; the tests pin it. */
  start?: Step;
  t: Translate;
}

export type Step =
  | { kind: "menu" }
  | { kind: "rename" }
  | { kind: "merge"; into: MeetingPerson | null };

/**
 * "Juntar com…" and "Renomear" for one person. Merging asks first: the name changes in each of
 * their meetings, and nothing remembers the old spelling.
 */
export function PersonEdit({
  person,
  people,
  onRename,
  onClose,
  start = { kind: "menu" },
  t,
}: PersonEditProps) {
  const [step, setStep] = useState<Step>(start);
  const [typed, setTyped] = useState(person.name);
  const others = people.filter((p) => p.key !== person.key);
  const cancel = (
    <Button size="sm" variant="secondary" onClick={onClose}>
      {t("meeting.directory.cancel")}
    </Button>
  );

  if (step.kind === "rename") {
    const name = typed.trim();
    // A name that is someone else's merges the two: it asks like "Juntar com…".
    const clash = others.find((p) => nameFold(p.name) === nameFold(name));
    return (
      <form
        className="flex flex-wrap items-center gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          if (name) onRename(person.key, name);
        }}
      >
        <Input
          variant="compact"
          autoFocus
          aria-label={t("meeting.directory.newName")}
          value={typed}
          onChange={(event) => setTyped(event.target.value)}
        />
        <Button size="sm" type="submit" disabled={!name}>
          {t(
            clash
              ? "meeting.directory.mergeAction"
              : "meeting.directory.rename",
          )}
        </Button>
        {cancel}
        {clash && (
          <p className="w-full text-body">
            {t("meeting.directory.mergeConfirm", {
              from: person.name,
              to: name,
              count: person.meetings,
            })}
          </p>
        )}
      </form>
    );
  }

  if (step.kind === "merge") {
    const into = step.into;
    return (
      <div className="space-y-2">
        <Dropdown
          options={others.map((p) => ({ value: p.key, label: p.name }))}
          selectedValue={into?.key ?? null}
          placeholder={t("meeting.directory.pick")}
          onSelect={(key) =>
            setStep({
              kind: "merge",
              into: others.find((p) => p.key === key) ?? null,
            })
          }
        />
        {into && (
          <p className="text-body">
            {t("meeting.directory.mergeConfirm", {
              from: person.name,
              to: into.name,
              count: person.meetings,
            })}
          </p>
        )}
        <div className="flex gap-2">
          <Button
            size="sm"
            disabled={!into}
            onClick={() => into && onRename(person.key, into.name)}
          >
            {t("meeting.directory.mergeAction")}
          </Button>
          {cancel}
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-wrap gap-2">
      <Button
        size="sm"
        variant="secondary"
        disabled={others.length === 0}
        onClick={() => setStep({ kind: "merge", into: null })}
      >
        {t("meeting.directory.merge")}
      </Button>
      <Button
        size="sm"
        variant="secondary"
        onClick={() => setStep({ kind: "rename" })}
      >
        {t("meeting.directory.rename")}
      </Button>
      {cancel}
    </div>
  );
}
