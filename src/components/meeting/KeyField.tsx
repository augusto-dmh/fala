import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

export interface KeyFieldProps {
  /** Stores the key in the keyring; the field empties and never shows it again. */
  onSave: (key: string) => void;
}

export function KeyField({ onSave }: KeyFieldProps) {
  const { t } = useTranslation();
  const [key, setKey] = useState("");
  return (
    <div className="flex flex-wrap items-center gap-2 p-4 text-sm">
      <span>{t("meeting.key.label")}</span>
      <Input
        type="password"
        variant="compact"
        autoComplete="off"
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
