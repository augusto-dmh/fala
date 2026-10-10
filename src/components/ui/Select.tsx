import React from "react";
import SelectComponent from "react-select";
import CreatableSelect from "react-select/creatable";
import type {
  ActionMeta,
  Props as ReactSelectProps,
  SingleValue,
  StylesConfig,
} from "react-select";

export type SelectOption = {
  value: string;
  label: string;
  isDisabled?: boolean;
};

type BaseProps = {
  value: string | null;
  options: SelectOption[];
  placeholder?: string;
  disabled?: boolean;
  isLoading?: boolean;
  isClearable?: boolean;
  onChange: (value: string | null, action: ActionMeta<SelectOption>) => void;
  onBlur?: () => void;
  className?: string;
  formatCreateLabel?: (input: string) => string;
};

type CreatableProps = {
  isCreatable: true;
  onCreateOption: (value: string) => void;
};

type NonCreatableProps = {
  isCreatable?: false;
  onCreateOption?: never;
};

export type SelectProps = BaseProps & (CreatableProps | NonCreatableProps);

// Fluent combo box on the ink tokens: quiet fill, hairline border with a
// darker bottom edge, ink underline on focus, 8 px menu with a shadow.
const baseBackground = "var(--color-surface-1)";
const hoverBackground = "var(--color-surface-2)";
const selectedBackground =
  "color-mix(in srgb, var(--color-text) 8%, transparent)";
const focusedOptionBackground =
  "color-mix(in srgb, var(--color-text) 5%, transparent)";

export const selectStyles: StylesConfig<SelectOption, false> = {
  control: (base, state) => ({
    ...base,
    minHeight: 32,
    borderRadius: 4,
    borderColor: "var(--color-border)",
    borderBottomColor: state.isFocused
      ? "var(--color-accent)"
      : "var(--color-text-3)",
    boxShadow: state.isFocused ? "inset 0 -1px 0 var(--color-accent)" : "none",
    backgroundColor: baseBackground,
    fontSize: "14px",
    color: "var(--color-text)",
    transition: "background-color 100ms ease-out, border-color 100ms ease-out",
    ":hover": {
      borderColor: "var(--color-border)",
      borderBottomColor: state.isFocused
        ? "var(--color-accent)"
        : "var(--color-text-3)",
      backgroundColor: state.isFocused ? baseBackground : hoverBackground,
    },
  }),
  valueContainer: (base) => ({
    ...base,
    paddingInline: 10,
    paddingBlock: 2,
  }),
  input: (base) => ({
    ...base,
    color: "var(--color-text)",
  }),
  singleValue: (base) => ({
    ...base,
    color: "var(--color-text)",
  }),
  indicatorSeparator: (base) => ({
    ...base,
    backgroundColor: "var(--color-border)",
  }),
  dropdownIndicator: (base) => ({
    ...base,
    padding: 6,
    color: "var(--color-text-2)",
    ":hover": {
      color: "var(--color-text)",
    },
  }),
  clearIndicator: (base) => ({
    ...base,
    padding: 6,
    color: "var(--color-text-2)",
    ":hover": {
      color: "var(--color-text)",
    },
  }),
  menu: (provided) => ({
    ...provided,
    zIndex: 30,
    padding: 4,
    borderRadius: 8,
    backgroundColor: "var(--color-surface-1)",
    color: "var(--color-text)",
    border: "1px solid var(--color-border)",
    boxShadow: "0 8px 16px rgba(0, 0, 0, 0.14)",
  }),
  option: (base, state) => ({
    ...base,
    borderRadius: 4,
    fontSize: "14px",
    backgroundColor: state.isSelected
      ? selectedBackground
      : state.isFocused
        ? focusedOptionBackground
        : "transparent",
    color: "var(--color-text)",
    cursor: state.isDisabled ? "not-allowed" : base.cursor,
    opacity: state.isDisabled ? 0.5 : 1,
    ":active": {
      backgroundColor: selectedBackground,
    },
  }),
  placeholder: (base) => ({
    ...base,
    color: "var(--color-text-3)",
  }),
};

export const Select: React.FC<SelectProps> = React.memo(
  ({
    value,
    options,
    placeholder,
    disabled,
    isLoading,
    isClearable = true,
    onChange,
    onBlur,
    className = "",
    isCreatable,
    formatCreateLabel,
    onCreateOption,
  }) => {
    const selectValue = React.useMemo(() => {
      if (!value) return null;
      const existing = options.find((option) => option.value === value);
      if (existing) return existing;
      return { value, label: value, isDisabled: false };
    }, [value, options]);

    const handleChange = (
      option: SingleValue<SelectOption>,
      action: ActionMeta<SelectOption>,
    ) => {
      onChange(option?.value ?? null, action);
    };

    const sharedProps: Partial<ReactSelectProps<SelectOption, false>> = {
      className,
      classNamePrefix: "app-select",
      value: selectValue,
      options,
      onChange: handleChange,
      placeholder,
      isDisabled: disabled,
      isLoading,
      onBlur,
      isClearable,
      styles: selectStyles,
    };

    if (isCreatable) {
      return (
        <CreatableSelect<SelectOption, false>
          {...sharedProps}
          onCreateOption={onCreateOption}
          formatCreateLabel={formatCreateLabel}
        />
      );
    }

    return <SelectComponent<SelectOption, false> {...sharedProps} />;
  },
);

Select.displayName = "Select";
