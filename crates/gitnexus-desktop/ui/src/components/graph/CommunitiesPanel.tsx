/**
 * CommunitiesPanel — Right sidebar listing functional groups with colors.
 * Inspired by sigmajs.org demo ClustersPanel.
 * Click a community → isolate its nodes in the graph.
 * Ctrl+Click → toggle additive multi-select.
 */

import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { Layers, Eye, EyeOff } from "lucide-react";
import { commands } from "../../lib/tauri-commands";
import { getCommunityColor } from "../../lib/graph-adapter";
import { useAppStore } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";

export function CommunitiesPanel() {
  const { t } = useI18n();
  const selectedFeatures = useAppStore((s) => s.selectedFeatures);
  const toggleFeature = useAppStore((s) => s.toggleFeature);
  const resetFeatures = useAppStore((s) => s.resetFeatures);

  const { data: features } = useQuery({
    queryKey: ["features"],
    queryFn: () => commands.getFeatures(),
    staleTime: 60_000,
  });

  const sorted = useMemo(() => {
    if (!features) return [];
    return [...features].sort((a, b) => b.memberCount - a.memberCount);
  }, [features]);

  const maxCount = sorted.length > 0 ? sorted[0].memberCount : 1;
  const hasFilter = selectedFeatures.size > 0;

  const handleClick = (name: string, e: React.MouseEvent) => {
    if (e.ctrlKey || e.metaKey) {
      // Additive toggle
      toggleFeature(name);
    } else {
      // Exclusive: if already the only selected, reset; otherwise isolate
      if (selectedFeatures.size === 1 && selectedFeatures.has(name)) {
        resetFeatures();
      } else {
        resetFeatures();
        toggleFeature(name);
      }
    }
  };

  if (!sorted.length) return null;

  return (
    <div
      className="flex flex-col h-full border-l"
      style={{
        width: 260,
        background: "var(--bg-1)",
        borderColor: "var(--surface-border)",
        flexShrink: 0,
      }}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between px-3 py-2.5 border-b"
        style={{ borderColor: "var(--surface-border)" }}
      >
        <div className="flex items-center gap-1.5">
          <Layers size={13} style={{ color: "var(--accent)" }} />
          <span className="text-xs font-semibold" style={{ color: "var(--text-1)" }}>
            {t("communities.title")}
          </span>
          <span className="text-[10px]" style={{ color: "var(--text-4)" }}>
            ({sorted.length})
          </span>
        </div>
        {hasFilter && (
          <button
            onClick={() => resetFeatures()}
            className="text-[10px] font-medium px-2 py-0.5 rounded-full transition-colors"
            style={{
              background: "var(--accent)",
              color: "white",
              border: "none",
              cursor: "pointer",
            }}
          >
            {t("communities.showAll")}
          </button>
        )}
      </div>

      {/* List */}
      <div className="flex-1 overflow-y-auto py-1">
        {sorted.map((feat) => {
          const color = getCommunityColor(feat.name);
          const isSelected = selectedFeatures.has(feat.name);
          const barWidth = Math.max(4, (feat.memberCount / maxCount) * 100);

          return (
            <button
              key={feat.id}
              onClick={(e) => handleClick(feat.name, e)}
              className="w-full flex items-center gap-2 px-3 py-1.5 text-left transition-all group"
              aria-pressed={isSelected}
              style={{
                background: isSelected ? `${color}15` : "transparent",
                borderLeft: isSelected ? `3px solid ${color}` : "3px solid transparent",
                cursor: "pointer",
                border: "none",
                borderBottom: "none",
              }}
              title={feat.description || feat.name}
            >
              {/* Color dot */}
              <span
                className="shrink-0 rounded-full"
                style={{
                  width: 8,
                  height: 8,
                  background: color,
                  boxShadow: isSelected ? `0 0 6px ${color}` : "none",
                }}
              />

              {/* Name + count */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center justify-between gap-1">
                  <span
                    className="text-[11px] font-medium truncate"
                    style={{
                      color: isSelected ? "var(--text-0)" : "var(--text-2)",
                      maxWidth: 140,
                    }}
                  >
                    {feat.name}
                  </span>
                  <span
                    className="text-[9px] shrink-0"
                    style={{ color: "var(--text-4)" }}
                  >
                    {feat.memberCount}
                  </span>
                </div>

                {/* Progress bar */}
                <div
                  className="mt-0.5 rounded-full overflow-hidden"
                  style={{
                    height: 3,
                    background: "var(--bg-3)",
                    width: "100%",
                  }}
                >
                  <div
                    className="rounded-full transition-all"
                    style={{
                      width: `${barWidth}%`,
                      height: "100%",
                      background: isSelected ? color : `${color}60`,
                    }}
                  />
                </div>
              </div>

              {/* Visibility icon */}
              {hasFilter && (
                <span style={{ color: isSelected ? color : "var(--text-4)", flexShrink: 0 }}>
                  {isSelected ? <Eye size={11} /> : <EyeOff size={11} />}
                </span>
              )}
            </button>
          );
        })}
      </div>

      {/* Footer hint */}
      <div
        className="px-3 py-2 border-t text-center"
        style={{
          borderColor: "var(--surface-border)",
          color: "var(--text-4)",
          fontSize: 9,
        }}
      >
        {t("communities.hint")}
      </div>
    </div>
  );
}
