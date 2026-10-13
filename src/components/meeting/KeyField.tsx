import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

export interface KeyFieldProps {
  /** Stores the key in the keyring; the field empties and never shows it again. */
  onSave: (key: string) => void;
  /** The field's name for screen readers; the ElevenLabs key by default. */
  label?: string;
}

export function KeyField({ onSave, label }: KeyFieldProps) {
  const { t } = useTranslation();
  const [key, setKey] = useState("");
  return (
    <div className="flex flex-wrap items-center gap-2 text-sm">
      <Input
        type="password"
        variant="compact"
        autoComplete="off"
        aria-label={label ?? t("meeting.key.label")}
        value={key}
        placeholder={t("meeting.key.placeholder")}
        onChange={(event) => setKey(event.target.value)}
      />
      <Button
        size="sm"
        disabled={key.trim() === ""}
        onClick={() => {
          onSave(key);
          setKey("");
        }}
      >
        {t("meeting.key.save")}
      </Button>
    </div>
  );
}
