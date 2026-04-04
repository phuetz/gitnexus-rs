import { useRef, useEffect } from "react";
import { Group, Panel, type PanelImperativeHandle } from "react-resizable-panels";
import { PanelSeparator } from "../layout/PanelSeparator";
import { ExplorerLeftPanel } from "./ExplorerLeftPanel";
import { ExplorerRightPanel } from "./ExplorerRightPanel";
import { GraphExplorer } from "../graph/GraphExplorer";
import { ErrorBoundary } from "../shared/ErrorBoundary";
import { useAppStore } from "../../stores/app-store";
import { useResponsive } from "../../hooks/use-responsive";
import { WelcomeScreen } from "./WelcomeScreen";

export function ExplorerMode() {
  const activeRepo = useAppStore((s) => s.activeRepo);
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const { isCompact } = useResponsive();
  const rightPanelRef = useRef<PanelImperativeHandle>(null);

  // Auto-expand/collapse right panel when node selection changes
  useEffect(() => {
    if (selectedNodeId) {
      rightPanelRef.current?.expand();
    } else {
      rightPanelRef.current?.collapse();
    }
  }, [selectedNodeId]);

  if (!activeRepo) {
    return <WelcomeScreen />;
  }

  return (
    <Group orientation="horizontal" className="h-full">
      <Panel defaultSize={isCompact ? 0 : 20} minSize={12} maxSize={25} collapsible>
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
        panelRef={rightPanelRef}
        defaultSize={0}
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
