/**
 * ChatPanel — Intelligent Q&A chat with IDE-style filtering and multi-step research.
 *
 * Features:
 * - Natural language questions answered using knowledge graph + LLM
 * - IDE-style file/symbol/module filtering (Ctrl+P, Ctrl+Shift+O)
 * - Query complexity analysis with visual indicators
 * - Deep Research mode: multi-step plans executed like Manus AI
 * - Source citations with expandable code snippets
 * - Markdown-rendered responses with syntax highlighting
 * - Conversation history
 */

import { lazy, Suspense, useState, useRef, useEffect, useCallback, useMemo, forwardRef } from "react";
import { useMutation } from "@tanstack/react-query";
import { useI18n } from "../../hooks/use-i18n";
import { ChatSuggestions } from "./ChatSuggestions";
import {
  Send,
  Loader2,
  Settings2,
  Sparkles,
  Microscope,
  Zap,
  Copy,
  Trash2,
  Download,
} from "lucide-react";
import { toast } from "sonner";
import { commands } from "../../lib/tauri-commands";
import { isTauri } from "../../lib/tauri-env";
import type {
  ChatSmartResponse,
  QueryComplexity,
} from "../../lib/tauri-commands";
import { useAppStore } from "../../stores/app-store";
import { useChatStore } from "../../stores/chat-store";
import { useChatSessionStore, type Message } from "../../stores/chat-session-store";
import { ChatContextBar } from "./ChatContextBar";

const FileFilterModal = lazy(() =>
  import("./FileFilterModal").then((m) => ({ default: m.FileFilterModal })),
);
const SymbolFilterModal = lazy(() =>
  import("./SymbolFilterModal").then((m) => ({ default: m.SymbolFilterModal })),
);
const ModuleFilterModal = lazy(() =>
  import("./ModuleFilterModal").then((m) => ({ default: m.ModuleFilterModal })),
);
const ResearchPlanViewer = lazy(() =>
  import("./ResearchPlanViewer").then((m) => ({ default: m.ResearchPlanViewer })),
);
const SourceReferences = lazy(() =>
  import("./SourceReferences").then((m) => ({ default: m.SourceReferences })),
);
const ChatMarkdown = lazy(() =>
  import("./ChatMarkdown").then((m) => ({ default: m.ChatMarkdown })),
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
  
  const getActiveSession = useChatSessionStore(s => s.getActiveSession);
  const addMessage = useChatSessionStore(s => s.addMessage);
  const clearSessionMessages = useChatSessionStore(s => s.clearSessionMessages);

  const activeSession = getActiveSession(activeRepo);
  const messages = useMemo(() => activeSession?.messages || [], [activeSession?.messages]);

  const [input, setInput] = useState("");
  const [streamingText, setStreamingText] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
  const [activeTools, setActiveTools] = useState<string[]>([]);
  const streamingMsgIdRef = useRef<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const {
    deepResearchEnabled,
    activeModal,
    closeModal,
    hasActiveFilters,
    setActivePlan,
  } = useChatStore();

  // Scroll to bottom when messages change or streaming updates
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamingText, activeTools]);

  // ── Listen for SSE stream chunks from the backend ───────────
  useEffect(() => {
    if (!isTauri()) return;

    let cancelled = false;
    let chunkUnlisten: (() => void) | null = null;
    let doneUnlisten: (() => void) | null = null;
    let toolStartUnlisten: (() => void) | null = null;
    let toolEndUnlisten: (() => void) | null = null;

    import("@tauri-apps/api/event").then((mod) => {
      mod.listen<string>("chat-stream-chunk", (event) => {
        if (cancelled) return;
        setStreamingText((prev) => prev + event.payload);
      }).then((fn) => {
        if (cancelled) fn(); else chunkUnlisten = fn;
      });

      mod.listen<string>("tool_execution_start", (event) => {
        if (cancelled) return;
        setActiveTools((prev) => [...prev, event.payload]);
      }).then((fn) => {
        if (cancelled) fn(); else toolStartUnlisten = fn;
      });

      mod.listen<string>("tool_execution_end", (event) => {
        if (cancelled) return;
        setActiveTools((prev) => prev.filter((t) => t !== event.payload));
      }).then((fn) => {
        if (cancelled) fn(); else toolEndUnlisten = fn;
      });

      mod.listen<void>("chat-stream-done", () => {
        if (cancelled) return;
        setActiveTools([]);
        setIsStreaming(false);
      }).then((fn) => {
        if (cancelled) fn(); else doneUnlisten = fn;
      });
    });

    return () => {
      cancelled = true;
      chunkUnlisten?.();
      doneUnlisten?.();
      toolStartUnlisten?.();
      toolEndUnlisten?.();
    };
  }, []);

  // ── Keyboard shortcuts for filter modals ─────────────────────
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Ctrl+P → File picker
      if ((e.ctrlKey || e.metaKey) && e.key === "p" && !e.shiftKey) {
        e.preventDefault();
        useChatStore.getState().openModal("files");
      }
      // Ctrl+Shift+O → Symbol picker
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "O") {
        e.preventDefault();
        useChatStore.getState().openModal("symbols");
      }
      // Ctrl+Shift+M → Module picker
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "M") {
        e.preventDefault();
        useChatStore.getState().openModal("modules");
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // ── Smart Ask mutation (uses plan executor for deep research) ─
  const messagesRef = useRef(messages);
  useEffect(() => { messagesRef.current = messages; }, [messages]);

  const askMutation = useMutation({
    mutationFn: async (question: string) => {
      // Build history from ref + the user message just added (ref may not be synced yet)
      const history = [
        ...messagesRef.current,
        { role: "user" as const, content: question },
      ].map((m) => ({
        role: m.role,
        content: m.content,
      }));

      const currentFilters = useChatStore.getState().filters;
      const isDeep = useChatStore.getState().deepResearchEnabled;
      const hasFilters = useChatStore.getState().hasActiveFilters();

      // Use the smart planner/executor for deep research or filtered queries
      if (isDeep || hasFilters) {
        return commands.chatExecutePlan({
          question,
          history,
          filters: hasFilters ? currentFilters : undefined,
          deepResearch: isDeep,
        });
      }

      // Standard chat — reset streaming state and start listening for chunks
      setStreamingText("");
      setIsStreaming(true);
      streamingMsgIdRef.current = `msg-${Date.now()}-stream`;

      const response = await commands.chatAsk({ question, history });
      return {
        answer: response.answer,
        sources: response.sources,
        model: response.model,
        complexity: "simple" as QueryComplexity,
      } as ChatSmartResponse;
    },
    onSuccess: (response) => {
      // Finalize: stop streaming, add the complete message
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

      const errorMessage: Message = {
        id: `msg-${Date.now()}`,
        role: "assistant",
        content: `**Error:** ${(error as Error).message}\n\nCould not get an AI response. Check your LLM configuration in Chat Settings (\u2699\uFE0F button), or use Ollama for local inference.`,
        timestamp: Date.now(),
      };
      addMessage(activeRepo, errorMessage);
    },
  });

  const handleSend = useCallback(() => {
    const question = input.trim();
    if (!question || askMutation.isPending) return;

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

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  // ─── Empty State ──────────────────────────────────────────────

  if (messages.length === 0) {
    return (
      <div className="h-full flex flex-col">
        {/* Context filter bar */}
        <ChatContextBar />

        {/* Suggestions */}
        <div className="flex-1 min-h-0 overflow-auto">
          <ChatSuggestions onSelect={(q) => { setInput(q); inputRef.current?.focus(); }} />
        </div>

        {/* Input bar */}
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
        />

        {/* Filter modals */}
        {renderFilterModals(activeModal, closeModal)}
      </div>
    );
  }

  // ─── Conversation View ────────────────────────────────────────

  return (
    <div className="h-full flex flex-col">
      {/* Context filter bar + action buttons */}
      <div className="flex items-center">
        <div className="flex-1">
          <ChatContextBar />
        </div>
        <div className="flex items-center gap-1 mr-2">
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
              URL.revokeObjectURL(url);
              toast.success("Chat exported as Markdown");
            }}
            className="text-[11px] hover-surface rounded px-2 py-1"
            style={{ color: "var(--text-3)" }}
            title="Export chat as Markdown"
            aria-label="Export chat"
          >
            <Download size={12} />
          </button>
          <button
            onClick={() => {
              if (!window.confirm(t("chat.confirmClear"))) return;
              clearSessionMessages(activeRepo);
              toast.success(t("chat.conversationCleared"));
            }}
            className="text-[11px] hover-surface rounded px-2 py-1"
            style={{ color: "var(--text-3)" }}
            title={t("chat.clearConversation") || "Clear conversation"}
            aria-label="Clear conversation"
          >
            <Trash2 size={12} />
          </button>
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto px-4 py-4 space-y-4" aria-live="polite" aria-label="Chat messages">
        {messages.map((msg) => (
          <MessageBubble
            key={msg.id}
            message={msg}
            onNavigateToNode={onNavigateToNode}
          />
        ))}

        {/* Active tool badges — shown during tool execution */}
        {activeTools.length > 0 && (
          <div className="fade-in flex flex-wrap gap-1.5 px-1 py-1">
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

        {/* Streaming response — show tokens as they arrive */}
        {isStreaming && streamingText && (
          <div className="fade-in">
            <div className="flex items-center gap-1.5 mb-1">
              <span
                className="w-2 h-2 rounded-full flex-shrink-0"
                style={{ background: "var(--purple)" }}
              />
              <span className="text-[11px] font-medium" style={{ color: "var(--text-3)" }}>
                GitNexus
              </span>
            </div>
            <div
              className="prose-sm text-[13px] leading-relaxed"
              style={{ color: "var(--text-1)" }}
              aria-live="assertive"
            >
              <Suspense fallback={<MarkdownFallback content={streamingText} />}>
                <ChatMarkdown content={streamingText} onNavigateToNode={onNavigateToNode} />
              </Suspense>
              <span className="typing-cursor" />
            </div>
          </div>
        )}

        {/* Thinking/loading shimmer — before any tokens arrive */}
        {askMutation.isPending && !isStreaming && (
          <div className="fade-in">
            <div className="flex items-center gap-1.5 mb-1">
              <span
                className="w-2 h-2 rounded-full flex-shrink-0"
                style={{ background: "var(--purple)" }}
              />
              <span className="text-[11px] font-medium" style={{ color: "var(--text-3)" }}>
                GitNexus
              </span>
              <span className="text-[11px]" style={{ color: "var(--text-3)" }}>
                {deepResearchEnabled
                  ? t("chat.executingResearch")
                  : hasActiveFilters()
                  ? t("chat.searchingContext")
                  : t("chat.thinking")}
              </span>
            </div>
            <div className="space-y-2 py-4 px-4">
              <div className="shimmer rounded" style={{ height: 14, width: "80%", background: "var(--bg-3)" }} />
              <div className="shimmer rounded" style={{ height: 14, width: "65%", background: "var(--bg-3)" }} />
              <div className="shimmer rounded" style={{ height: 14, width: "45%", background: "var(--bg-3)" }} />
            </div>
          </div>
        )}

        {/* Waiting for first token — streaming started but no text yet */}
        {isStreaming && !streamingText && (
          <div className="fade-in">
            <div className="flex items-center gap-1.5 mb-1">
              <span
                className="w-2 h-2 rounded-full flex-shrink-0"
                style={{ background: "var(--purple)" }}
              />
              <span className="text-[11px] font-medium" style={{ color: "var(--text-3)" }}>
                GitNexus
              </span>
              <span className="text-[11px]" style={{ color: "var(--text-3)" }}>
                {t("chat.generatingResponse")}
              </span>
            </div>
            <div className="py-2 px-4">
              <span className="typing-cursor" />
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input bar */}
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
      />

      {/* Filter modals */}
      {renderFilterModals(activeModal, closeModal)}
    </div>
  );
}

// ─── MessageBubble (flat developer-tool layout) ────────────────────

function MessageBubble({
  message,
  onNavigateToNode,
}: {
  message: Message;
  onNavigateToNode?: (nodeId: string) => void;
}) {
  const { t } = useI18n();
  const handleCopyMessage = useCallback(() => {
    navigator.clipboard.writeText(message.content).then(
      () => toast.success(t("chat.copiedToClipboard")),
      () => toast.error("Failed to copy"),
    );
  }, [message.content, t]);

  const handleExportMessage = useCallback(() => {
    try {
      const date = new Date().toISOString().split("T")[0];
      const filename = `gitnexus-response-${date}.md`;
      const blob = new Blob([message.content], { type: "text/markdown" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toast.success("Response exported successfully");
    } catch (e) {
      toast.error(`Failed to export: ${String(e)}`);
    }
  }, [message.content]);

  if (message.role === "user") {
    return (
      <div className="group relative fade-in">
        {/* Role label */}
        <div className="flex items-center gap-1.5 mb-1">
          <span
            className="w-2 h-2 rounded-full flex-shrink-0"
            style={{ background: "var(--accent)" }}
          />
          <span className="text-[11px] font-medium" style={{ color: "var(--text-3)" }}>
            {t("chat.you")}
          </span>
        </div>
        {/* Message content */}
        <div
          className="px-4 py-3 rounded-lg text-[13px] leading-relaxed"
          style={{ background: "var(--bg-2)", color: "var(--text-1)" }}
        >
          {message.content}
        </div>
        {/* Hover actions */}
        <div
          className="absolute top-0 right-0 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity"
          style={{ marginTop: -2 }}
        >
          <button
            onClick={handleCopyMessage}
            className="p-1 rounded transition-colors"
            style={{ background: "var(--bg-3)", color: "var(--text-3)" }}
            aria-label="Copy message"
          >
            <Copy size={12} />
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="group relative fade-in">
      {/* Role label */}
      <div className="flex items-center gap-1.5 mb-1">
        <span
          className="w-2 h-2 rounded-full flex-shrink-0"
          style={{ background: "var(--purple)" }}
        />
        <span className="text-[11px] font-medium" style={{ color: "var(--text-3)" }}>
          GitNexus
        </span>
        {/* Complexity badge inline */}
        {message.complexity && <ComplexityIndicator complexity={message.complexity} />}
      </div>

      {/* Hover actions */}
      <div
        className="absolute top-0 right-0 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity"
        style={{ marginTop: -2 }}
      >
        <button
          onClick={handleExportMessage}
          className="p-1 rounded transition-colors"
          style={{ background: "var(--bg-3)", color: "var(--text-3)" }}
          title="Export response as Markdown"
          aria-label="Export response"
        >
          <Download size={12} />
        </button>
        <button
          onClick={handleCopyMessage}
          className="p-1 rounded transition-colors"
          style={{ background: "var(--bg-3)", color: "var(--text-3)" }}
          aria-label="Copy message"
        >
          <Copy size={12} />
        </button>
      </div>

      {/* Research plan (if present) */}
      {message.plan && (
        <div className="mb-3">
          <Suspense fallback={null}>
            <ResearchPlanViewer plan={message.plan} />
          </Suspense>
        </div>
      )}

      {/* Response content */}
      <div
        className="prose-sm text-[13px] leading-relaxed"
        style={{ color: "var(--text-1)" }}
      >
        <Suspense fallback={<MarkdownFallback content={message.content} />}>
          <ChatMarkdown content={message.content} onNavigateToNode={onNavigateToNode} />
        </Suspense>
      </div>

      {/* Enhanced source references */}
      {message.sources && message.sources.length > 0 && (
        <Suspense fallback={null}>
          <SourceReferences
            sources={message.sources}
            onNavigateToNode={onNavigateToNode}
          />
        </Suspense>
      )}

      {/* Model indicator */}
      {message.model && (
        <div className="mt-2 text-[11px]" style={{ color: "var(--text-3)" }}>
          Answered by {message.model}
        </div>
      )}
    </div>
  );
}

// ─── ComplexityIndicator ────────────────────────────────────────────

function ComplexityIndicator({ complexity }: { complexity: QueryComplexity }) {
  const { t } = useI18n();
  const configs: Record<string, { label: string; color: string; icon: typeof Zap }> = {
    simple: { label: t("chat.quickAnswer"), color: "var(--green)", icon: Zap },
    medium: { label: t("chat.multiSource"), color: "var(--orange)", icon: Sparkles },
    complex: { label: t("chat.deepResearch"), color: "var(--purple)", icon: Microscope },
  };
  const config = configs[complexity] ?? configs.simple;

  const Icon = config.icon;

  return (
    <span
      className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium"
      style={{
        background: `color-mix(in srgb, ${config.color} 10%, transparent)`,
        color: config.color,
      }}
    >
      <Icon size={9} />
      {config.label}
    </span>
  );
}

// ─── ChatInput ──────────────────────────────────────────────────────

interface ChatInputProps {
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  onKeyDown: (e: React.KeyboardEvent) => void;
  isPending: boolean;
  onOpenSettings?: () => void;
  deepResearch: boolean;
  hasFilters: boolean;
}

const ChatInput = forwardRef<HTMLTextAreaElement, ChatInputProps>(
  ({ value, onChange, onSend, onKeyDown, isPending, onOpenSettings, deepResearch, hasFilters }, ref) => {
    const { t } = useI18n();
    const internalRef = useRef<HTMLTextAreaElement | null>(null);
    const placeholder = deepResearch
      ? "Ask a complex question (deep research mode)..."
      : hasFilters
      ? "Ask about filtered context..."
      : "Ask about this codebase...";

    // Auto-resize textarea on input change
    useEffect(() => {
      const el = internalRef.current;
      if (!el) return;
      el.style.height = "auto";
      el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
    }, [value]);

    // Merge forwarded ref with internal ref
    const setRefs = useCallback(
      (node: HTMLTextAreaElement | null) => {
        internalRef.current = node;
        if (typeof ref === "function") {
          ref(node);
        } else if (ref) {
          (ref as React.MutableRefObject<HTMLTextAreaElement | null>).current = node;
        }
      },
      [ref],
    );

    return (
      <div
        className="flex-shrink-0 px-4 py-3"
        style={{ borderTop: "1px solid var(--surface-border)" }}
      >
        <div
          className="chat-input-container flex items-end gap-2 rounded-xl px-3 py-2 transition-all"
          style={{
            background: "var(--surface)",
            border: deepResearch
              ? "1px solid var(--purple)"
              : "1px solid var(--surface-border)",
          }}
        >
          {/* Deep research indicator */}
          {deepResearch && (
            <Microscope
              size={14}
              className="mb-1 flex-shrink-0"
              style={{ color: "var(--purple)" }}
            />
          )}

          <textarea
            ref={setRefs}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder={placeholder}
            aria-label="Ask a question about the code"
            rows={1}
            className="flex-1 bg-transparent resize-none text-[13px] outline-none focus:ring-1 focus:ring-[var(--accent)] min-h-[24px] max-h-[200px]"
            style={{
              color: "var(--text-0)",
              fontFamily: "var(--font-body)",
            }}
          />
          <div className="flex items-center gap-1">
            {onOpenSettings && (
              <button
                onClick={onOpenSettings}
                className="p-1.5 rounded-lg transition-colors"
                style={{ color: "var(--text-3)" }}
                aria-label="Chat Settings"
              >
                <Settings2 size={14} />
              </button>
            )}
            <button
              onClick={onSend}
              disabled={!value.trim() || isPending}
              aria-label={isPending ? "Sending..." : "Send message"}
              className="p-1.5 rounded-lg transition-all"
              style={{
                background: value.trim() && !isPending
                  ? deepResearch ? "var(--purple)" : "var(--accent)"
                  : "transparent",
                color: value.trim() && !isPending ? "#fff" : "var(--text-3)",
              }}
            >
              {isPending ? (
                <Loader2 size={14} className="animate-spin" />
              ) : (
                <Send size={14} />
              )}
            </button>
          </div>
        </div>
        <p className="mt-1.5 text-[11px] text-center" style={{ color: "var(--text-3)" }}>
          {deepResearch
            ? t("chat.deepResearchHint")
            : t("chat.inputHint")}
        </p>
      </div>
    );
  }
);

ChatInput.displayName = "ChatInput";

