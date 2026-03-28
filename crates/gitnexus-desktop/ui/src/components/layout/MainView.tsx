import { Group, Panel, Separator } from "react-resizable-panels";
import { useAppStore } from "../../stores/app-store";
import { AnimatedPage, AnimatePresence } from "../shared/motion";
import { RepoManager } from "../repos/RepoManager";
import { GraphExplorer } from "../graph/GraphExplorer";
import { FileTreeView } from "../files/FileTreeView";
import { FilePreview } from "../files/FilePreview";
import { ImpactView } from "../impact/ImpactView";
import { DocsViewer } from "../docs/DocsViewer";
import { ExportPanel } from "../export/ExportPanel";
import { RepoDashboard } from "../repos/RepoDashboard";

export function MainView() {
  const activeRepo = useAppStore((s) => s.activeRepo);
  const sidebarTab = useAppStore((s) => s.sidebarTab);
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const selectedNodeName = useAppStore((s) => s.selectedNodeName);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);

  if (!activeRepo) {
    return <RepoManager />;
  }

  const renderView = () => {
    switch (sidebarTab) {
      case "overview":
        return <RepoDashboard />;
      case "repos":
        return <RepoManager />;
      case "search":
        return <GraphExplorer />;
      case "files": {
        if (selectedNodeId && selectedNodeId.startsWith("File:")) {
          return (
            <Group orientation="horizontal" className="h-full">
              <Panel defaultSize={35} minSize={20}>
                <FileTreeView />
              </Panel>
              <Separator
                className="cursor-col-resize group relative"
                style={{ width: 5, background: "transparent" }}
              >
                <div
                  className="absolute inset-y-0 left-1/2 -translate-x-1/2"
                  style={{ width: 1, background: "var(--surface-border)" }}
                />
                <div
                  className="absolute inset-y-0 left-1/2 -translate-x-1/2 transition-opacity duration-150 opacity-0 group-hover:opacity-100"
                  style={{ width: 3, background: "var(--accent)", borderRadius: 2 }}
                />
              </Separator>
              <Panel defaultSize={65} minSize={30}>
                <FilePreview
                  nodeId={selectedNodeId}
                  fileName={selectedNodeName}
                  onClose={() => setSelectedNodeId(null)}
                />
              </Panel>
            </Group>
          );
        }
        return <FileTreeView />;
      }
      case "impact":
        return <ImpactView />;
      case "docs":
        return <DocsViewer />;
      case "export":
        return <ExportPanel />;
      case "graph":
      default:
        return <GraphExplorer />;
    }
  };

  return (
    <AnimatePresence mode="wait">
      <AnimatedPage key={sidebarTab}>
        {renderView()}
      </AnimatedPage>
    </AnimatePresence>
  );
}
