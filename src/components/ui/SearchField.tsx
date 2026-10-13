import React from "react";
import { Search } from "lucide-react";
import { Input } from "./Input";

type SearchFieldProps = Omit<
  React.ComponentProps<typeof Input>,
  "type" | "className"
> & {
  /** Classes for the wrapper, which takes the field's place in the layout. */
  className?: string;
};

/** A search box with the magnifier in front, the same on every page that filters a list. */
export const SearchField: React.FC<SearchFieldProps> = ({
  className = "",
  ...props
}) => (
  <div className={`relative ${className}`}>
    <Search
      aria-hidden="true"
      className={`pointer-events-none absolute start-2.5 top-1/2 size-4 -translate-y-1/2 text-text-3 ${
        props.disabled ? "opacity-50" : ""
      }`}
    />
    <Input type="search" className="w-full ps-8" {...props} />
  </div>
);
