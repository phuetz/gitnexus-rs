import { FileTreeView } from "../files/FileTreeView";
import { FeatureNavigator } from "../graph/FeatureNavigator";
import { useAppStore } from "../../stores/app-store";

export function ExplorerLeftPanel() {
  const selectedFeatures = useAppStore((s) => s.selectedFeatures);
  const toggleFeature = useAppStore((s) => s.toggleFeature);
  const resetFeatures = useAppStore((s) => s.resetFeatures);

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
          onToggleFeature={toggleFeature}
          onReset={resetFeatures}
        />
      </div>
    </div>
  );
}
