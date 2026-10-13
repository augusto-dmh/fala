import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

export interface KeyFieldProps {
  /** Stores the key in the keyring; the field empties and never shows it again. A promise of
   *  `false` (the save failed) keeps what was typed. */
  onSave: (key: string) => void | Promise<boolean>;
  /** The field's name for screen readers; the ElevenLabs key by default. */
  label?: string;
  /** A key is stored: the field shows "Saved" and [Replace] instead of the input. */
  saved?: boolean;
  /** Forgets the stored key; without it, a saved key can only be replaced. */
  onRemove?: () => void;
  disabled?: boolean;
}

/** One way to enter every API key: paste and save, then only "Saved" with [Replace]. */
export function KeyField({
  onSave,
  label,
  saved = false,
  onRemove,
  disabled = false,
}: KeyFieldProps) {
  const { t } = useTranslation();
  const [key, setKey] = useState("");
  const [replacing, setReplacing] = useState(false);
  const name = label ?? t("meeting.key.label");

  if (saved && !replacing) {
    return (
      <div className="flex flex-wrap items-center gap-2 text-sm">
        <span className="inline-flex items-center gap-1 text-text-2">
          <Check className="h-4 w-4 text-ok" aria-hidden="true" />
          {t("keyField.saved")}
        </span>
        <Button
          size="sm"
          variant="secondary"
          disabled={disabled}
          aria-label={t("keyField.replaceNamed", { name })}
          onClick={() => setReplacing(true)}
        >
          {t("keyField.replace")}
        </Button>
        {onRemove && (
          <Button
            size="sm"
            variant="secondary"
            disabled={disabled}
            aria-label={t("keyField.removeNamed", { name })}
            onClick={onRemove}
          >
            {t("keyField.remove")}
          </Button>
        )}
      </div>
    );
  }

  const save = async () => {
    if ((await onSave(key.trim())) === false) return;
    setKey("");
    setReplacing(false);
  };

  return (
    <div className="flex flex-wrap items-center gap-2 text-sm">
      <Input
        type="password"
        variant="compact"
        autoComplete="off"
        aria-label={name}
        // [Replace] leaves the page: the focus goes to the field that took its place.
        autoFocus={replacing}
        value={key}
        disabled={disabled}
        placeholder={t("keyField.placeholder")}
        onChange={(event) => setKey(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && key.trim() !== "") {
            event.preventDefault();
            void save();
          }
        }}
      />
      <Button
        size="sm"
        disabled={disabled || key.trim() === ""}
        onClick={() => void save()}
      >
        {t("keyField.save")}
      </Button>
      {replacing && (
        <Button
          size="sm"
          variant="secondary"
          onClick={() => {
            setKey("");
            setReplacing(false);
          }}
        >
          {t("keyField.cancel")}
        </Button>
      )}
    </div>
  );
}
