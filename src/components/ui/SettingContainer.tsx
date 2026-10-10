import React from "react";

interface SettingContainerProps {
  title: string;
  description: string;
  children: React.ReactNode;
  /** Kept for API compatibility: the description is always visible now. */
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  layout?: "horizontal" | "stacked";
  disabled?: boolean;
  /** Kept for API compatibility: there is no tooltip any more. */
  tooltipPosition?: "top" | "bottom";
  /** Optional 20 px icon drawn to the left of the title. */
  icon?: React.ReactNode;
}

/** A settings row drawn like a Windows 11 settings card: optional icon, title
 *  and an always-visible description on the left, the control on the right. */
export const SettingContainer: React.FC<SettingContainerProps> = ({
  title,
  description,
  children,
  grouped = false,
  layout = "horizontal",
  disabled = false,
  icon,
}) => {
  const hasDescription = Boolean(description && description.trim());
  // Inside a SettingsGroup the group draws the card; alone, the row is one.
  const card = grouped ? "" : "bg-surface-1 border border-border rounded-lg";
  const dim = disabled ? "opacity-50" : "";

  const label = (
    <div className={`flex items-center gap-3 min-w-0 ${dim}`}>
      {icon && (
        <span
          className="flex shrink-0 items-center justify-center w-5 h-5 text-text-2 [&>svg]:w-5 [&>svg]:h-5"
          aria-hidden="true"
        >
          {icon}
        </span>
      )}
      <div className="min-w-0">
        <h3 className="text-body text-text">{title}</h3>
        {hasDescription && (
          <p className="text-caption text-text-2">{description}</p>
        )}
      </div>
    </div>
  );

  if (layout === "stacked") {
    return (
      <div className={`flex flex-col gap-3 px-4 py-3 ${card}`}>
        {label}
        <div className="w-full">{children}</div>
      </div>
    );
  }

  return (
    <div
      className={`flex items-center justify-between gap-4 px-4 py-2 ${
        hasDescription ? "min-h-[68px]" : "min-h-[56px]"
      } ${card}`}
    >
      <div className="max-w-2/3">{label}</div>
      <div className="relative">{children}</div>
    </div>
  );
};
