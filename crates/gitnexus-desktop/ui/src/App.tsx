import { useEffect } from "react";
import { Group, Panel, Separator } from "react-resizable-panels";
import { Sidebar } from "./components/layout/Sidebar";
import { CommandBar } from "./components/layout/CommandBar";
import { StatusBar } from "./components/layout/StatusBar";
import { MainView } from "./components/layout/MainView";
import { DetailPanel } from "./components/layout/DetailPanel";
import { SearchModal } from "./components/search/SearchModal";
import { SettingsModal } from "./components/layout/SettingsModal";
import { ErrorBoundary } from "./components/shared/ErrorBoundary";
import { useAppStore } from "./stores/app-store";
import { useKeyboardShortcuts } from "./hooks/use-keyboard-shortcuts";
import { useResponsive } from "./hooks/use-responsive";
import { useI18n } from "./hooks/use-i18n";

/** Tabs where the detail panel (node inspector) is relevant */
const DETAIL_PANEL_TABS = new Set(["graph", "impact", "search"]);

function App() {
  const { t } = useI18n();
  const activeRepo = useAppStore((s) => s.activeRepo);
  const sidebarTab = useAppStore((s) => s.sidebarTab);
  useKeyboardShortcuts();

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

  const showDetailPanel = activeRepo && DETAIL_PANEL_TABS.has(sidebarTab) && !isNarrow;

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
          {showDetailPanel ? (
            <Group orientation="horizontal" className="h-full">
              <Panel defaultSize={62} minSize={30}>
                <ErrorBoundary>
                  <MainView />
                </ErrorBoundary>
              </Panel>
              <Separator
                className="cursor-col-resize group relative"
                style={{ width: 5, background: "transparent" }}
              >
                {/* Visible drag handle */}
                <div
                  className="absolute inset-y-0 left-1/2 -translate-x-1/2"
                  style={{
                    width: 1,
                    background: "var(--surface-border)",
                  }}
                />
                {/* Hover accent indicator */}
                <div
                  className="absolute inset-y-0 left-1/2 -translate-x-1/2 transition-opacity duration-150 opacity-0 group-hover:opacity-100"
                  style={{
                    width: 3,
                    background: "var(--accent)",
                    borderRadius: 2,
                  }}
                />
                {/* Grip dots visible on hover */}
                <div
                  className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 flex flex-col gap-1 opacity-0 group-hover:opacity-100 transition-opacity duration-150"
                >
                  <span className="w-1 h-1 rounded-full" style={{ background: "var(--accent)" }} />
                  <span className="w-1 h-1 rounded-full" style={{ background: "var(--accent)" }} />
                  <span className="w-1 h-1 rounded-full" style={{ background: "var(--accent)" }} />
                </div>
              </Separator>
              <Panel defaultSize={38} minSize={24}>
                <aside aria-label="Symbol details">
                  <ErrorBoundary>
                    <DetailPanel />
                  </ErrorBoundary>
                </aside>
              </Panel>
            </Group>
          ) : (
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
    </div>
  );
}

export default App;
