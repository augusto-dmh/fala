import React from "react";

// Placeholder wordmark. Replace when Fala has its own brand.
const WORDMARK = "Fala";

const FalaTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  return (
    <svg
      width={width}
      height={height}
      className={className}
      viewBox="0 0 930 328"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <text
        x="465"
        y="250"
        textAnchor="middle"
        fontSize="260"
        fontWeight="700"
        fontFamily="system-ui, sans-serif"
        className="logo-primary"
      >
        {WORDMARK}
      </text>
    </svg>
  );
};

export default FalaTextLogo;
