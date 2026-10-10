import React from "react";
import { useTranslation } from "react-i18next";
import { PageHeader } from "./PageHeader";
import { SettingsGroup } from "./ui/SettingsGroup";
import { CustomWords } from "./settings";

/** The user's own words and terms, promoted out of Advanced. */
export const DictionaryPage: React.FC = () => {
  const { t } = useTranslation();
  return (
    <div className="w-full space-y-6">
      <PageHeader
        title={t("rail.dictionary")}
        description={t("dictionary.description")}
      />
      <SettingsGroup>
        <CustomWords grouped />
      </SettingsGroup>
    </div>
  );
};
