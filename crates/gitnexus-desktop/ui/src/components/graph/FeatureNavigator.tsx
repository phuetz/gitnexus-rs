import { memo, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { Layers, Check, RotateCcw } from "lucide-react";
import { commands } from "../../lib/tauri-commands";
import { COMMUNITY_COLORS } from "../../lib/graph-adapter";


function hashString(s: string): number {
  let hash = 0;
  for (let i = 0; i < s.length; i++) {
    hash = ((hash << 5) - hash + s.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

interface FeatureNavigatorProps {
  selectedFeatures: Set<string>;
  onToggleFeature: (name: string) => void;
  onReset: () => void;
}

export const FeatureNavigator = memo(function FeatureNavigator({ selectedFeatures, onToggleFeature, onReset }: FeatureNavigatorProps) {
  const { data: features } = useQuery({
    queryKey: ["features"],
    queryFn: () => commands.getFeatures(),
    staleTime: 60_000,
  });

  // Pre-compute feature→color map once when features change
  const colorMap = useMemo(() => {
    if (!features) return new Map<string, string>();
    const map = new Map<string, string>();
    for (const f of features) {
      map.set(f.name, COMMUNITY_COLORS[hashString(f.name) % COMMUNITY_COLORS.length]);
    }
    return map;
  }, [features]);

  if (!features || features.length === 0) {
    return (
      <div
        className="flex flex-col items-center justify-center h-full p-4 text-center"
        style={{ width: 220, background: "var(--bg-1)", borderRight: "1px solid var(--surface-border)" }}
      >
        <Layers size={20} style={{ color: "var(--text-4)", marginBottom: 8 }} />
        <p className="text-[11px]" style={{ color: "var(--text-3)" }}>No features detected</p>
      </div>
    );
  }

  return (
    <div
      className="flex flex-col h-full"
      style={{
        width: 220,
        background: "var(--bg-1)",
        borderRight: "1px solid var(--surface-border)",
      }}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between px-3 py-2 shrink-0"
        style={{ borderBottom: "1px solid var(--surface-border)" }}
      >
        <div className="flex items-center gap-1.5">
          <Layers size={13} style={{ color: "var(--accent)" }} />
          <span className="text-xs font-semibold" style={{ color: "var(--text-0)" }}>
            Features
          </span>
          {selectedFeatures.size > 0 && (
            <span className="text-[9px] px-1.5 py-0.5 rounded-full font-medium"
              style={{ background: "var(--accent)", color: "white" }}>
              {selectedFeatures.size}
            </span>
          )}
        </div>
        {selectedFeatures.size > 0 && (
          <button
            onClick={onReset}
            className="p-1 rounded transition-colors"
            style={{ color: "var(--text-3)" }}
            title="Show all"
          >
            <RotateCcw size={12} />
          </button>
        )}
      </div>

      {/* Feature list */}
      <div className="flex-1 overflow-y-auto py-1">
        {features.map((feature) => {
          const isSelected = selectedFeatures.has(feature.name);
          const color = colorMap.get(feature.name) || "#565f89";

          return (
            <button
              key={feature.id}
              onClick={() => onToggleFeature(feature.name)}
              className="w-full flex items-center gap-2 px-3 py-1.5 text-left transition-colors"
              style={{
                background: isSelected ? `${color}15` : "transparent",
                borderLeft: isSelected ? `3px solid ${color}` : "3px solid transparent",
              }}
              onMouseEnter={(e) => {
                if (!isSelected) e.currentTarget.style.background = "var(--bg-2)";
              }}
              onMouseLeave={(e) => {
                if (!isSelected) e.currentTarget.style.background = "transparent";
              }}
            >
              {/* Color dot */}
              <span
                className="w-2 h-2 rounded-full shrink-0"
                style={{ background: color }}
              />

              {/* Name + count */}
              <div className="flex-1 min-w-0">
                <div className="text-[11px] font-medium truncate"
                  style={{ color: isSelected ? "var(--text-0)" : "var(--text-2)" }}>
                  {feature.name}
                </div>
                {feature.description && (
                  <div className="text-[9px] truncate" style={{ color: "var(--text-4)" }}>
                    {feature.description}
                  </div>
                )}
              </div>

              {/* Member count */}
              <span className="text-[9px] shrink-0" style={{ color: "var(--text-3)" }}>
                {feature.memberCount}
              </span>

              {/* Check */}
              {isSelected && <Check size={12} className="shrink-0" style={{ color }} />}
            </button>
          );
        })}
      </div>
    </div>
  );
});
