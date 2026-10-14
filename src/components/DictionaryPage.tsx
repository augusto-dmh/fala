import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../hooks/useSettings";
import { PageHeader } from "./PageHeader";
import { SearchField } from "./ui/SearchField";
import { DictionaryList } from "./DictionaryList";

/** The user's own words and terms, the spoken -> written pairs and the snippets, in one list. */
export const DictionaryPage: React.FC = () => {
  const { t } = useTranslation();
  const { refreshSettings } = useSettings();
  const [query, setQuery] = useState("");

  // Each dictation that applies a pair or a snippet adds to its count in the store; show the
  // stored counts.
  useEffect(() => {
    void refreshSettings();
  }, [refreshSettings]);

  return (
    <div className="w-full space-y-6">
      <PageHeader
        title={t("rail.dictionary")}
        description={t("dictionary.description")}
      />
      <SearchField
        className="w-full"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={t("dictionary.search")}
        aria-label={t("dictionary.search")}
      />
      <DictionaryList filter={query} />
    </div>
  );
};
