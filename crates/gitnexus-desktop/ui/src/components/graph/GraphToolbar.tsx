import { Maximize2 } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import type { GraphStats, ZoomLevel } from "../../lib/tauri-commands";

const ZOOM_LEVELS: { id: ZoomLevel; label: string }[] = [
  { id: "package", label: "Packages" },
  { id: "module", label: "Modules" },
  { id: "symbol", label: "Symbols" },
];

const LAYOUTS = [
  { id: "cose", label: "Force" },
  { id: "grid", label: "Grid" },
  { id: "circle", label: "Circle" },
  { id: "breadthfirst", label: "Tree" },
];

export function GraphToolbar({
  stats,
  layout,
  onLayoutChange,
  onFit,
}: {
  stats?: GraphStats;
  layout: string;
  onLayoutChange: (layout: string) => void;
  onFit: () => void;
}) {
  const zoomLevel = useAppStore((s) => s.zoomLevel);
  const setZoomLevel = useAppStore((s) => s.setZoomLevel);

  return (
    <div className="flex items-center gap-3 px-3 py-2 border-b border-[var(--border)] bg-[var(--bg-secondary)] flex-wrap">
      {/* Zoom level */}
      <div className="flex rounded-lg border border-[var(--border)] overflow-hidden">
        {ZOOM_LEVELS.map(({ id, label }) => (
          <button
            key={id}
            onClick={() => setZoomLevel(id)}
            className={`px-3 py-1 text-xs transition-colors ${
              zoomLevel === id
                ? "bg-[var(--accent)] text-white"
                : "text-[var(--text-muted)] hover:text-[var(--text-secondary)] hover:bg-[var(--bg-tertiary)]"
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      {/* Layout selector */}
      <select
        value={layout}
        onChange={(e) => onLayoutChange(e.target.value)}
        className="px-2 py-1 rounded border border-[var(--border)] bg-[var(--bg-primary)] text-[var(--text-secondary)] text-xs focus:outline-none focus:border-[var(--accent)]"
      >
        {LAYOUTS.map(({ id, label }) => (
          <option key={id} value={id}>
            {label}
          </option>
        ))}
      </select>

      {/* Fit button */}
      <button
        onClick={onFit}
        title="Fit to screen"
        className="p-1.5 rounded hover:bg-[var(--bg-tertiary)] text-[var(--text-muted)] hover:text-[var(--text-secondary)] transition-colors"
      >
        <Maximize2 size={14} />
      </button>

      {/* Stats */}
      {stats && (
        <div className="flex gap-3 text-xs text-[var(--text-muted)] ml-auto">
          <span>{stats.nodeCount} nodes</span>
          <span>{stats.edgeCount} edges</span>
          {stats.truncated && (
            <span className="text-[var(--warning)]">truncated</span>
          )}
        </div>
      )}
    </div>
  );
}
