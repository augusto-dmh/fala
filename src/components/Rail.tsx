import React from "react";
import { useTranslation } from "react-i18next";
import { BookA, House, Settings, type LucideIcon } from "lucide-react";
import ModelSelector from "./model-selector";

export type RailDestination = "home" | "dictionary" | "settings";

interface RailItemConfig {
  id: RailDestination;
  labelKey: string;
  icon: LucideIcon;
}

/** Content destinations at the top of the rail; settings sits in the footer. */
export const RAIL_ITEMS: readonly RailItemConfig[] = [
  { id: "home", labelKey: "rail.home", icon: House },
  { id: "dictionary", labelKey: "rail.dictionary", icon: BookA },
];

const SETTINGS_ITEM: RailItemConfig = {
  id: "settings",
  labelKey: "rail.settings",
  icon: Settings,
};

const WORDMARK = "Fala";

/** The pill in miniature: a black capsule with white bars, as on screen. */
const PillMark: React.FC = () => (
  <svg
    width="28"
    height="12"
    viewBox="0 0 28 12"
    aria-hidden="true"
    className="shrink-0"
  >
    <rect width="28" height="12" rx="6" className="fill-black" />
    {[3, 5, 8, 5, 3].map((h, i) => (
      <rect
        key={i}
        x={7 + i * 3}
        y={6 - h / 2}
        width="2"
        height={h}
        rx="1"
        className="fill-white"
      />
    ))}
  </svg>
);

interface RailButtonProps {
  item: RailItemConfig;
  active: boolean;
  onSelect: (id: RailDestination) => void;
}

const RailButton: React.FC<RailButtonProps> = ({ item, active, onSelect }) => {
  const { t } = useTranslation();
  const Icon = item.icon;
  const label = t(item.labelKey);
  return (
    <button
      type="button"
      onClick={() => onSelect(item.id)}
      aria-current={active ? "page" : undefined}
      aria-label={label}
      title={label}
      className={`relative flex items-center gap-3 w-full h-9 px-3 rounded-lg text-body text-text transition-colors cursor-pointer focus-visible:focus-ring max-[840px]:justify-center max-[840px]:px-0 ${
        active
          ? "bg-text/5 font-semibold before:absolute before:start-0 before:top-1/2 before:-translate-y-1/2 before:h-4 before:w-[3px] before:rounded-full before:bg-accent"
          : "hover:bg-text/5"
      }`}
    >
      <Icon size={20} strokeWidth={1.5} className="shrink-0" />
      <span className="truncate max-[840px]:hidden">{label}</span>
    </button>
  );
};

interface RailProps {
  active: RailDestination;
  onSelect: (id: RailDestination) => void;
  onOpenModels: () => void;
}

/** Left navigation: 220 px, collapsing to 48 px of icons below 840 px. */
export const Rail: React.FC<RailProps> = ({
  active,
  onSelect,
  onOpenModels,
}) => {
  const { t } = useTranslation();
  return (
    <nav
      aria-label={t("rail.navigation")}
      className="flex flex-col shrink-0 w-[220px] max-[840px]:w-[48px] h-full px-1 py-3 border-e border-border bg-surface-0"
    >
      <div className="flex items-center gap-2 h-9 px-3 mb-2 max-[840px]:justify-center max-[840px]:px-0">
        <PillMark />
        <span className="font-display text-subtitle font-semibold text-text max-[840px]:hidden">
          {WORDMARK}
        </span>
      </div>
      <div className="flex flex-col gap-1">
        {RAIL_ITEMS.map((item) => (
          <RailButton
            key={item.id}
            item={item}
            active={active === item.id}
            onSelect={onSelect}
          />
        ))}
      </div>
      <div className="mt-auto flex flex-col gap-1 pt-2">
        <div className="px-3 py-1 text-caption text-text-2 max-[840px]:px-0 max-[840px]:flex max-[840px]:justify-center">
          <ModelSelector variant="status" onOpen={onOpenModels} />
        </div>
        <RailButton
          item={SETTINGS_ITEM}
          active={active === "settings"}
          onSelect={onSelect}
        />
      </div>
    </nav>
  );
};
