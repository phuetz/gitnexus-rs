import { Group, Panel } from "react-resizable-panels";
import { PanelSeparator } from "../layout/PanelSeparator";
import { ExplorerLeftPanel } from "./ExplorerLeftPanel";
import { ExplorerRightPanel } from "./ExplorerRightPanel";
import { GraphExplorer } from "../graph/GraphExplorer";
import { ErrorBoundary } from "../shared/ErrorBoundary";
import { useAppStore } from "../../stores/app-store";

export function ExplorerMode() {
  const activeRepo = useAppStore((s) => s.activeRepo);
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);

  if (!activeRepo) {
    return (
      <div
        className="flex items-center justify-center h-full"
        style={{ color: "var(--text-2)" }}
      >
        <div className="text-center">
          <p
            style={{
              fontFamily: "var(--font-display)",
              fontSize: 20,
              fontWeight: 600,
            }}
          >
            No repository loaded
          </p>
          <p
            style={{
              fontSize: 13,
              marginTop: 8,
              color: "var(--text-3)",
            }}
          >
            Open a repository from the Manage tab to start exploring
          </p>
        </div>
      </div>
    );
  }

  return (
    <Group orientation="horizontal" className="h-full">
      <Panel defaultSize={20} minSize={12} maxSize={25} collapsible>
        <ErrorBoundary>
          <ExplorerLeftPanel />
        </ErrorBoundary>
      </Panel>
      <PanelSeparator />
      <Panel minSize={30}>
        <ErrorBoundary>
          <GraphExplorer />
        </ErrorBoundary>
      </Panel>
      <PanelSeparator />
      <Panel
        defaultSize={selectedNodeId ? 28 : 0}
        minSize={0}
        maxSize={35}
        collapsible
      >
        <ErrorBoundary>
          <ExplorerRightPanel />
        </ErrorBoundary>
      </Panel>
    </Group>
  );
}
