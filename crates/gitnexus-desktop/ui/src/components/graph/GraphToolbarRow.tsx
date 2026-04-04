import { GraphToolbar } from "./GraphToolbar";
import { LensSelector } from "../explorer/LensSelector";
import { EgoDepthSlider } from "../explorer/EgoDepthSlider";
import { ViewModeToggle, type ViewMode } from "./ViewModeToggle";
import type { GraphStats } from "../../lib/tauri-commands";

const SEP_STYLE = {
  width: 1,
  height: 20,
  background: "var(--surface-border)",
  flexShrink: 0,
} as const;

interface GraphToolbarRowProps {
  stats: GraphStats | undefined;
  layout: string;
  onLayoutChange: (layout: string) => void;
  onFit: () => void;
  onExport: () => void;
  onFlows: () => void;
  hiddenEdgeTypes: Set<string>;
  onToggleEdgeType: (type: string) => void;
  depthFilter: number | null;
  onDepthFilterChange: (depth: number | null) => void;
  viewMode: ViewMode;
  onViewModeChange: (mode: ViewMode) => void;
}

export function GraphToolbarRow({
  stats,
  layout,
  onLayoutChange,
  onFit,
  onExport,
  onFlows,
  hiddenEdgeTypes,
  onToggleEdgeType,
  depthFilter,
  onDepthFilterChange,
  viewMode,
  onViewModeChange,
}: GraphToolbarRowProps) {
  return (
    <div
      className="flex items-center"
      style={{ gap: 6, paddingRight: 8 }}
      role="toolbar"
      aria-label="Graph controls"
    >
      {/* Group 1: Navigation & Visualization */}
      <div style={{ flex: 1 }}>
        <GraphToolbar
          stats={stats}
          layout={layout}
          onLayoutChange={onLayoutChange}
          onFit={onFit}
          onExport={onExport}
          hiddenEdgeTypes={hiddenEdgeTypes}
          onToggleEdgeType={onToggleEdgeType}
          depthFilter={depthFilter}
          onDepthFilterChange={onDepthFilterChange}
          onFlows={onFlows}
        />
      </div>

      {/* Separator */}
      <div style={SEP_STYLE} />

      {/* Group 2: Lens & Depth */}
      <div className="flex items-center gap-2" style={{ paddingLeft: 4, paddingRight: 4 }}>
        <LensSelector />
        <EgoDepthSlider />
      </div>

      {/* Separator */}
      <div style={SEP_STYLE} />

      {/* Group 3: View Mode */}
      <div style={{ paddingLeft: 4 }}>
        <ViewModeToggle mode={viewMode} onChange={onViewModeChange} />
      </div>
    </div>
  );
}
