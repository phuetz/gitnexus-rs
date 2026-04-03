/**
 * ChatMode — Full-screen chat interface for the Chat app mode.
 *
 * - Wraps ChatPanel (which manages its own message state + ChatContextBar)
 * - Provides a settings button that opens ChatSettings modal
 * - Cross-mode navigation: source node clicks navigate to Explorer
 * - No-LLM guard: shows setup card when no API key is configured
 * - No-repo guard: shows an empty state when no repo is loaded
 */

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { MessageSquare, Settings2 } from "lucide-react";
import { commands } from "../../lib/tauri-commands";
import { useAppStore } from "../../stores/app-store";
import { ChatPanel } from "./ChatPanel";
import { ChatSettings } from "./ChatSettings";

// ─── No-repo empty state ─────────────────────────────────────────────

function NoRepoState() {
  return (
    <div
      className="flex items-center justify-center h-full"
      style={{ color: "var(--text-2)" }}
    >
      <div className="text-center">
        <MessageSquare
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
          No repository loaded
        </p>
        <p
          style={{
            fontSize: 13,
            marginTop: 8,
            color: "var(--text-3)",
          }}
        >
          Open a repository to start chatting about your code
        </p>
      </div>
    </div>
  );
}

// ─── No-LLM setup card ───────────────────────────────────────────────

function NoLlmSetup() {
  const setMode = useAppStore((s) => s.setMode);

  return (
    <div className="flex items-center justify-center h-full">
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

// ─── Main component ──────────────────────────────────────────────────

export function ChatMode() {
  const activeRepo = useAppStore((s) => s.activeRepo);
  const setMode = useAppStore((s) => s.setMode);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Fetch LLM config to detect unconfigured state
  const { data: chatConfig, isLoading: configLoading } = useQuery({
    queryKey: ["chat-config"],
    queryFn: () => commands.chatGetConfig(),
    // Don't retry aggressively — missing config is not an error
    retry: 1,
  });

  // Guard: no repo loaded
  if (!activeRepo) {
    return <NoRepoState />;
  }

  // Guard: config loaded but no LLM configured (non-Ollama with no API key)
  const isConfigured =
    configLoading ||
    !chatConfig ||
    chatConfig.provider === "ollama" ||
    (chatConfig.apiKey != null && chatConfig.apiKey.trim().length > 0);

  if (!isConfigured) {
    return <NoLlmSetup />;
  }

  // Cross-mode navigation handler: navigate to Explorer and select the node
  const handleNavigateToNode = (nodeId: string) => {
    setMode("explorer");
    setSelectedNodeId(nodeId);
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header: title + settings button */}
      <div
        className="shrink-0 flex items-center justify-between px-4 py-2"
        style={{
          background: "var(--glass-bg)",
          backdropFilter: "blur(var(--glass-blur))",
          borderBottom: "1px solid var(--glass-border)",
        }}
      >
        <h2
          style={{
            fontFamily: "var(--font-display)",
            fontSize: 16,
            fontWeight: 600,
            color: "var(--text-0)",
          }}
        >
          Chat
        </h2>
        <button
          onClick={() => setSettingsOpen(true)}
          className="p-1.5 rounded-lg transition-colors"
          style={{ color: "var(--text-3)" }}
          title="Chat AI Settings"
          aria-label="Open chat AI settings"
        >
          <Settings2 size={16} />
        </button>
      </div>

      {/* Chat panel — manages its own messages, context bar, and filter modals */}
      <div className="flex-1 min-h-0">
        <ChatPanel
          onOpenSettings={() => setSettingsOpen(true)}
          onNavigateToNode={handleNavigateToNode}
        />
      </div>

      {/* Settings modal */}
      {settingsOpen && (
        <ChatSettings onClose={() => setSettingsOpen(false)} />
      )}
    </div>
  );
}
