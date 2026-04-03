import { useMemo } from "react";
import { NODE_COLORS } from "../../lib/graph-adapter";
import { useI18n } from "../../hooks/use-i18n";
import type { CytoNode } from "../../lib/tauri-commands";

const LABEL_COLORS = NODE_COLORS;

interface GraphLegendProps {
  nodes: CytoNode[];
  expanded: boolean;
  onExpand: () => void;
  onCollapse: () => void;
}

export function GraphLegend({ nodes, expanded, onExpand, onCollapse }: GraphLegendProps) {
  const { t } = useI18n();

  const sortedEntries = useMemo(() => {
    const counts = new Map<string, number>();
    for (const node of nodes) {
      counts.set(node.label, (counts.get(node.label) || 0) + 1);
    }
    return Array.from(counts.entries()).sort(
      (a, b) => b[1] - a[1] || a[0].localeCompare(b[0]),
    );
  }, [nodes]);

  return (
    <div
      className="absolute z-15 pointer-events-auto"
      style={{
        bottom: "12px",
        right: "12px",
        borderRadius: "var(--radius-md)",
        backgroundColor: "var(--bg-2)",
        backdropFilter: "blur(12px)",
        border: "1px solid var(--surface-border)",
        padding: "8px 12px",
      }}
    >
      {!expanded ? (
        <button
          onClick={onExpand}
          className="uppercase text-[10px] font-semibold transition-colors"
          style={{ color: "var(--text-3)" }}
          onMouseEnter={(e) => {
            e.currentTarget.style.color = "var(--text-2)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.color = "var(--text-3)";
          }}
        >
          {t("graph.legend")}
        </button>
      ) : (
        <div>
          <div className="flex items-center justify-between mb-2">
            <span
              className="uppercase text-[10px] font-semibold"
              style={{ color: "var(--text-3)" }}
            >
              {t("graph.legend")}
            </span>
            <button
              onClick={onCollapse}
              className="ml-2 text-xs transition-colors"
              style={{ color: "var(--text-3)" }}
              onMouseEnter={(e) => {
                e.currentTarget.style.color = "var(--text-2)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.color = "var(--text-3)";
              }}
            >
              x
            </button>
          </div>
          <div
            className="space-y-1 max-h-[calc(8*28px)] overflow-y-auto"
            style={{ maxWidth: "180px" }}
          >
            {sortedEntries.map(([type, count]) => (
              <div
                key={type}
                className="flex items-center gap-2"
                style={{ padding: "4px 0" }}
              >
                <span
                  className="w-2.5 h-2.5 rounded-full flex-shrink-0"
                  style={{
                    backgroundColor: LABEL_COLORS[type] || "#565f89",
                  }}
                />
                <span
                  className="text-[11px] truncate"
                  style={{ color: "var(--text-1)" }}
                >
                  {type}
                </span>
                <span
                  className="text-[10px] ml-auto flex-shrink-0"
                  style={{ color: "var(--text-3)" }}
                >
                  {count}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
