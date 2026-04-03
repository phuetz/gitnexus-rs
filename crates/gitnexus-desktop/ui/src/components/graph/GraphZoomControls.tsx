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
      <button
        onClick={onZoomIn}
        className="w-8 h-8 rounded-lg flex items-center justify-center text-sm font-bold"
        style={btnStyle}
        title="Zoom in (Ctrl+=)"
      >
        +
      </button>
      <button
        onClick={onZoomOut}
        className="w-8 h-8 rounded-lg flex items-center justify-center text-sm font-bold"
        style={btnStyle}
        title="Zoom out (Ctrl+-)"
      >
        {"\u2212"}
      </button>
      <button
        onClick={onFitView}
        className="w-8 h-8 rounded-lg flex items-center justify-center text-[10px] font-bold"
        style={btnStyle}
        title="Fit view (Ctrl+0)"
      >
        {"\u229E"}
      </button>
    </div>
  );
}
