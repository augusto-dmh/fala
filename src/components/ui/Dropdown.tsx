import React, { useContext, useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingTitleContext } from "./SettingContainer";

export interface DropdownOption {
  value: string;
  label: string;
  description?: string;
  disabled?: boolean;
}

interface DropdownProps {
  options: DropdownOption[];
  className?: string;
  menuClassName?: string;
  selectedValue: string | null;
  onSelect: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  onOpen?: () => void;
  /** Each new value opens the list, as a click would (another window asked for the picker). */
  openSignal?: number;
}

/**
 * The option a key moves to from `from` (an index into `options`, -1 for none), skipping
 * disabled ones: ArrowDown/ArrowUp step and wrap, Home/End jump to the ends. `null` for any other
 * key or when nothing is enabled.
 */
export function stepOption(
  options: readonly Pick<DropdownOption, "disabled">[],
  from: number,
  key: string,
): number | null {
  const enabled = options.flatMap((option, i) => (option.disabled ? [] : [i]));
  if (enabled.length === 0) return null;
  if (key === "Home") return enabled[0];
  if (key === "End") return enabled[enabled.length - 1];
  const at = enabled.indexOf(from);
  if (key === "ArrowDown")
    return at < 0 ? enabled[0] : enabled[(at + 1) % enabled.length];
  if (key === "ArrowUp")
    return at < 0
      ? enabled[enabled.length - 1]
      : enabled[(at - 1 + enabled.length) % enabled.length];
  return null;
}

export const Dropdown: React.FC<DropdownProps> = ({
  options,
  selectedValue,
  onSelect,
  className = "",
  menuClassName,
  placeholder,
  disabled = false,
  onOpen,
  openSignal,
}) => {
  const { t } = useTranslation();
  const [isOpen, setIsOpen] = useState(false);
  useEffect(() => {
    if (!openSignal || disabled) return;
    onOpen?.();
    setIsOpen(true);
    // Only a new signal opens: a refreshed `onOpen` or a re-enabled control does not.
  }, [openSignal]);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const optionRefs = useRef<(HTMLButtonElement | null)[]>([]);
  // The option to focus while the list is open (a keyboard open), by value: an `onOpen` that
  // refreshes the options (devices) keeps it on the same option. `null` leaves focus alone.
  const [focusValue, setFocusValue] = useState<string | null>(null);
  const listId = useId();
  const valueId = useId();
  // Inside a settings row, the row's title names the button and the list.
  const titleId = useContext(SettingTitleContext);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
        setFocusValue(null);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const selectedIndex = options.findIndex(
    (option) => option.value === selectedValue,
  );
  const selectedOption = options[selectedIndex];

  const focusOption = (value: string | null) => {
    if (value === null) return;
    optionRefs.current[options.findIndex((o) => o.value === value)]?.focus();
  };

  useEffect(() => {
    if (isOpen) focusOption(focusValue);
    // `options` too: a refresh on open re-renders the buttons.
  }, [isOpen, focusValue, options]);

  /** The option a keyboard open lands on: the current one, else the first (or last) enabled. */
  const startValue = (key: string): string | null => {
    if (selectedOption && !selectedOption.disabled) return selectedOption.value;
    const first = stepOption(options, -1, key === "ArrowUp" ? "End" : "Home");
    return first === null ? null : options[first].value;
  };

  const open = (focus: string | null) => {
    onOpen?.();
    setFocusValue(focus);
    setIsOpen(true);
  };

  // Closing from the keyboard (Esc, a choice) puts focus back on the button.
  const close = (refocus: boolean) => {
    setIsOpen(false);
    setFocusValue(null);
    if (refocus) triggerRef.current?.focus();
  };

  const handleSelect = (value: string) => {
    onSelect(value);
    close(true);
  };

  // Enter or Space (a click with `detail` 0) opens onto the current option, like the arrows.
  const handleToggle = (event: React.MouseEvent) => {
    if (disabled) return;
    if (isOpen) close(false);
    else open(event.detail === 0 ? startValue("ArrowDown") : null);
  };

  const handleTriggerKey = (event: React.KeyboardEvent) => {
    if (disabled) return;
    if (event.key === "Escape" && isOpen) {
      event.preventDefault();
      event.stopPropagation();
      close(true);
      return;
    }
    if (event.key === "Tab" && isOpen) {
      close(false);
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const start = startValue(event.key);
      if (!isOpen) {
        open(start);
        return;
      }
      setFocusValue(start);
      focusOption(start);
    }
  };

  const handleListKey = (event: React.KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close(true);
      return;
    }
    if (event.key === "Tab") {
      close(false);
      return;
    }
    const current = optionRefs.current.findIndex(
      (option) => option === document.activeElement,
    );
    const next = stepOption(options, current, event.key);
    if (next !== null) {
      event.preventDefault();
      setFocusValue(options[next].value);
      optionRefs.current[next]?.focus();
    }
  };

  return (
    <div className={`relative ${className}`} ref={dropdownRef}>
      <button
        ref={triggerRef}
        type="button"
        aria-haspopup="listbox"
        aria-expanded={isOpen && !disabled}
        aria-controls={isOpen && !disabled ? listId : undefined}
        aria-labelledby={titleId ? `${titleId} ${valueId}` : undefined}
        onKeyDown={handleTriggerKey}
        className={`min-h-[32px] px-3 py-[5px] text-body text-text bg-surface-2 border border-border rounded-md min-w-[200px] w-full text-start grid grid-cols-[1fr_auto] gap-2 items-center transition-colors focus-visible:focus-ring ${
          disabled
            ? "opacity-50 cursor-not-allowed"
            : "hover:bg-text/5 active:bg-text/10 cursor-pointer"
        }`}
        onClick={handleToggle}
        disabled={disabled}
      >
        <span id={valueId} className="truncate">
          {selectedOption?.label || placeholder || t("common.selectOption")}
        </span>
        <svg
          aria-hidden="true"
          className={`w-4 h-4 text-text-2 transition-transform ${isOpen ? "transform rotate-180" : ""}`}
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
      </button>
      {isOpen && !disabled && (
        <div
          id={listId}
          role="listbox"
          aria-labelledby={titleId ?? undefined}
          onKeyDown={handleListKey}
          className={`absolute top-full mt-1 p-1 bg-surface-1 border border-border rounded-xl shadow-lg z-50 max-h-60 overflow-y-auto ${
            menuClassName ?? "left-0 right-0"
          }`}
        >
          {options.length === 0 ? (
            <div className="px-3 py-1.5 text-body text-text-2">
              {t("common.noOptionsFound")}
            </div>
          ) : (
            options.map((option, index) => (
              <button
                key={option.value}
                ref={(element) => {
                  optionRefs.current[index] = element;
                }}
                type="button"
                role="option"
                aria-selected={index === selectedIndex}
                tabIndex={-1}
                className={`relative w-full text-body text-text text-start rounded-md hover:bg-text/5 focus-visible:bg-text/5 focus-visible:outline-none transition-colors ${
                  option.description ? "px-3 py-2" : "px-3 py-1.5"
                } ${
                  selectedValue === option.value
                    ? "bg-text/5 before:absolute before:start-0 before:top-1/2 before:-translate-y-1/2 before:h-4 before:w-[3px] before:rounded-full before:bg-accent"
                    : ""
                } ${option.disabled ? "opacity-50 cursor-not-allowed" : ""}`}
                onClick={() => handleSelect(option.value)}
                disabled={option.disabled}
              >
                <span
                  className={`block whitespace-normal break-words ${
                    option.description ? "font-semibold" : ""
                  }`}
                >
                  {option.label}
                </span>
                {option.description && (
                  <span className="mt-0.5 block whitespace-normal text-caption font-normal text-text-2">
                    {option.description}
                  </span>
                )}
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
};
