import { useState, useCallback } from "react";
import { FileTreeView } from "../files/FileTreeView";
import { FeatureNavigator } from "../graph/FeatureNavigator";

export function ExplorerLeftPanel() {
  const [selectedFeatures, setSelectedFeatures] = useState<Set<string>>(new Set());

  const handleToggleFeature = useCallback((name: string) => {
    setSelectedFeatures((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }, []);

  const handleResetFeatures = useCallback(() => {
    setSelectedFeatures(new Set());
  }, []);

  return (
    <div
      className="flex flex-col h-full overflow-hidden"
      style={{
        background: "var(--glass-bg)",
        backdropFilter: "blur(var(--glass-blur))",
        borderRight: "1px solid var(--glass-border)",
      }}
    >
      {/* File tree — has its own internal search */}
      <div className="flex-1 min-h-0 overflow-auto">
        <FileTreeView />
      </div>

      {/* Feature navigator (communities) */}
      <div
        className="shrink-0"
        style={{
          borderTop: "1px solid var(--surface-border)",
          maxHeight: "30%",
          overflow: "auto",
        }}
      >
        <FeatureNavigator
          selectedFeatures={selectedFeatures}
          onToggleFeature={handleToggleFeature}
          onReset={handleResetFeatures}
        />
      </div>
    </div>
  );
}
