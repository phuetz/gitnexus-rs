import { Maximize2, ChevronDown, GitBranch, Download, HelpCircle, Skull } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";
import { Tooltip } from "../shared/Tooltip";
import { AnimatedCounter } from "../shared/motion";
import type { GraphStats, ZoomLevel } from "../../lib/tauri-commands";
import { useState } from "react";
import { toast } from "sonner";

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
  complexityThreshold,
  onComplexityChange,
  hotspotDays,
  onHotspotDaysChange,
  showDeadCode,
  onToggleDeadCode,
}: {
  stats?: GraphStats;
  layout: string;
  onLayoutChange: (layout: string) => void;
  onFit: () => void;
  onFlows?: () => void;
  onExport?: () => void;
  hiddenEdgeTypes?: Set<string>;
  onToggleEdgeType?: (type: string) => void;
  complexityThreshold?: number;
  onComplexityChange?: (v: number) => void;
  hotspotDays?: number;
  onHotspotDaysChange?: (v: number) => void;
  showDeadCode?: boolean;
  onToggleDeadCode?: () => void;
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

      {/* Help Button */}
      <Tooltip content={t("graph.help") || "Aide"}>
        <button
          onClick={() => {
            toast.info(t("graph.helpTitle") || "Comment lire ce graphe ?", {
              description: t("graph.helpDescription") || "Les gros nœuds sont des fichiers/classes. Les petits sont des méthodes. Cliquez sur un nœud pour voir ses voisins, ou sur la légende pour filtrer par type.",
              duration: 10000,
            });
          }}
          aria-label={t("graph.help") || "Aide"}
          className="p-2 rounded-md transition-all hover:bg-[var(--bg-3)] hover:text-[var(--text-0)]"
          style={{
            color: "var(--text-3)",
            cursor: "pointer",
          }}
        >
          <HelpCircle size={16} />
        </button>
      </Tooltip>

      {/* Dead Code Toggle */}
      {onToggleDeadCode !== undefined && (
        <Tooltip content={t("graph.deadCode") || "Code Mort"}>
          <button
            onClick={onToggleDeadCode}
            aria-label={t("graph.deadCode") || "Code Mort"}
            className="p-2 rounded-md transition-all hover:bg-[var(--bg-3)]"
            style={{
              background: showDeadCode ? "rgba(247, 118, 142, 0.2)" : "transparent",
              color: showDeadCode ? "#f7768e" : "var(--text-3)",
              cursor: "pointer",
              border: showDeadCode ? "1px solid #f7768e" : "1px solid transparent",
            }}
          >
            <Skull size={16} />
          </button>
        </Tooltip>
      )}

      {/* Complexity Filter */}
      {onComplexityChange !== undefined && (
        <div className="flex items-center gap-2 px-3 py-1 rounded-md border border-[var(--surface-border)] bg-[var(--surface)]">
          <span className="text-[10px] font-medium text-[var(--text-3)]">Cplx &gt; {complexityThreshold}</span>
          <input
            type="range"
            min="0"
            max="100"
            value={complexityThreshold}
            onChange={(e) => onComplexityChange(parseInt(e.target.value))}
            className="w-20 h-1 bg-[var(--bg-3)] rounded-lg appearance-none cursor-pointer accent-[var(--accent)]"
          />
        </div>
      )}

      {/* Hotspot Time Range Filter */}
      {onHotspotDaysChange !== undefined && (
        <div className="flex items-center gap-1 px-2 py-1 rounded-md border border-[var(--surface-border)] bg-[var(--surface)]">
          <span className="text-[10px] font-medium text-[var(--text-3)] mr-1">Git:</span>
          {[30, 90, 365].map(d => (
            <button
              key={d}
              onClick={() => onHotspotDaysChange(d)}
              className="px-1.5 py-0.5 rounded text-[9px] font-bold transition-all"
              style={{
                background: hotspotDays === d ? "var(--accent)" : "transparent",
                color: hotspotDays === d ? "white" : "var(--text-3)",
                border: hotspotDays === d ? "none" : "1px solid var(--surface-border)",
                cursor: "pointer"
              }}
            >
              {d}d
            </button>
          ))}
        </div>
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
    </div>
  );
}
