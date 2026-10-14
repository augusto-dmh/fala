import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingContainer } from "../ui/SettingContainer";
import { Button } from "../ui/Button";
import { useSettings } from "../../hooks/useSettings";
import { useOsType } from "../../hooks/useOsType";
import { KeyChips, ShortcutsDialog } from "./ShortcutsDialog";

/** The General row: the dictation keys as chips and the button that opens the dialog. */
export const KeyboardShortcuts: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const osType = useOsType();
  const [open, setOpen] = useState(false);
  const dictate = getSetting("bindings")?.transcribe;

  return (
    <SettingContainer
      title={t("settings.shortcuts.row.title")}
      description={t("settings.shortcuts.row.description")}
      grouped={true}
    >
      <div className="flex items-center gap-3">
        {dictate?.current_binding && (
          <KeyChips combo={dictate.current_binding} osType={osType} />
        )}
        <Button variant="secondary" onClick={() => setOpen(true)}>
          {t("settings.shortcuts.row.edit")}
        </Button>
      </div>
      <ShortcutsDialog open={open} onOpenChange={setOpen} />
    </SettingContainer>
  );
};
