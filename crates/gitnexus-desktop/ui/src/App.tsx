import { Group, Panel, Separator } from "react-resizable-panels";
import { Sidebar } from "./components/layout/Sidebar";
import { CommandBar } from "./components/layout/CommandBar";
import { StatusBar } from "./components/layout/StatusBar";
import { MainView } from "./components/layout/MainView";
import { DetailPanel } from "./components/layout/DetailPanel";
import { SearchModal } from "./components/search/SearchModal";
import { ErrorBoundary } from "./components/shared/ErrorBoundary";
import { useAppStore } from "./stores/app-store";
import { useKeyboardShortcuts } from "./hooks/use-keyboard-shortcuts";

function App() {
  const activeRepo = useAppStore((s) => s.activeRepo);
  useKeyboardShortcuts();

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden" style={{ background: "var(--bg-0)" }}>
      {/* Command bar / header */}
      <CommandBar />

      <div className="flex flex-1 min-h-0">
        {/* Sidebar navigation */}
        <Sidebar />

        {/* Main content */}
        {activeRepo ? (
          <Group orientation="horizontal" className="flex-1">
            <Panel defaultSize={62} minSize={30}>
              <ErrorBoundary>
                <MainView />
              </ErrorBoundary>
            </Panel>
            <Separator className="w-[3px] bg-transparent hover:bg-[var(--accent)] transition-colors cursor-col-resize group relative">
              <div className="absolute inset-y-0 -left-[2px] -right-[2px] group-hover:bg-[var(--accent-subtle)]" />
            </Separator>
            <Panel defaultSize={38} minSize={24}>
              <ErrorBoundary>
                <DetailPanel />
              </ErrorBoundary>
            </Panel>
          </Group>
        ) : (
          <div className="flex-1">
            <ErrorBoundary>
              <MainView />
            </ErrorBoundary>
          </div>
        )}
      </div>

      {/* Status bar */}
      <StatusBar />

      {/* Search modal overlay */}
      <SearchModal />
    </div>
  );
}

export default App;
