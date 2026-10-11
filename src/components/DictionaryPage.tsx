import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../hooks/useSettings";
import { PageHeader } from "./PageHeader";
import { SettingsGroup } from "./ui/SettingsGroup";
import { Input } from "./ui/Input";
import { CustomWords, DictionaryReplacements } from "./settings";

/** The user's own words and terms, promoted out of Advanced, and the spoken -> written pairs. */
export const DictionaryPage: React.FC = () => {
  const { t } = useTranslation();
  const { refreshSettings } = useSettings();
  const [query, setQuery] = useState("");

  // Each dictation that applies a pair adds to its count in the store; show the stored counts.
  useEffect(() => {
    void refreshSettings();
  }, [refreshSettings]);

  return (
    <div className="w-full space-y-6">
      <PageHeader
        title={t("rail.dictionary")}
        description={t("dictionary.description")}
      />
      <Input
        type="search"
        className="w-full"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={t("dictionary.search")}
        aria-label={t("dictionary.search")}
      />
      <SettingsGroup>
        <CustomWords grouped filter={query} />
      </SettingsGroup>
      <SettingsGroup>
        <DictionaryReplacements filter={query} />
      </SettingsGroup>
    </div>
  );
};
