import { useEffect } from "react";
import { Group, Panel } from "react-resizable-panels";
import { Sidebar } from "./components/layout/Sidebar";
import { CommandBar } from "./components/layout/CommandBar";
import { StatusBar } from "./components/layout/StatusBar";
import { MainView } from "./components/layout/MainView";
import { DetailPanel } from "./components/layout/DetailPanel";
import { CodeInspectorPanel } from "./components/layout/CodeInspectorPanel";
import { PanelSeparator } from "./components/layout/PanelSeparator";
import { SearchModal } from "./components/search/SearchModal";
import { SettingsModal } from "./components/layout/SettingsModal";
import { CommandPalette } from "./components/layout/CommandPalette";
import { ErrorBoundary } from "./components/shared/ErrorBoundary";
import { useAppStore } from "./stores/app-store";
import { useKeyboardShortcuts } from "./hooks/use-keyboard-shortcuts";
import { useResponsive } from "./hooks/use-responsive";
import { useScreenCapture } from "./hooks/use-screen-capture";
import { useI18n } from "./hooks/use-i18n";

/** Tabs where the 2-column detail panel is relevant (not graph — graph uses 3 columns) */
const TWO_COL_DETAIL_TABS = new Set(["impact", "search"]);

function App() {
  const { t } = useI18n();
  const activeRepo = useAppStore((s) => s.activeRepo);
  const sidebarTab = useAppStore((s) => s.sidebarTab);
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  useKeyboardShortcuts();
  useScreenCapture();

  const { isCompact, isNarrow } = useResponsive();

  // Auto-collapse/expand sidebar based on viewport width
  useEffect(() => {
    const collapsed = useAppStore.getState().sidebarCollapsed;
    if (isCompact && !collapsed) {
      useAppStore.getState().toggleSidebar();
    } else if (!isCompact && collapsed) {
      useAppStore.getState().toggleSidebar();
    }
  }, [isCompact]);

  const showThreeCol = activeRepo && sidebarTab === "graph" && !isNarrow;
  const showTwoCol = activeRepo && TWO_COL_DETAIL_TABS.has(sidebarTab) && !isNarrow;

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden" style={{ background: "var(--bg-0)" }}>
      {/* Skip to content link */}
      <a href="#main-content" className="sr-only focus:not-sr-only focus:absolute focus:z-[9999]" style={{ top: 8, left: 8, padding: "8px 16px", backgroundColor: "var(--accent)", color: "white", borderRadius: 8, fontWeight: 600, fontSize: 13 }}>
        {t("a11y.skipToContent")}
      </a>

      {/* Accessible hidden H1 */}
      <h1 className="sr-only">{t("a11y.codeIntelligencePlatform")}</h1>

      {/* Command bar / header */}
      <header>
        <CommandBar />
      </header>

      <div className="flex flex-1 min-h-0">
        {/* Sidebar navigation */}
        <nav aria-label="Main navigation">
          <Sidebar />
        </nav>

        {/* Main content */}
        <main id="main-content" className="flex-1 min-w-0">
          {showThreeCol ? (
            /* ── 3-column layout: Code Inspector | Graph | Analysis ── */
            <Group orientation="horizontal" className="h-full" >
              {/* Left: Code Inspector (visible when node selected) */}
              <Panel
                defaultSize={selectedNodeId ? 22 : 0}
                minSize={0}
                collapsible
              >
                <aside aria-label="Code inspector">
                  <ErrorBoundary>
                    <CodeInspectorPanel />
                  </ErrorBoundary>
                </aside>
              </Panel>
              <PanelSeparator />

              {/* Center: Graph Canvas */}
              <Panel minSize={30}>
                <ErrorBoundary>
                  <MainView />
                </ErrorBoundary>
              </Panel>
              <PanelSeparator />

              {/* Right: Analysis Panel */}
              <Panel defaultSize={28} minSize={20} collapsible>
                <aside aria-label="Symbol details">
                  <ErrorBoundary>
                    <DetailPanel />
                  </ErrorBoundary>
                </aside>
              </Panel>
            </Group>
          ) : showTwoCol ? (
            /* ── 2-column layout: MainView | DetailPanel (impact/search tabs) ── */
            <Group orientation="horizontal" className="h-full" >
              <Panel defaultSize={62} minSize={30}>
                <ErrorBoundary>
                  <MainView />
                </ErrorBoundary>
              </Panel>
              <PanelSeparator />
              <Panel defaultSize={38} minSize={24}>
                <aside aria-label="Symbol details">
                  <ErrorBoundary>
                    <DetailPanel />
                  </ErrorBoundary>
                </aside>
              </Panel>
            </Group>
          ) : (
            /* ── Single column: all other tabs ── */
            <ErrorBoundary>
              <MainView />
            </ErrorBoundary>
          )}
        </main>
      </div>

      {/* Status bar */}
      <footer>
        <StatusBar />
      </footer>

      {/* Search modal overlay */}
      <SearchModal />

      {/* Settings modal */}
      <SettingsModal />

      {/* Command palette */}
      <CommandPalette />
    </div>
  );
}

export default App;
