import React from "react";

type ModelStatus =
  | "ready"
  | "loading"
  | "downloading"
  | "verifying"
  | "extracting"
  | "error"
  | "unloaded"
  | "none";

interface ModelStatusButtonProps {
  status: ModelStatus;
  displayText: string;
  isDropdownOpen: boolean;
  onClick: () => void;
  className?: string;
  /** Rail footer: no chevron, and the text hides when the rail collapses. */
  compact?: boolean;
}

const ModelStatusButton: React.FC<ModelStatusButtonProps> = ({
  status,
  displayText,
  isDropdownOpen,
  onClick,
  className = "",
  compact = false,
}) => {
  const getStatusColor = (status: ModelStatus): string => {
    switch (status) {
      case "ready":
        return "bg-ok";
      case "loading":
        return "bg-warn animate-pulse";
      case "downloading":
        return "bg-text-2 animate-pulse";
      case "verifying":
        return "bg-warn animate-pulse";
      case "extracting":
        return "bg-warn animate-pulse";
      case "error":
        return "bg-danger";
      case "unloaded":
        return "bg-text-3";
      case "none":
        return "bg-danger";
      default:
        return "bg-text-3";
    }
  };

  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex items-center gap-2 hover:text-text/80 transition-colors ${className}`}
      title={`Model status: ${displayText}`}
    >
      <div
        className={`w-2 h-2 shrink-0 rounded-full ${getStatusColor(status)}`}
      />
      <span
        className={
          compact ? "min-w-0 truncate max-[840px]:sr-only" : "max-w-28 truncate"
        }
      >
        {displayText}
      </span>
      {!compact && (
        <svg
          className={`w-3 h-3 transition-transform ${isDropdownOpen ? "rotate-180" : ""}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M19 9l-7 7-7-7"
          />
        </svg>
      )}
    </button>
  );
};

export default ModelStatusButton;
