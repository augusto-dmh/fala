import React from "react";

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  variant?: "default" | "compact";
}

export const Input: React.FC<InputProps> = ({
  className = "",
  variant = "default",
  disabled,
  ...props
}) => {
  // Fluent text box: quiet fill, hairline border with a darker bottom edge;
  // focus draws a 2 px ink underline instead of a ring.
  const baseClasses =
    "text-body text-text placeholder:text-text-3 bg-surface-1 border border-border border-b-text-3 rounded-md text-start transition-colors";

  const interactiveClasses = disabled
    ? "opacity-50 cursor-not-allowed"
    : "hover:bg-surface-2 focus:outline-none focus:bg-surface-1 focus:border-b-accent focus:shadow-[inset_0_-1px_0_var(--color-accent)]";

  const variantClasses = {
    default: "min-h-[32px] px-3 py-[5px]",
    compact: "min-h-[28px] px-2 py-[3px]",
  } as const;

  return (
    <input
      className={`${baseClasses} ${variantClasses[variant]} ${interactiveClasses} ${className}`}
      disabled={disabled}
      {...props}
    />
  );
};
