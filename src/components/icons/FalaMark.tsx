// Placeholder mark: five voice bars, echoing the pill. Replace when Fala has its own brand.
const BARS = [0.35, 0.65, 1, 0.65, 0.35];

const FalaMark = ({
  width,
  height,
}: {
  width?: number | string;
  height?: number | string;
}) => (
  <svg
    width={width || 24}
    height={height || 24}
    viewBox="0 0 90 100"
    className="fill-text"
    xmlns="http://www.w3.org/2000/svg"
  >
    {BARS.map((f, i) => (
      <rect
        key={i}
        x={i * 20}
        y={50 - 50 * f}
        width={10}
        height={100 * f}
        rx={5}
      />
    ))}
  </svg>
);

export default FalaMark;
