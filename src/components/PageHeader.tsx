import React from "react";
import { ChevronRight } from "lucide-react";

interface PageHeaderProps {
  title: string;
  description?: string;
  /** Shown instead of the title on a page one level down. */
  breadcrumb?: { parent: string; onParent: () => void; current: string };
  actions?: React.ReactNode;
}

/** Page title (28/36 semibold) with optional breadcrumb and actions. */
export const PageHeader: React.FC<PageHeaderProps> = ({
  title,
  description,
  breadcrumb,
  actions,
}) => (
  <header className="flex items-start justify-between gap-4">
    <div className="min-w-0">
      {breadcrumb ? (
        <h1 className="flex items-center gap-2 font-display text-title font-semibold text-text">
          <button
            type="button"
            onClick={breadcrumb.onParent}
            className="text-text-2 hover:text-text rounded-lg cursor-pointer transition-colors focus-visible:focus-ring"
          >
            {breadcrumb.parent}
          </button>
          <ChevronRight
            size={20}
            strokeWidth={1.5}
            className="shrink-0 text-text-2 rtl:rotate-180"
            aria-hidden="true"
          />
          <span className="truncate">{breadcrumb.current}</span>
        </h1>
      ) : (
        <h1 className="font-display text-title font-semibold text-text">
          {title}
        </h1>
      )}
      {description && (
        <p className="mt-1 text-body text-text-2">{description}</p>
      )}
    </div>
    {actions && <div className="flex items-center gap-2">{actions}</div>}
  </header>
);
