/**
 * ChatPanel — Intelligent Q&A chat with IDE-style filtering and multi-step research.
 */

import { lazy, Suspense, useState, useRef, useEffect, useCallback, useMemo } from "react";
import { useMutation } from "@tanstack/react-query";
import { useI18n } from "../../hooks/use-i18n";
import { confirm } from "../../lib/confirm";
import { ChatSuggestions } from "./ChatSuggestions";
import {
  Loader2,
  Sparkles,
  Microscope,
  Hammer,
  Trash2,
  Download,
  Wrench,
  Search,
  FileJson,
} from "lucide-react";
import { toast } from "sonner";
import { commands } from "../../lib/tauri-commands";
import type {
  ChatSmartResponse,
  FeatureDevArtifact,
  CodeReviewArtifact,
  SimplifyArtifact,
} from "../../lib/tauri-commands";
import { useAppStore } from "../../stores/app-store";
import { useChatStore } from "../../stores/chat-store";
import { useChatSessionStore, type Message } from "../../stores/chat-session-store";
import { ChatContextBar } from "./ChatContextBar";
import { ArtifactPanel } from "./ArtifactPanel";
import { ShieldCheck } from "lucide-react";

// Refactored components and hooks
import { ChatInput } from "./ChatInput";
import { ChatMessage } from "./ChatMessage";
import { useChatStream } from "../../hooks/use-chat-stream";
import { CodePreviewPanel } from "./CodePreviewPanel";
import { ResearchProgress } from "./ResearchProgress";

const FileFilterModal = lazy(() =>
  import("./FileFilterModal").then((m) => ({ default: m.FileFilterModal })),
);
const SymbolFilterModal = lazy(() =>
  import("./SymbolFilterModal").then((m) => ({ default: m.SymbolFilterModal })),
);
const ModuleFilterModal = lazy(() =>
  import("./ModuleFilterModal").then((m) => ({ default: m.ModuleFilterModal })),
);
const ChatMarkdown = lazy(() =>
  import("./ChatMarkdown").then((m) => ({ default: m.ChatMarkdown })),
);
const ChatToolsPanel = lazy(() =>
  import("./ChatToolsPanel").then((m) => ({ default: m.ChatToolsPanel })),
);
const ChatSearch = lazy(() =>
  import("./ChatSearch").then((m) => ({ default: m.ChatSearch })),
);

// ─── Props ──────────────────────────────────────────────────────────

interface ChatPanelProps {
  onOpenSettings?: () => void;
  onNavigateToNode?: (nodeId: string) => void;
}

const modalFallback = null;

function MarkdownFallback({ content }: { content: string }) {
  return <div className="whitespace-pre-wrap">{content}</div>;
}

function renderFilterModals(activeModal: string | null, closeModal: () => void) {
  return (
    <Suspense fallback={modalFallback}>
      {activeModal === "files" && <FileFilterModal open onClose={closeModal} />}
      {activeModal === "symbols" && <SymbolFilterModal open onClose={closeModal} />}
      {activeModal === "modules" && <ModuleFilterModal open onClose={closeModal} />}
    </Suspense>
  );
}

// ─── Component ──────────────────────────────────────────────────────

export function ChatPanel({ onOpenSettings, onNavigateToNode }: ChatPanelProps) {
  const { t } = useI18n();
  const activeRepo = useAppStore((s) => s.activeRepo) || "global";
  
  const sessions = useChatSessionStore(s => s.sessions);
  const activeSessionId = useChatSessionStore(s => s.activeSessionId);
  const setActiveSession = useChatSessionStore(s => s.setActiveSession);
  const addMessage = useChatSessionStore(s => s.addMessage);
  const clearSessionMessages = useChatSessionStore(s => s.clearSessionMessages);

  const activeSession = useMemo(() => {
    if (activeSessionId) {
      const session = sessions.find(s => s.id === activeSessionId && s.repo === activeRepo);
      if (session) return session;
    }
    const repoSessions = sessions.filter(s => s.repo === activeRepo).sort((a, b) => b.updatedAt - a.updatedAt);
    return repoSessions[0] ?? null;
  }, [sessions, activeSessionId, activeRepo]);

  useEffect(() => {
    if (activeSession && activeSession.id !== activeSessionId) {
      setActiveSession(activeSession.id);
    }
  }, [activeSession, activeSessionId, setActiveSession]);

  const messages = useMemo(() => activeSession?.messages || [], [activeSession?.messages]);

  const [input, setInput] = useState("");
  const [previewFile, setPreviewFile] = useState<{ path: string; startLine?: number; endLine?: number } | null>(null);
  const [toolsPanelOpen, setToolsPanelOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const {
    streamingText,
    setStreamingText,
    isStreaming,
    setIsStreaming,
    activeTools,
    toolHistory,
    setToolHistory,
    liveArtifact,
    setLiveArtifact,
    activePhase,
    setActivePhase
  } = useChatStream();

  const streamingMsgIdRef = useRef<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Global "focus the chat input" shortcut: Ctrl+L from anywhere switches
  // to Chat mode and dispatches this event. We listen here because the
  // input ref only exists when ChatPanel is mounted.
  useEffect(() => {
    const onFocus = () => {
      inputRef.current?.focus();
      inputRef.current?.select();
    };
    window.addEventListener("gitnexus:focus-chat-input", onFocus);
    return () => window.removeEventListener("gitnexus:focus-chat-input", onFocus);
  }, []);

  const {
    chatMode,
    setChatMode,
    deepResearchEnabled,
    activeModal,
    closeModal,
    hasActiveFilters,
    setActivePlan,
    pendingQuestion,
    clearPendingQuestion,
  } = useChatStore();

  // Scroll to bottom when messages change or streaming updates
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamingText, activeTools]);

  // ── Keyboard shortcuts for filter modals ─────────────────────
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "p" && !e.shiftKey) {
        e.preventDefault();
        useChatStore.getState().openModal("files");
      }
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "O") {
        e.preventDefault();
        useChatStore.getState().openModal("symbols");
      }
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "M") {
        e.preventDefault();
        useChatStore.getState().openModal("modules");
      }
      // Theme B — cross-session search
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === "F" || e.key === "f")) {
        e.preventDefault();
        setSearchOpen(true);
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // Listen for a global event so the command palette / menus can open
  // the cross-session search modal without reaching into this component.
  useEffect(() => {
    const onOpenSearch = () => setSearchOpen(true);
    window.addEventListener("gitnexus:open-chat-search", onOpenSearch);
    return () => window.removeEventListener("gitnexus:open-chat-search", onOpenSearch);
  }, []);

  // ── Smart Ask mutation (uses plan executor for deep research) ─
  const messagesRef = useRef(messages);
  useEffect(() => { messagesRef.current = messages; }, [messages]);

  const askMutation = useMutation({
    mutationFn: async (question: string): Promise<
      | ChatSmartResponse
      | { __kind: "feature_dev"; artifact: FeatureDevArtifact }
      | { __kind: "code_review"; artifact: CodeReviewArtifact }
      | { __kind: "simplify"; artifact: SimplifyArtifact }
    > => {
      const history = [
        ...messagesRef.current,
        { role: "user" as const, content: question },
      ].map((m) => ({
        role: m.role,
        content: m.content,
      }));

      const mode = useChatStore.getState().chatMode;
      const currentFilters = useChatStore.getState().filters;
      const isDeep = mode === "deep_research";
      const hasFilters = useChatStore.getState().hasActiveFilters();

      if (mode === "simplify") {
        const match = question.match(/^\s*simplify:\s*(.+)$/i);
        const target = match ? match[1].trim() : question.trim() || undefined;
        const artifact = await commands.simplifyRun({ target });
        return { __kind: "simplify", artifact };
      }

      if (mode === "code_review") {
        const match = question.match(/^\s*review:\s*(.+)$/i);
        const targetSymbols = match
          ? match[1].split(/[,\s]+/).map((s) => s.trim()).filter(Boolean)
          : [];
        const artifact = await commands.codeReviewRun({
          targetSymbols,
          minConfidence: 0.8,
          includeAllSeverities: false,
        });
        return { __kind: "code_review", artifact };
      }

      if (mode === "feature_dev") {
        const placeholderId = `fd_pending_${Date.now()}`;
        setLiveArtifact({
          id: placeholderId,
          featureDescription: question,
          sections: [],
          status: "running",
        });
        setActivePhase("explorer");

        const artifact = await commands.featureDevRun({
          featureDescription: question,
          filters: hasFilters ? currentFilters : undefined,
        });
        setLiveArtifact((prev: FeatureDevArtifact | null) =>
          prev && prev.id === placeholderId ? { ...prev, id: artifact.id } : prev,
        );
        return { __kind: "feature_dev", artifact };
      }

      if (isDeep || hasFilters) {
        return commands.chatExecutePlan({
          question,
          history,
          filters: hasFilters ? currentFilters : undefined,
          deepResearch: isDeep,
        });
      }

      setStreamingText("");
      setToolHistory([]);
      setIsStreaming(true);
      streamingMsgIdRef.current = `msg-${Date.now()}-stream`;

      const response = await commands.chatAsk({ question, history });
      // Don't fake a complexity value — chatAsk is the streaming agentic path
      // and doesn't run the planner. Leaving `complexity` undefined hides the
      // badge, which is the honest UX; the former hardcoded "simple" value
      // triggered a "⚡ Réponse rapide" badge even when the agent ran 5 tool
      // iterations.
      return {
        answer: response.answer,
        sources: response.sources,
        model: response.model,
      } as ChatSmartResponse;
    },
    onSuccess: (response) => {
      if ("__kind" in response && response.__kind === "simplify") {
        setIsStreaming(false);
        const proposalCount = response.artifact.proposals.length;
        const assistantMessage: Message = {
          id: `msg-${Date.now()}`,
          role: "assistant",
          content: `**Simplify done.** ${proposalCount} proposal${proposalCount === 1 ? "" : "s"}.`,
          simplifyArtifact: response.artifact,
          timestamp: Date.now(),
        };
        addMessage(activeRepo, assistantMessage);
        return;
      }

      if ("__kind" in response && response.__kind === "code_review") {
        setIsStreaming(false);
        const issueCount = response.artifact.review.issues.length;
        const verdict = response.artifact.review.verdict;
        const assistantMessage: Message = {
          id: `msg-${Date.now()}`,
          role: "assistant",
          content: `**Code review complete.** Verdict: ${verdict} · ${issueCount} issue(s).`,
          reviewArtifact: response.artifact,
          timestamp: Date.now(),
        };
        addMessage(activeRepo, assistantMessage);
        return;
      }

      if ("__kind" in response && response.__kind === "feature_dev") {
        setIsStreaming(false);
        setActivePhase(null);
        setLiveArtifact(null);
        const assistantMessage: Message = {
          id: `msg-${Date.now()}`,
          role: "assistant",
          content: `**Feature-Dev artifact generated.** ${response.artifact.sections.length} sections.`,
          artifact: response.artifact,
          timestamp: Date.now(),
        };
        addMessage(activeRepo, assistantMessage);
        return;
      }

      setIsStreaming(false);
      setStreamingText("");
      streamingMsgIdRef.current = null;

      if (response.sources && response.sources.length > 0) {
        useAppStore.getState().setSearchMatchIds(response.sources.map(s => s.nodeId));
      } else {
        useAppStore.getState().setSearchMatchIds([]);
      }

      const assistantMessage: Message = {
        id: `msg-${Date.now()}`,
        role: "assistant",
        content: response.answer,
        sources: response.sources,
        model: response.model,
        plan: response.plan ?? undefined,
        complexity: response.complexity,
        timestamp: Date.now(),
      };
      addMessage(activeRepo, assistantMessage);
      if (response.plan) {
        setActivePlan(response.plan);
      }
    },
    onError: (error) => {
      setIsStreaming(false);
      setStreamingText("");
      streamingMsgIdRef.current = null;
      setActivePhase(null);
      setLiveArtifact(null);

      const errorMessage: Message = {
        id: `msg-${Date.now()}`,
        role: "assistant",
        content: `**Error:** ${(error as Error).message}\n\nCould not get an AI response. Check your LLM configuration in Chat Settings (\u2699\uFE0F button), or use Ollama for local inference.`,
        timestamp: Date.now(),
      };
      addMessage(activeRepo, errorMessage);
    },
  });

  const handleSend = useCallback(async () => {
    const question = input.trim();
    if (!question || askMutation.isPending) return;

    // GitNexus built-in slash commands (Copilot-inspired)
    const builtinSlash: Record<string, string> = {
      "expliquer":    "Explique le module ou symbole suivant : ",
      "expliquer":    "Explique le module ou symbole suivant : ",
      "algorithme":   "Décris l'algorithme étape par étape de : ",
      "impact":       "Analyse le blast radius et les dépendances de : ",
      "architecture": "Présente l'architecture globale de : ",
      "diagramme":    "Génère un diagramme Mermaid pour : ",
      "explain":      "Explain in detail: ",
      "algorithm":    "Describe the algorithm step by step for: ",
    };
    const slashMatch = question.match(/^\/(\w[\w-]*)\b\s*(.*)$/);
    if (slashMatch) {
      const builtinPrefix = builtinSlash[slashMatch[1]];
      if (builtinPrefix) {
        const expandedQuestion = builtinPrefix + (slashMatch[2] || "");
        setInput("");
        useChatStore.getState().dispatchQuestion("qa", expandedQuestion, true);
        return;
      }
      try {
        const resolved = await commands.userCommandResolve(slashMatch[1], slashMatch[2]);
        if (resolved) {
          setInput("");
          useChatStore
            .getState()
            .dispatchQuestion(
              (resolved.mode as "qa" | "deep_research" | "feature_dev" | "code_review" | "simplify") ||
                "qa",
              resolved.text,
              true,
            );
          return;
        }
      } catch {
        // Fall through
      }
    }

    const userMessage: Message = {
      id: `msg-${Date.now()}`,
      role: "user",
      content: question,
      timestamp: Date.now(),
    };
    addMessage(activeRepo, userMessage);
    setInput("");

    askMutation.mutate(question);
  }, [input, askMutation, addMessage, activeRepo]);

  useEffect(() => {
    if (!pendingQuestion) return;
    if (pendingQuestion.autoSend && !askMutation.isPending) {
      const text = pendingQuestion.text;
      const userMessage: Message = {
        id: `msg-${Date.now()}`,
        role: "user",
        content: text,
        timestamp: Date.now(),
      };
      addMessage(activeRepo, userMessage);
      // eslint-disable-next-line react-hooks/set-state-in-effect -- syncing external store event (pendingQuestion) into local input; single-shot, store is cleared immediately below
      setInput("");
      askMutation.mutate(text);
    } else {
      setInput(pendingQuestion.text);
      inputRef.current?.focus();
    }
    clearPendingQuestion();
  }, [pendingQuestion, askMutation, addMessage, activeRepo, clearPendingQuestion]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  // ─── Render ──────────────────────────────────────────────────

  return (
    <div className="h-full flex flex-row overflow-hidden bg-bg-0">
      <div className={`flex-1 flex flex-col min-w-0 transition-all duration-300 ${previewFile ? 'max-w-[50%]' : 'max-w-full'}`}>
        {/* Context filter bar + mode switcher */}
        <div className="flex items-center">
          <div className="flex-1">
            <ChatContextBar />
          </div>
          <div className="flex items-center gap-1 mr-2">
            <ModeSwitcher mode={chatMode} onChange={setChatMode} />
            <button
              onClick={() => setSearchOpen(true)}
              className="text-[11px] hover:bg-[var(--bg-3)] rounded px-2 py-1"
              style={{ color: "var(--text-3)" }}
              title={(t("chat.searchAll") || "Search all sessions") + " (Ctrl+Shift+F)"}
            >
              <Search size={12} />
            </button>
            <button
              onClick={() => setToolsPanelOpen((v) => !v)}
              className="text-[11px] hover:bg-[var(--bg-3)] rounded px-2 py-1"
              style={{
                color: toolsPanelOpen ? "var(--accent)" : "var(--text-3)",
                background: toolsPanelOpen ? "var(--accent-subtle)" : undefined,
              }}
              title={t("chat.toolsPanel") || "Agent tools panel"}
            >
              <Wrench size={12} />
            </button>
            {messages.length > 0 && (
              <>
                <button
                  onClick={() => {
                    const date = new Date().toISOString().split("T")[0];
                    const filename = `gitnexus-chat-${activeRepo || "global"}-${date}.md`;
                    const content = messages
                      .map((m) => `### ${m.role === "user" ? "You" : "GitNexus"}\n\n${m.content}\n`)
                      .join("\n---\n\n");
                    const blob = new Blob([content], { type: "text/markdown" });
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement("a");
                    a.href = url;
                    a.download = filename;
                    document.body.appendChild(a);
                    a.click();
                    document.body.removeChild(a);
                    setTimeout(() => URL.revokeObjectURL(url), 1000);
                    toast.success(t("chat.exportedAsMarkdown"));
                  }}
                  className="text-[11px] hover:bg-[var(--bg-3)] rounded px-2 py-1"
                  style={{ color: "var(--text-3)" }}
                  title={t("chat.exportChatMarkdown")}
                >
                  <Download size={12} />
                </button>
                <button
                  onClick={() => {
                    const date = new Date().toISOString().split("T")[0];
                    const filename = `gitnexus-chat-${activeRepo || "global"}-${date}.json`;
                    // Structured dump: include every persisted field so a
                    // round-trip can rebuild the session exactly (useful for
                    // sharing a retry/fork trail or archiving an experiment).
                    const payload = {
                      exportedAt: new Date().toISOString(),
                      repo: activeRepo,
                      session: activeSession
                        ? {
                            id: activeSession.id,
                            title: activeSession.title,
                            parentId: activeSession.parentId,
                            branchFromMessageId: activeSession.branchFromMessageId,
                            updatedAt: activeSession.updatedAt,
                          }
                        : null,
                      messages,
                    };
                    const blob = new Blob([JSON.stringify(payload, null, 2)], {
                      type: "application/json",
                    });
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement("a");
                    a.href = url;
                    a.download = filename;
                    document.body.appendChild(a);
                    a.click();
                    document.body.removeChild(a);
                    setTimeout(() => URL.revokeObjectURL(url), 1000);
                    toast.success(t("chat.exportedAsJson") || "Chat exported as JSON");
                  }}
                  className="text-[11px] hover:bg-[var(--bg-3)] rounded px-2 py-1"
                  style={{ color: "var(--text-3)" }}
                  title={t("chat.exportChatJson") || "Export chat as JSON (messages + toolCalls)"}
                >
                  <FileJson size={12} />
                </button>
                <button
                  onClick={async () => {
                    const ok = await confirm({
                      title: t("confirm.deleteTitle"),
                      message: t("chat.confirmClear"),
                      confirmLabel: t("confirm.delete"),
                      danger: true,
                    });
                    if (!ok) return;
                    clearSessionMessages(activeRepo);
                    toast.success(t("chat.conversationCleared"));
                  }}
                  className="text-[11px] hover:bg-[var(--bg-3)] rounded px-2 py-1"
                  style={{ color: "var(--text-3)" }}
                  title={t("chat.clearConversation")}
                >
                  <Trash2 size={12} />
                </button>
              </>
            )}
          </div>
        </div>

        {/* Content Area */}
        <div className="flex-1 min-h-0 overflow-auto">
          {messages.length === 0 ? (
            {/* Slash command quick chips */}
            <div className="flex flex-wrap gap-1.5 px-4 pb-2">
              {[
                { cmd: "/expliquer ", label: "/expliquer", icon: "📖" },
                { cmd: "/algorithme ", label: "/algorithme", icon: "⚙️" },
                { cmd: "/impact ", label: "/impact", icon: "💥" },
                { cmd: "/architecture ", label: "/architecture", icon: "🏗️" },
                { cmd: "/diagramme ", label: "/diagramme", icon: "📊" },
              ].map(({ cmd, label, icon }) => (
                <button
                  key={cmd}
                  onClick={() => { setInput(cmd); inputRef.current?.focus(); }}
                  className="px-2 py-0.5 rounded-full text-[11px] transition-all hover:opacity-80"
                  style={{ background: "var(--bg-2)", border: "1px solid var(--surface-border)", color: "var(--text-2)" }}
                >
                  {icon} {label}
                </button>
              ))}
            </div>
            <ChatSuggestions onSelect={(q) => { setInput(q); inputRef.current?.focus(); }} />
          ) : (
            <div className="px-4 py-4 space-y-4" aria-live="polite">
              {messages.map((msg) => (
                  <ChatMessage
                    key={msg.id}
                    message={msg}
                    sessionId={activeSession?.id}
                    onNavigateToNode={onNavigateToNode}
                    onFilePreview={setPreviewFile}
                  />
                ))}

              {/* Research Progress Pipeline */}
              {toolHistory.length > 0 && isStreaming && (
                <ResearchProgress steps={toolHistory} />
              )}

              {/* Active tool badges */}              {activeTools.length > 0 && (
                <div className="flex flex-wrap gap-1.5 px-1 py-1">
                  {activeTools.map((tool) => (
                    <span
                      key={tool}
                      className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium"
                      style={{
                        background: "color-mix(in srgb, var(--orange) 15%, transparent)",
                        color: "var(--orange)",
                      }}
                    >
                      <Loader2 size={9} className="animate-spin" />
                      {tool}
                    </span>
                  ))}
                </div>
              )}

              {/* Streaming response */}
              {isStreaming && streamingText && (
                <div className="fade-in">
                  <div className="flex items-center gap-1.5 mb-1">
                    <span className="w-2 h-2 rounded-full flex-shrink-0" style={{ background: "var(--purple)" }} />
                    <span className="text-[11px] font-medium" style={{ color: "var(--text-3)" }}>GitNexus</span>
                  </div>
                  <div className="prose-sm text-[13px] leading-relaxed" style={{ color: "var(--text-1)" }}>
                    <Suspense fallback={<MarkdownFallback content={streamingText} />}>
                      <ChatMarkdown content={streamingText} onNavigateToNode={onNavigateToNode} />
                    </Suspense>
                    <span className="typing-cursor" />
                  </div>
                </div>
              )}

              {/* Thinking state */}
              {askMutation.isPending && !isStreaming && (
                <div className="fade-in">
                  <div className="flex items-center gap-1.5 mb-1">
                    <span className="w-2 h-2 rounded-full flex-shrink-0" style={{ background: "var(--purple)" }} />
                    <span className="text-[11px] font-medium" style={{ color: "var(--text-3)" }}>GitNexus</span>
                    <span className="text-[11px]" style={{ color: "var(--text-3)" }}>
                      {deepResearchEnabled ? t("chat.executingResearch") : hasActiveFilters() ? t("chat.searchingContext") : t("chat.thinking")}
                    </span>
                  </div>
                  <div className="space-y-2 py-4 px-4">
                    <div className="shimmer rounded" style={{ height: 14, width: "80%", background: "var(--bg-3)" }} />
                    <div className="shimmer rounded" style={{ height: 14, width: "65%", background: "var(--bg-3)" }} />
                  </div>
                </div>
              )}

              {/* Live artifact */}
              {liveArtifact && (
                <div className="fade-in">
                  <div className="flex items-center gap-1.5 mb-1">
                    <Hammer size={12} style={{ color: "var(--accent)" }} />
                    <span className="text-[11px] font-medium" style={{ color: "var(--text-3)" }}>Feature-Dev in progress…</span>
                  </div>
                  <ArtifactPanel artifact={liveArtifact} activePhase={activePhase} />
                </div>
              )}

              <div ref={messagesEndRef} />
            </div>
          )}
        </div>

        {/* Input area */}
        <ChatInput
          ref={inputRef}
          value={input}
          onChange={setInput}
          onSend={handleSend}
          onKeyDown={handleKeyDown}
          isPending={askMutation.isPending}
          onOpenSettings={onOpenSettings}
          deepResearch={deepResearchEnabled}
          hasFilters={hasActiveFilters()}
          isStreaming={isStreaming}
          onCancel={() => { void commands.chatCancel(); }}
        />
      </div>

      {/* Code Preview Split Panel */}
      {previewFile && (
        <div className="w-1/2 min-w-[300px] border-l border-surface-border fade-in">
          <CodePreviewPanel
            filePath={previewFile.path}
            startLine={previewFile.startLine}
            endLine={previewFile.endLine}
            onClose={() => setPreviewFile(null)}
          />
        </div>
      )}

      {/* Tools panel right rail */}
      {toolsPanelOpen && !previewFile && (
        <div className="w-[300px] min-w-[260px] fade-in">
          <Suspense fallback={null}>
            <ChatToolsPanel onClose={() => setToolsPanelOpen(false)} />
          </Suspense>
        </div>
      )}

      {/* Cross-session search modal */}
      <Suspense fallback={null}>
        <ChatSearch open={searchOpen} onClose={() => setSearchOpen(false)} />
      </Suspense>

      {/* Filter modals */}
      {renderFilterModals(activeModal, closeModal)}
    </div>
  );
}

// ─── ModeSwitcher ──────────────────────────────────────────────────

function ModeSwitcher({
  mode,
  onChange,
}: {
  mode: "qa" | "deep_research" | "feature_dev" | "code_review" | "simplify";
  onChange: (
    m: "qa" | "deep_research" | "feature_dev" | "code_review" | "simplify",
  ) => void;
}) {
  const { t } = useI18n();
  const modes: Array<{
    key: "qa" | "deep_research" | "feature_dev" | "code_review" | "simplify";
    icon: typeof Microscope;
    label: string;
    tooltip: string;
    color: string;
  }> = [
    {
      key: "qa",
      icon: Sparkles,
      label: t("chat.mode.qa.label"),
      tooltip: t("chat.mode.qa.tooltip"),
      color: "var(--accent)",
    },
    {
      key: "deep_research",
      icon: Microscope,
      label: t("chat.mode.research.label"),
      tooltip: t("chat.mode.research.tooltip"),
      color: "var(--purple)",
    },
    {
      key: "feature_dev",
      icon: Hammer,
      label: t("chat.mode.featureDev.label"),
      tooltip: t("chat.mode.featureDev.tooltip"),
      color: "var(--orange, #e0af68)",
    },
    {
      key: "code_review",
      icon: ShieldCheck,
      label: t("chat.mode.review.label"),
      tooltip: t("chat.mode.review.tooltip"),
      color: "var(--green, #9ece6a)",
    },
    {
      key: "simplify",
      icon: Sparkles,
      label: t("chat.mode.simplify.label"),
      tooltip: t("chat.mode.simplify.tooltip"),
      color: "var(--purple, #bb9af7)",
    },
  ];

  return (
    <div
      className="flex items-center gap-0.5 rounded-md p-0.5"
      style={{
        background: "var(--surface)",
        border: "1px solid var(--surface-border)",
      }}
    >
      {modes.map((m) => {
        const active = m.key === mode;
        const Icon = m.icon;
        return (
          <button
            key={m.key}
            onClick={() => onChange(m.key)}
            title={m.tooltip}
            aria-pressed={active}
            className="inline-flex items-center gap-1 rounded-sm transition-colors"
            style={{
              padding: "3px 6px",
              fontSize: 10,
              fontWeight: 500,
              background: active ? m.color : "transparent",
              color: active ? "#fff" : "var(--text-3)",
              cursor: "pointer",
              border: "none",
              fontFamily: "inherit",
            }}
          >
            <Icon size={10} />
            <span>{m.label}</span>
          </button>
        );
      })}
    </div>
  );
}
