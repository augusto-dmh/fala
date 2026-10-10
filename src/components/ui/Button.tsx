import React from "react";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?:
    | "primary"
    | "primary-soft"
    | "secondary"
    | "warning"
    | "danger"
    | "danger-ghost"
    | "ghost";
  size?: "sm" | "md" | "lg";
}

export const Button: React.FC<ButtonProps> = ({
  children,
  className = "",
  variant = "primary",
  size = "md",
  ...props
}) => {
  const baseClasses =
    "font-normal rounded-lg border transition-colors focus-visible:focus-ring disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer";

  const variantClasses = {
    // Ink fill: the only accent the app has.
    primary:
      "text-on-accent bg-accent border-accent hover:bg-accent/90 active:bg-accent/80",
    "primary-soft":
      "text-text bg-text/10 border-transparent hover:bg-text/15 active:bg-text/20",
    // Windows 11 standard button: a quiet fill with a hairline border.
    secondary:
      "text-text bg-surface-2 border-border hover:bg-text/5 active:bg-text/10",
    // Secondary's neutral resting look, with the semantic warn token on hover,
    // for buttons sitting on warning surfaces like SecureInputWarning.
    warning:
      "text-text bg-surface-2 border-border hover:bg-warn/15 hover:border-warn",
    danger:
      "text-on-accent bg-danger border-danger hover:bg-danger/90 active:bg-danger/80",
    "danger-ghost":
      "text-danger border-transparent hover:bg-danger/10 active:bg-danger/15",
    ghost: "text-current border-transparent hover:bg-text/5 active:bg-text/10",
  };

  const sizeClasses = {
    sm: "min-h-[24px] px-2 text-caption",
    md: "min-h-[32px] px-3 text-body",
    lg: "min-h-[40px] px-4 text-body",
  };

  return (
    <button
      className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};
