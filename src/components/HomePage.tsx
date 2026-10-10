import React from "react";
import { useTranslation } from "react-i18next";
import { PageHeader } from "./PageHeader";
import { HistorySettings } from "./settings";

/** Início: what was dictated, grouped by day, with search and "Recuperar". */
export const HomePage: React.FC = () => {
  const { t } = useTranslation();
  return (
    <div className="w-full space-y-6">
      <PageHeader title={t("rail.home")} />
      <HistorySettings />
    </div>
  );
};
