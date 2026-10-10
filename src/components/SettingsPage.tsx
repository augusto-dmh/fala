import React from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronRight,
  Cpu,
  FlaskConical,
  Sparkles,
  type LucideIcon,
} from "lucide-react";
import { useSettings } from "../hooks/useSettings";
import { PageHeader } from "./PageHeader";
import { SettingsGroup } from "./ui/SettingsGroup";
import {
  AboutSettings,
  AdvancedSettings,
  DebugSettings,
  GeneralSettings,
  ModelsSettings,
  PostProcessingSettings,
  type OnboardingPreviewStep,
} from "./settings";
import {
  SETTINGS_TABS,
  resolveSettingsView,
  visibleAdvancedPages,
  type AdvancedPage,
  type SettingsView,
} from "./settingsNav";

const TAB_LABELS = {
  general: "settingsPage.tabs.general",
  advanced: "settingsPage.tabs.advanced",
  about: "settingsPage.tabs.about",
} as const;

const PAGE_INFO: Record<
  AdvancedPage,
  { labelKey: string; descriptionKey: string; icon: LucideIcon }
> = {
  models: {
    labelKey: "settingsPage.advancedPages.models.title",
    descriptionKey: "settingsPage.advancedPages.models.description",
    icon: Cpu,
  },
  postprocessing: {
    labelKey: "settingsPage.advancedPages.postProcessing.title",
    descriptionKey: "settingsPage.advancedPages.postProcessing.description",
    icon: Sparkles,
  },
  debug: {
    labelKey: "settingsPage.advancedPages.debug.title",
    descriptionKey: "settingsPage.advancedPages.debug.description",
    icon: FlaskConical,
  },
};

interface SettingsPageProps {
  view: SettingsView;
  onNavigate: (view: SettingsView) => void;
  onPreviewOnboarding: (step: OnboardingPreviewStep) => void;
}

/** Configurações: tabs General, Advanced and About; Models, post-processing
 *  and Debug live one level down, under Advanced. */
export const SettingsPage: React.FC<SettingsPageProps> = ({
  view,
  onNavigate,
  onPreviewOnboarding,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();
  const { current, tab, subPage } = resolveSettingsView(view, settings);

  return (
    <div className="w-full space-y-6">
      <PageHeader
        title={t("rail.settings")}
        breadcrumb={
          subPage
            ? {
                parent: t(TAB_LABELS.advanced),
                onParent: () => onNavigate("advanced"),
                current: t(PAGE_INFO[subPage].labelKey),
              }
            : undefined
        }
      />
      {!subPage && (
        <div
          role="tablist"
          aria-label={t("rail.settings")}
          className="flex gap-1 border-b border-border"
        >
          {SETTINGS_TABS.map((id) => {
            const selected = tab === id;
            return (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={selected}
                onClick={() => onNavigate(id)}
                className={`relative h-9 px-3 text-body cursor-pointer rounded-t-lg transition-colors focus-visible:focus-ring ${
                  selected
                    ? "text-text font-semibold after:absolute after:start-3 after:end-3 after:bottom-0 after:h-[3px] after:rounded-full after:bg-accent"
                    : "text-text-2 hover:text-text"
                }`}
              >
                {t(TAB_LABELS[id])}
              </button>
            );
          })}
        </div>
      )}
      {current === "general" && <GeneralSettings />}
      {current === "advanced" && (
        <>
          <AdvancedSettings />
          <div className="max-w-3xl w-full mx-auto">
            <SettingsGroup title={t("settingsPage.advancedPages.title")}>
              {visibleAdvancedPages(settings).map((page) => {
                const {
                  labelKey,
                  descriptionKey,
                  icon: Icon,
                } = PAGE_INFO[page];
                return (
                  <button
                    key={page}
                    type="button"
                    onClick={() => onNavigate(page)}
                    className="flex items-center gap-3 w-full min-h-[68px] px-4 py-2 text-start cursor-pointer transition-colors hover:bg-surface-2 focus-visible:focus-ring"
                  >
                    <Icon
                      size={20}
                      strokeWidth={1.5}
                      className="shrink-0 text-text-2"
                      aria-hidden="true"
                    />
                    <span className="flex-1 min-w-0">
                      <span className="block text-body text-text">
                        {t(labelKey)}
                      </span>
                      <span className="block text-caption text-text-2">
                        {t(descriptionKey)}
                      </span>
                    </span>
                    <ChevronRight
                      size={16}
                      strokeWidth={1.5}
                      className="shrink-0 text-text-2 rtl:rotate-180"
                      aria-hidden="true"
                    />
                  </button>
                );
              })}
            </SettingsGroup>
          </div>
        </>
      )}
      {current === "models" && <ModelsSettings />}
      {current === "postprocessing" && <PostProcessingSettings />}
      {current === "debug" && (
        <DebugSettings onPreviewOnboarding={onPreviewOnboarding} />
      )}
      {current === "about" && <AboutSettings />}
    </div>
  );
};
