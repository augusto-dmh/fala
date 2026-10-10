import React from "react";
import { SettingContainer } from "./SettingContainer";
import { ResetButton } from "./ResetButton";

interface SliderProps {
  value: number;
  onChange: (value: number) => void;
  min: number;
  max: number;
  step?: number;
  disabled?: boolean;
  label: string;
  description: string;
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  showValue?: boolean;
  formatValue?: (value: number) => string;
  onReset?: () => void;
  isResetting?: boolean;
}

// Fluent slider: 4 px rail, ink fill, 20 px thumb with an ink centre.
const THUMB =
  "[&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[20px] [&::-webkit-slider-thumb]:h-[20px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-accent [&::-webkit-slider-thumb]:border-[5px] [&::-webkit-slider-thumb]:border-solid [&::-webkit-slider-thumb]:border-surface-1 [&::-webkit-slider-thumb]:shadow-[0_0_0_1px_var(--color-border)] [&::-webkit-slider-thumb]:transition-[border-width] hover:[&::-webkit-slider-thumb]:border-[4px]";

export const Slider: React.FC<SliderProps> = ({
  value,
  onChange,
  min,
  max,
  step = 0.01,
  disabled = false,
  label,
  description,
  descriptionMode = "tooltip",
  grouped = false,
  showValue = true,
  formatValue = (v) => v.toFixed(2),
  onReset,
  isResetting = false,
}) => {
  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    onChange(parseFloat(e.target.value));
  };
  const filled = ((value - min) / (max - min)) * 100;

  return (
    <SettingContainer
      title={label}
      description={description}
      descriptionMode={descriptionMode}
      grouped={grouped}
      layout="horizontal"
      disabled={disabled}
    >
      <div className="w-full">
        <div className="flex items-center gap-2 h-6">
          <input
            type="range"
            min={min}
            max={max}
            step={step}
            value={value}
            onChange={handleChange}
            disabled={disabled}
            className={`flex-grow h-[4px] rounded-full appearance-none cursor-pointer focus-visible:focus-ring disabled:opacity-50 disabled:cursor-not-allowed ${THUMB}`}
            style={{
              background: `linear-gradient(to right, var(--color-accent) ${filled}%, color-mix(in srgb, var(--color-text-2) 45%, transparent) ${filled}%)`,
            }}
          />
          {showValue && (
            <span className="text-body text-text-2 tabular-nums w-12 text-end">
              {formatValue(value)}
            </span>
          )}
          {onReset && (
            <ResetButton onClick={onReset} disabled={disabled || isResetting} />
          )}
        </div>
      </div>
    </SettingContainer>
  );
};
