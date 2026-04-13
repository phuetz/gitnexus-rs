/**
 * ChatMode — Full-screen chat interface for the Chat app mode.
 *
 * - Wraps ChatPanel (which manages its own message state + ChatContextBar)
 * - Provides a settings button that opens ChatSettings modal
 * - Cross-mode navigation: source node clicks navigate to Explorer
 * - No-LLM guard: shows setup card when no API key is configured
 * - No-repo guard: shows an empty state when no repo is loaded
 */

import { lazy, Suspense, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { MessageSquare, Settings2, ChevronDown, Database, Compass, BookOpen } from "lucide-react";
import { Group, Panel } from "react-resizable-panels";
import { PanelSeparator } from "../layout/PanelSeparator";
import { ErrorBoundary } from "../shared/ErrorBoundary";
import { commands } from "../../lib/tauri-commands";
import { useAppStore } from "../../stores/app-store";
import { useRepos, useOpenRepo } from "../../hooks/use-tauri-query";
import { ChatPanel } from "./ChatPanel";
import { LoadingOrbs } from "../shared/LoadingOrbs";
import { useResponsive } from "../../hooks/use-responsive";
import { ChatHistorySidebar } from "./ChatHistorySidebar";
import { toast } from "sonner";

const ChatSettings = lazy(() =>
  import("./ChatSettings").then((m) => ({ default: m.ChatSettings })),
);
const GraphExplorer = lazy(() =>
  import("../graph/GraphExplorer").then((m) => ({ default: m.GraphExplorer })),
);

// ─── No-repo empty state ─────────────────────────────────────────────

function NoRepoState() {
  return (
    <div
      className="flex items-center justify-center h-full w-full"
      style={{ color: "var(--text-2)", background: "var(--bg-0)" }}
    >
      <div className="text-center">
        <Database
          size={48}
          style={{ color: "var(--text-4)", margin: "0 auto 16px" }}
        />
        <p
          style={{
            fontFamily: "var(--font-display)",
            fontSize: 20,
            fontWeight: 600,
            color: "var(--text-0)",
          }}
        >
          No repository selected
        </p>
        <p
          style={{
            fontSize: 13,
            marginTop: 8,
            color: "var(--text-3)",
          }}
        >
          Select a repository from the dropdown above to start chatting
        </p>
      </div>
    </div>
  );
}

// ─── No-LLM setup card ───────────────────────────────────────────────

function NoLlmSetup() {
  const setMode = useAppStore((s) => s.setMode);

  return (
    <div className="flex items-center justify-center h-full w-full" style={{ background: "var(--bg-0)" }}>
      <div
        className="text-center p-8 rounded-xl"
        style={{
          background: "var(--glass-bg)",
          border: "1px solid var(--glass-border)",
          maxWidth: 400,
        }}
      >
        <MessageSquare
          size={40}
          style={{ color: "var(--accent)", margin: "0 auto 16px" }}
        />
        <h3
          style={{
            fontFamily: "var(--font-display)",
            fontSize: 18,
            fontWeight: 600,
            color: "var(--text-0)",
            marginBottom: 8,
          }}
        >
          Configure AI Assistant
        </h3>
        <p
          style={{
            fontSize: 13,
            color: "var(--text-2)",
            marginBottom: 20,
            lineHeight: 1.5,
          }}
        >
          Set up an LLM provider to chat about your code. You'll need an API
          key from OpenAI, Anthropic, or another provider — or use Ollama for
          local inference with no key required.
        </p>
        <button
          onClick={() => setMode("manage")}
          className="px-4 py-2 rounded-lg text-sm font-medium transition-colors"
          style={{ background: "var(--accent)", color: "white" }}
        >
          Go to Settings
        </button>
      </div>
    </div>
  );
}

// ─── Repo Selector ───────────────────────────────────────────────────

function RepoSelector() {
  const activeRepo = useAppStore((s) => s.activeRepo);
  const setActiveRepo = useAppStore((s) => s.setActiveRepo);
  const { data: repos } = useRepos();
  const openRepo = useOpenRepo();

  const handleSwitchRepo = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const name = e.target.value;
    if (!name) return;
    try {
      await openRepo.mutateAsync(name);
      setActiveRepo(name);
      toast.success("Switched to " + name);
    } catch (err) {
      console.error(err);
      toast.error(`Failed to switch repo: ${String(err)}`);
    }
  };

  return (
    <div className="relative flex items-center">
      <select
        value={activeRepo || ""}
        onChange={handleSwitchRepo}
        className="appearance-none bg-transparent outline-none pl-2 pr-8 py-1 rounded cursor-pointer text-[14px] font-medium"
        style={{ color: "var(--text-1)", border: "1px solid var(--surface-border)" }}
      >
        <option value="" disabled>Select Repository</option>
        {repos?.map((repo) => (
          <option key={repo.name} value={repo.name}>
            {repo.name}
          </option>
        ))}
      </select>
      <ChevronDown size={14} className="absolute right-2 pointer-events-none" style={{ color: "var(--text-3)" }} />
    </div>
  );
}

// ─── Main component ──────────────────────────────────────────────────

export function ChatMode() {
  const activeRepo = useAppStore((s) => s.activeRepo);
  const setMode = useAppStore((s) => s.setMode);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const { isCompact } = useResponsive();

  // Fetch LLM config to detect unconfigured state
  const { data: chatConfig, isLoading: configLoading } = useQuery({
    queryKey: ["chat-config"],
    queryFn: () => commands.chatGetConfig(),
    // Don't retry aggressively — missing config is not an error
    retry: 1,
  });

  const handleNavigateToNode = (nodeId: string) => {
    setMode("explorer");
    setSelectedNodeId(nodeId, null);
  };

  const isConfigured =
    !chatConfig ||
    chatConfig.provider === "ollama" ||
    (chatConfig.apiKey != null && chatConfig.apiKey.trim().length > 0);

  return (
    <Group orientation="horizontal" className="h-full w-full">
      {/* Sidebar Panel */}
      {!isCompact && (
        <>
          <Panel defaultSize={15} minSize={10} maxSize={25} collapsible>
            <ErrorBoundary>
              <ChatHistorySidebar />
            </ErrorBoundary>
          </Panel>
          <PanelSeparator />
        </>
      )}

      {/* Graph Panel */}
      <Panel minSize={30}>
        <ErrorBoundary>
          <Suspense
            fallback={
              <div className="h-full flex items-center justify-center">
                <LoadingOrbs />
              </div>
            }
          >
            {activeRepo ? <GraphExplorer /> : <NoRepoState />}
          </Suspense>
        </ErrorBoundary>
      </Panel>

      <PanelSeparator />

      {/* Chat Panel */}
      <Panel
        defaultSize={isCompact ? 50 : 35}
        minSize={25}
        maxSize={60}
        collapsible
      >
        <div className="flex flex-col h-full w-full" style={{ background: "var(--bg-0)" }}>
          {/* Header: Repo selector + settings button */}
          <div
            className="shrink-0 flex items-center justify-between px-4 py-2"
            style={{
              background: "var(--glass-bg)",
              backdropFilter: "blur(var(--glass-blur))",
              borderBottom: "1px solid var(--glass-border)",
            }}
          >
            <div className="flex items-center gap-3">
              <MessageSquare size={16} style={{ color: "var(--accent)" }} />
              <RepoSelector />
            </div>
            
            <div className="flex items-center gap-1">
              <button
                onClick={() => setMode("explorer")}
                className="p-1.5 rounded-lg transition-colors"
                style={{ color: "var(--text-3)", cursor: "pointer" }}
                title="Open File Explorer & Graph"
                aria-label="Open Explorer"
              >
                <Compass size={16} />
              </button>
              
              <button
                onClick={() => {
                  useAppStore.getState().setMode("manage");
                  // Depending on the implementation, opening docs might require dispatching an event
                  // or just navigating to manage. The manage mode defaults to showing tabs.
                  // (Assuming Manage Mode handles the "docs" tab selection if implemented or user can click it)
                }}
                className="p-1.5 rounded-lg transition-colors"
                style={{ color: "var(--text-3)", cursor: "pointer" }}
                title="Open Documentation"
                aria-label="Open Documentation"
              >
                <BookOpen size={16} />
              </button>

              <div style={{ width: 1, height: 16, background: "var(--surface-border)", margin: "0 4px" }} />

              <button
                onClick={() => setSettingsOpen(true)}
                className="p-1.5 rounded-lg transition-colors"
                style={{ color: "var(--text-3)", cursor: "pointer" }}
                title="Chat AI Settings"
                aria-label="Open chat AI settings"
              >
                <Settings2 size={16} />
              </button>
            </div>
          </div>

          {/* Main content area */}
          <div className="flex-1 min-h-0">
            {configLoading ? (
              <div className="flex items-center justify-center h-full">
                <div className="pulse-subtle" style={{ color: "var(--text-3)", fontSize: 13 }}>
                  Loading assistant configuration...
                </div>
              </div>
            ) : !isConfigured ? (
              <NoLlmSetup />
            ) : !activeRepo ? (
              <NoRepoState />
            ) : (
              <ChatPanel
                onOpenSettings={() => setSettingsOpen(true)}
                onNavigateToNode={handleNavigateToNode}
              />
            )}
          </div>

          {/* Settings modal */}
          {settingsOpen && (
            <Suspense
              fallback={
                <div className="fixed inset-0 z-50 flex items-center justify-center">
                  <LoadingOrbs />
                </div>
              }
            >
              <ChatSettings onClose={() => setSettingsOpen(false)} />
            </Suspense>
          )}
        </div>
      </Panel>
    </Group>
  );
}
