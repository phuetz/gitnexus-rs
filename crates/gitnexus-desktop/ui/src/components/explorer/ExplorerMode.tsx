import { useEffect, useRef } from "react";
import { Group, Panel, type ImperativePanelHandle } from "react-resizable-panels";
import { PanelSeparator } from "../layout/PanelSeparator";
import { ExplorerLeftPanel } from "./ExplorerLeftPanel";
import { ExplorerRightPanel } from "./ExplorerRightPanel";
import { GraphExplorer } from "../graph/GraphExplorer";
import { ErrorBoundary } from "../shared/ErrorBoundary";
import { useAppStore } from "../../stores/app-store";
import { useResponsive } from "../../hooks/use-responsive";
import { useI18n } from "../../hooks/use-i18n";

export function ExplorerMode() {
  const activeRepo = useAppStore((s) => s.activeRepo);
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const { isCompact, isNarrow } = useResponsive();
  const { t } = useI18n();

  const leftPanelRef = useRef<ImperativePanelHandle>(null);
  const rightPanelRef = useRef<ImperativePanelHandle>(null);

  useEffect(() => {
    if (isCompact) leftPanelRef.current?.collapse();
    else leftPanelRef.current?.expand();
  }, [isCompact]);

  useEffect(() => {
    if (isNarrow) rightPanelRef.current?.collapse();
    else if (selectedNodeId) rightPanelRef.current?.expand();
  }, [isNarrow, selectedNodeId]);

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
            {t("explorer.noRepo")}
          </p>
          <p
            style={{
              fontSize: 13,
              marginTop: 8,
              color: "var(--text-3)",
            }}
          >
            {t("explorer.noRepoHint")}
          </p>
        </div>
      </div>
    );
  }

  return (
    <Group orientation="horizontal" className="h-full">
      <Panel ref={leftPanelRef} defaultSize={20} minSize={12} maxSize={25} collapsible>
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
        ref={rightPanelRef}
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
