// Where Settings can go, and which pages the current settings make reachable.
// Pure, so the navigation rules are tested without rendering.

/** The tabs at the top of Settings. */
export type SettingsTab = "general" | "advanced" | "about";
/** Pages one level down, under Advanced. */
export type AdvancedPage = "models" | "postprocessing" | "debug";
export type SettingsView = SettingsTab | AdvancedPage;

export const SETTINGS_TABS: readonly SettingsTab[] = [
  "general",
  "advanced",
  "about",
];
export const ADVANCED_PAGES: readonly AdvancedPage[] = [
  "models",
  "postprocessing",
  "debug",
];

interface NavSettings {
  post_process_enabled?: boolean | null;
  debug_mode?: boolean | null;
}

export const isAdvancedPage = (view: SettingsView): view is AdvancedPage =>
  (ADVANCED_PAGES as readonly string[]).includes(view);

/** Models is always there; post-processing and debug only when switched on. */
export const visibleAdvancedPages = (
  settings: NavSettings | null | undefined,
): AdvancedPage[] =>
  ADVANCED_PAGES.filter((page) => {
    if (page === "postprocessing")
      return settings?.post_process_enabled === true;
    if (page === "debug") return settings?.debug_mode === true;
    return true;
  });

export interface ResolvedView {
  /** The page actually shown. */
  current: SettingsView;
  /** The tab marked as selected (Advanced for its sub-pages). */
  tab: SettingsTab;
  /** The sub-page under Advanced, when one is shown. */
  subPage: AdvancedPage | null;
}

/** A sub-page that is no longer reachable (debug mode turned off) falls back
 *  to Advanced. */
export const resolveSettingsView = (
  view: SettingsView,
  settings: NavSettings | null | undefined,
): ResolvedView => {
  if (isAdvancedPage(view)) {
    if (!visibleAdvancedPages(settings).includes(view)) {
      return { current: "advanced", tab: "advanced", subPage: null };
    }
    return { current: view, tab: "advanced", subPage: view };
  }
  return { current: view, tab: view, subPage: null };
};
