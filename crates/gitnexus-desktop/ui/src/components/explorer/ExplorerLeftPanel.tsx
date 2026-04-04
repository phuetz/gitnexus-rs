import { memo, useState } from "react";
import { ChevronDown, ChevronRight, FolderTree, Layers } from "lucide-react";
import { FileTreeView } from "../files/FileTreeView";
import { FeatureNavigator } from "../graph/FeatureNavigator";
import { useAppStore } from "../../stores/app-store";

function SectionHeader({ icon: Icon, label, collapsed, onToggle }: {
  icon: typeof FolderTree;
  label: string;
  collapsed: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      onClick={onToggle}
      className="flex items-center gap-2 w-full shrink-0"
      style={{
        padding: "8px 12px",
        fontSize: 11,
        fontWeight: 600,
        color: "var(--text-3)",
        textTransform: "uppercase",
        letterSpacing: "0.04em",
        background: "transparent",
        border: "none",
        borderBottom: "1px solid var(--surface-border)",
        cursor: "pointer",
      }}
    >
      {collapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />}
      <Icon size={12} />
      {label}
    </button>
  );
}

export const ExplorerLeftPanel = memo(function ExplorerLeftPanel() {
  const selectedFeatures = useAppStore((s) => s.selectedFeatures);
  const toggleFeature = useAppStore((s) => s.toggleFeature);
  const resetFeatures = useAppStore((s) => s.resetFeatures);
  const [featuresCollapsed, setFeaturesCollapsed] = useState(true);

  return (
    <div
      className="flex flex-col h-full overflow-hidden"
      style={{
        background: "var(--glass-bg)",
        backdropFilter: "blur(var(--glass-blur))",
        borderRight: "1px solid var(--glass-border)",
      }}
    >
      {/* File tree section — always visible, takes priority */}
      <SectionHeader icon={FolderTree} label="Files" collapsed={false} onToggle={() => {}} />
      <div className="flex-1 min-h-0 overflow-auto">
        <FileTreeView />
      </div>

      {/* Feature navigator (communities) — collapsible */}
      <SectionHeader
        icon={Layers}
        label="Features"
        collapsed={featuresCollapsed}
        onToggle={() => setFeaturesCollapsed(!featuresCollapsed)}
      />
      {!featuresCollapsed && (
        <div
          className="shrink-0 overflow-auto"
          style={{ maxHeight: "35%" }}
        >
          <FeatureNavigator
            selectedFeatures={selectedFeatures}
            onToggleFeature={toggleFeature}
            onReset={resetFeatures}
          />
        </div>
      )}
    </div>
  );
});
