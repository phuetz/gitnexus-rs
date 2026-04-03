import { GraphToolbar } from "./GraphToolbar";
import { LensSelector } from "../explorer/LensSelector";
import { EgoDepthSlider } from "../explorer/EgoDepthSlider";
import { ViewModeToggle, type ViewMode } from "./ViewModeToggle";
import type { GraphStats } from "../../lib/tauri-commands";

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
    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
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
      <div className="flex items-center gap-3" style={{ paddingRight: 8 }}>
        <LensSelector />
        <EgoDepthSlider />
      </div>
      <div style={{ paddingRight: 12 }}>
        <ViewModeToggle mode={viewMode} onChange={onViewModeChange} />
      </div>
    </div>
  );
}
