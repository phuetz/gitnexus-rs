import { Tooltip } from "../shared/Tooltip";

interface GraphZoomControlsProps {
  onZoomIn: () => void;
  onZoomOut: () => void;
  onFitView: () => void;
  legendExpanded: boolean;
}

export function GraphZoomControls({
  onZoomIn,
  onZoomOut,
  onFitView,
  legendExpanded,
}: GraphZoomControlsProps) {
  const btnStyle = {
    background: "var(--bg-2)",
    border: "1px solid var(--surface-border)",
    color: "var(--text-2)",
    cursor: "pointer",
  } as const;

  return (
    <div
      className="absolute z-20 flex flex-col gap-1"
      style={{ bottom: legendExpanded ? 200 : 80, right: 16 }}
    >
      <Tooltip content="Zoom in (Ctrl+=)" side="left">
        <button
          onClick={onZoomIn}
          className="w-8 h-8 rounded-lg flex items-center justify-center text-sm font-bold"
          style={btnStyle}
          aria-label="Zoom in"
        >
          +
        </button>
      </Tooltip>
      <Tooltip content="Zoom out (Ctrl+-)" side="left">
        <button
          onClick={onZoomOut}
          className="w-8 h-8 rounded-lg flex items-center justify-center text-sm font-bold"
          style={btnStyle}
          aria-label="Zoom out"
        >
          {"\u2212"}
        </button>
      </Tooltip>
      <Tooltip content="Fit view (Ctrl+0)" side="left">
        <button
          onClick={onFitView}
          className="w-8 h-8 rounded-lg flex items-center justify-center text-[10px] font-bold"
          style={btnStyle}
          aria-label="Fit view"
        >
          {"\u229E"}
        </button>
      </Tooltip>
    </div>
  );
}
