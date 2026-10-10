import React from "react";

interface SettingsGroupProps {
  title?: string;
  description?: string;
  children: React.ReactNode;
}

/** A group of settings rows: a sentence-case subtitle, then one card per row,
 *  4 px apart, as in the Windows 11 Settings app. */
export const SettingsGroup: React.FC<SettingsGroupProps> = ({
  title,
  description,
  children,
}) => {
  return (
    <section className="space-y-2">
      {title && (
        <div className="px-1">
          <h2 className="text-body font-semibold text-text">{title}</h2>
          {description && (
            <p className="text-caption text-text-2 mt-0.5">{description}</p>
          )}
        </div>
      )}
      <div className="flex flex-col gap-1 [&>*]:bg-surface-1 [&>*]:border [&>*]:border-border [&>*]:rounded-lg">
        {children}
      </div>
    </section>
  );
};
