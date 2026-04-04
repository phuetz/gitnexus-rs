import { Maximize2, ChevronDown, GitBranch, Download } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";
import { Tooltip } from "../shared/Tooltip";
import { AnimatedCounter } from "../shared/motion";
import type { GraphStats, ZoomLevel } from "../../lib/tauri-commands";
import { useState } from "react";

const ZOOM_LEVELS: { id: ZoomLevel; i18nKey: string }[] = [
  { id: "package", i18nKey: "graph.packages" },
  { id: "module", i18nKey: "graph.modules" },
  { id: "symbol", i18nKey: "graph.symbols" },
];

const LAYOUTS = [
  { id: "forceatlas2", label: "Force" },
  { id: "grid", label: "Grid" },
  { id: "circle", label: "Circle" },
  { id: "random", label: "Random" },
];

export function GraphToolbar({
  stats,
  layout,
  onLayoutChange,
  onFit,
  onFlows,
  onExport,
  hiddenEdgeTypes,
  onToggleEdgeType,
  depthFilter,
  onDepthFilterChange,
}: {
  stats?: GraphStats;
  layout: string;
  onLayoutChange: (layout: string) => void;
  onFit: () => void;
  onFlows?: () => void;
  onExport?: () => void;
  hiddenEdgeTypes?: Set<string>;
  onToggleEdgeType?: (type: string) => void;
  depthFilter?: number | null;
  onDepthFilterChange?: (depth: number | null) => void;
}) {
  const { t, tt } = useI18n();
  const zoomLevel = useAppStore((s) => s.zoomLevel);
  const setZoomLevel = useAppStore((s) => s.setZoomLevel);
  const [showLayoutMenu, setShowLayoutMenu] = useState(false);

  const layoutLabel = LAYOUTS.find((l) => l.id === layout)?.label || "Layout";

  return (
    <div
      role="toolbar"
      aria-label={t("graph.edgeFilters")}
      className="flex items-center gap-4 px-4 py-2.5 border-b flex-wrap"
      style={{
        backgroundColor: "var(--bg-2)",
        borderBottomColor: "var(--surface-border)",
      }}
    >
      {/* Zoom Level Pill Toggle Group */}
      <div
        className="flex rounded-full p-1 gap-1"
        style={{
          backgroundColor: "var(--surface)",
          border: "1px solid",
          borderColor: "var(--surface-border)",
        }}
        title={t("graph.granularity")}
      >
        {ZOOM_LEVELS.map(({ id, i18nKey }) => {
          const { label, tip } = tt(i18nKey);
          return (
            <Tooltip key={id} content={tip}>
              <button
                onClick={() => setZoomLevel(id)}
                className="relative px-3 py-1.5 text-xs font-medium transition-all hover:bg-[var(--surface-hover)]"
                style={{
                  color: zoomLevel === id ? "var(--accent)" : "var(--text-3)",
                  backgroundColor: zoomLevel === id ? "var(--bg-2)" : "transparent",
                  borderRadius: "var(--radius-md)",
                  cursor: "pointer",
                  boxShadow: zoomLevel === id
                    ? "0 1px 3px rgba(0,0,0,0.2), inset 0 -2px 0 var(--accent)"
                    : "none",
                  fontWeight: zoomLevel === id ? 600 : 500,
                }}
              >
                {label}
              </button>
            </Tooltip>
          );
        })}
      </div>

      {/* Layout Dropdown Button */}
      <div className="relative">
        <Tooltip content={tt("graph.layout").tip}>
          <button
            onClick={() => setShowLayoutMenu(!showLayoutMenu)}
            aria-label={tt("graph.layout").label}
            className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium rounded-md transition-all hover:border-[var(--surface-border-hover)]"
            style={{
              backgroundColor: "var(--surface)",
              color: "var(--text-2)",
              border: "1px solid",
              borderColor: "var(--surface-border)",
              cursor: "pointer",
            }}
          >
            {layoutLabel}
            <ChevronDown
              size={14}
              style={{
                transform: showLayoutMenu ? "rotate(180deg)" : "rotate(0deg)",
                transition: "transform 0.2s",
              }}
            />
          </button>
        </Tooltip>

        {/* Dropdown Menu */}
        {showLayoutMenu && (
          <div
            className="absolute left-0 top-full mt-1 rounded-md overflow-hidden shadow-lg z-50"
            style={{
              backgroundColor: "var(--surface)",
              border: "1px solid",
              borderColor: "var(--surface-border)",
              minWidth: "120px",
            }}
            onMouseLeave={() => setShowLayoutMenu(false)}
          >
            {LAYOUTS.map(({ id, label }) => (
              <button
                key={id}
                onClick={() => {
                  onLayoutChange(id);
                  setShowLayoutMenu(false);
                }}
                className="w-full text-left px-3 py-2 text-xs transition-colors hover:bg-[var(--surface-hover)]"
                style={{
                  color: layout === id ? "var(--accent)" : "var(--text-2)",
                  backgroundColor:
                    layout === id ? "var(--bg-2)" : "transparent",
                  cursor: "pointer",
                }}
              >
                {label}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Fit Button */}
      <Tooltip content={tt("graph.fitView").tip}>
        <button
          onClick={onFit}
          aria-label={tt("graph.fitView").label}
          className="p-2 rounded-md transition-all hover:bg-[var(--surface-hover)] hover:text-[var(--text-2)]"
          style={{
            color: "var(--text-3)",
            cursor: "pointer",
          }}
        >
          <Maximize2 size={16} />
        </button>
      </Tooltip>

      {/* Export PNG Button */}
      {onExport && (
        <Tooltip content={t("graph.exportPng")}>
          <button
            onClick={onExport}
            aria-label={t("graph.exportPng")}
            className="p-2 rounded-md transition-all hover:bg-[var(--surface-hover)] hover:text-[var(--text-2)]"
            style={{
              color: "var(--text-3)",
              cursor: "pointer",
            }}
          >
            <Download size={16} />
          </button>
        </Tooltip>
      )}

      {/* Flows Button */}
      {onFlows && (
        <Tooltip content={t("graph.processFlows")}>
          <button
            onClick={onFlows}
            aria-label={t("graph.processFlows")}
            className="p-2 rounded-md transition-all hover:bg-[var(--bg-3)] hover:text-[var(--text-0)]"
            style={{
              color: "var(--text-3)",
              cursor: "pointer",
            }}
          >
            <GitBranch size={16} />
          </button>
        </Tooltip>
      )}

      {/* Stats Badges */}
      {stats && (
        <div className="flex gap-2 ml-auto">
          <div
            className="px-2.5 py-1 rounded-full text-xs font-medium"
            style={{
              backgroundColor: "var(--accent-subtle)",
              color: "var(--accent)",
            }}
          >
            <AnimatedCounter value={stats.nodeCount} /> {t("graph.nodesCount")}
          </div>
          <div
            className="px-2.5 py-1 rounded-full text-xs font-medium"
            style={{
              backgroundColor: "var(--accent-subtle)",
              color: "var(--accent)",
            }}
          >
            <AnimatedCounter value={stats.edgeCount} /> {t("graph.edgesCount")}
          </div>
          {stats.truncated && (
            <div
              className="px-2.5 py-1 rounded-full text-xs font-medium"
              style={{
                backgroundColor: "var(--amber)",
                color: "var(--bg-0)",
              }}
            >
              {t("graph.truncated")}
            </div>
          )}
        </div>
      )}

      {/* Edge Type Filters */}
      {hiddenEdgeTypes && onToggleEdgeType && (
        <div className="flex items-center gap-1 ml-2 pl-2" style={{ borderLeft: "1px solid var(--surface-border)" }}>
          <span className="text-[10px] font-medium" style={{ color: "var(--text-3)" }}>{t("graph.edges")}:</span>
          {["CALLS", "IMPORTS", "HAS_METHOD", "EXTENDS", "CALLS_ACTION", "CONTAINS"].map((type) => {
            const active = !hiddenEdgeTypes.has(type);
            return (
              <button
                key={type}
                onClick={() => onToggleEdgeType(type)}
                className="px-1.5 py-0.5 rounded text-[9px] font-medium transition-colors"
                style={{
                  background: active ? "var(--accent-subtle)" : "var(--bg-3)",
                  color: active ? "var(--accent)" : "var(--text-4)",
                  border: `1px solid ${active ? "var(--accent)" : "var(--surface-border)"}`,
                  cursor: "pointer",
                  opacity: active ? 1 : 0.5,
                }}
                aria-pressed={active}
                aria-label={`Toggle ${type.replace("_", " ")} edges`}
              >
                {type.replace("_", " ")}
              </button>
            );
          })}
        </div>
      )}

      {/* Depth Filter */}
      {onDepthFilterChange && (
        <div className="flex items-center gap-1 ml-2 pl-2" style={{ borderLeft: "1px solid var(--surface-border)" }}>
          <span className="text-[10px] font-medium" style={{ color: "var(--text-3)" }}>{t("graph.depth")}:</span>
          {([1, 2, 3, null] as const).map((d) => {
            const label = d === null ? t("graph.all") : String(d);
            const active = depthFilter === d;
            return (
              <button
                key={label}
                onClick={() => onDepthFilterChange(d)}
                className="px-1.5 py-0.5 rounded text-[9px] font-medium transition-colors"
                style={{
                  background: active ? "var(--accent)" : "var(--bg-3)",
                  color: active ? "white" : "var(--text-3)",
                  border: "none",
                  cursor: "pointer",
                }}
                aria-pressed={active}
              >
                {label}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
