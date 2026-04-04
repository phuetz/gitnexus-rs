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

import { useState, useRef, useEffect, useCallback, forwardRef } from "react";
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
} from "lucide-react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { toast } from "sonner";
import { commands } from "../../lib/tauri-commands";
import { isTauri } from "../../lib/tauri-env";
import type {
  ChatSource as ChatSourceType,
  ChatSmartResponse,
  ResearchPlan,
  QueryComplexity,
} from "../../lib/tauri-commands";
import { useAppStore } from "../../stores/app-store";
import { useChatStore } from "../../stores/chat-store";
import { ChatContextBar } from "./ChatContextBar";
import { FileFilterModal } from "./FileFilterModal";
import { SymbolFilterModal } from "./SymbolFilterModal";
import { ModuleFilterModal } from "./ModuleFilterModal";
import { ResearchPlanViewer } from "./ResearchPlanViewer";
import { SourceReferences } from "./SourceReferences";

// ─── Types ──────────────────────────────────────────────────────────

interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  sources?: ChatSourceType[];
  model?: string | null;
  plan?: ResearchPlan;
  complexity?: QueryComplexity;
  timestamp: number;
}

// ─── Props ──────────────────────────────────────────────────────────

interface ChatPanelProps {
  onOpenSettings?: () => void;
  onNavigateToNode?: (nodeId: string) => void;
}

// ─── Component ──────────────────────────────────────────────────────

export function ChatPanel({ onOpenSettings, onNavigateToNode }: ChatPanelProps) {
  const { t } = useI18n();
  const activeRepo = useAppStore((s) => s.activeRepo);
  const storageKey = `gitnexus-chat-${activeRepo || "global"}`;

  const [messages, setMessages] = useState<Message[]>(() => {
    try {
      const saved = localStorage.getItem(storageKey);
      if (!saved) return [];
      const parsed = JSON.parse(saved);
      if (!Array.isArray(parsed)) return [];
      // Validate each message has required fields to prevent render crashes
      return parsed.filter(
        (m: unknown): m is Message =>
          typeof m === "object" &&
          m !== null &&
          typeof (m as Message).id === "string" &&
          typeof (m as Message).role === "string" &&
          typeof (m as Message).content === "string"
      );
    } catch {
      return [];
    }
  });
  const [input, setInput] = useState("");
  const [streamingText, setStreamingText] = useState("");
  const [isStreaming, setIsStreaming] = useState(false);
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

  // Persist messages to localStorage (keep last 100)
  useEffect(() => {
    try {
      const toSave = messages.slice(-100);
      localStorage.setItem(storageKey, JSON.stringify(toSave));
    } catch {
      // localStorage full or unavailable — silently ignore
    }
  }, [messages, storageKey]);

  // Reload messages when activeRepo changes
  useEffect(() => {
    try {
      const saved = localStorage.getItem(storageKey);
      if (!saved) { setMessages([]); return; }
      const parsed = JSON.parse(saved);
      if (!Array.isArray(parsed)) { setMessages([]); return; }
      setMessages(
        parsed.filter(
          (m: unknown): m is Message =>
            typeof m === "object" &&
            m !== null &&
            typeof (m as Message).id === "string" &&
            typeof (m as Message).role === "string" &&
            typeof (m as Message).content === "string"
        )
      );
    } catch {
      setMessages([]);
    }
  }, [storageKey]);

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamingText]);

  // ── Listen for SSE stream chunks from the backend ───────────
  useEffect(() => {
    if (!isTauri()) return;

    let cancelled = false;
    let chunkUnlisten: (() => void) | null = null;
    let doneUnlisten: (() => void) | null = null;

    import("@tauri-apps/api/event").then((mod) => {
      mod.listen<string>("chat-stream-chunk", (event) => {
        if (cancelled) return;
        setStreamingText((prev) => prev + event.payload);
      }).then((fn) => {
        if (cancelled) fn(); else chunkUnlisten = fn;
      });

      mod.listen<void>("chat-stream-done", () => {
        // Stream done event fires before the command returns;
        // cleanup is handled in the mutation's onSuccess/onSettled.
      }).then((fn) => {
        if (cancelled) fn(); else doneUnlisten = fn;
      });
    });

    return () => {
      cancelled = true;
      chunkUnlisten?.();
      doneUnlisten?.();
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
      // Use ref to get the latest messages (including the user message just added)
      const history = messagesRef.current.map((m) => ({
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
      setMessages((prev) => [...prev, assistantMessage]);
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
      setMessages((prev) => [...prev, errorMessage]);
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
    setMessages((prev) => [...prev, userMessage]);
    setInput("");

    askMutation.mutate(question);
  }, [input, askMutation]);

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
        <FileFilterModal open={activeModal === "files"} onClose={closeModal} />
        <SymbolFilterModal open={activeModal === "symbols"} onClose={closeModal} />
        <ModuleFilterModal open={activeModal === "modules"} onClose={closeModal} />
      </div>
    );
  }

  // ─── Conversation View ────────────────────────────────────────

  return (
    <div className="h-full flex flex-col">
      {/* Context filter bar + clear button */}
      <div className="flex items-center">
        <div className="flex-1">
          <ChatContextBar />
        </div>
        <button
          onClick={() => {
            setMessages([]);
            localStorage.removeItem(storageKey);
            toast.success(t("chat.conversationCleared"));
          }}
          className="text-xs hover-surface rounded px-2 py-1 mr-2"
          style={{ color: "var(--text-3)" }}
          aria-label="Clear conversation"
        >
          <Trash2 size={12} />
        </button>
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
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={markdownComponents}
              >
                {streamingText}
              </ReactMarkdown>
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
                generating response...
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
      <FileFilterModal open={activeModal === "files"} onClose={closeModal} />
      <SymbolFilterModal open={activeModal === "symbols"} onClose={closeModal} />
      <ModuleFilterModal open={activeModal === "modules"} onClose={closeModal} />
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
          <ResearchPlanViewer plan={message.plan} />
        </div>
      )}

      {/* Response content */}
      <div
        className="prose-sm text-[13px] leading-relaxed"
        style={{ color: "var(--text-1)" }}
      >
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={markdownComponents}
        >
          {message.content}
        </ReactMarkdown>
      </div>

      {/* Enhanced source references */}
      {message.sources && message.sources.length > 0 && (
        <SourceReferences
          sources={message.sources}
          onNavigateToNode={onNavigateToNode}
        />
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
            ? "Deep Research: multi-step analysis with plan execution. Ctrl+P for files."
            : "Powered by knowledge graph context. Enter to send, Shift+Enter for new line."}
        </p>
      </div>
    );
  }
);

ChatInput.displayName = "ChatInput";

// ─── Markdown Components ────────────────────────────────────────────

// ─── Helper: extract text from React children ─────────────────────

function extractTextFromChildren(children: React.ReactNode): string {
  if (typeof children === "string") return children;
  if (typeof children === "number") return String(children);
  if (!children) return "";
  if (Array.isArray(children)) return children.map(extractTextFromChildren).join("");
  if (typeof children === "object" && children !== null && "props" in children) {
    const el = children as React.ReactElement<{ children?: React.ReactNode }>;
    return extractTextFromChildren(el.props.children);
  }
  return "";
}

const markdownComponents: Partial<Components> = {
  pre: ({ children }: { children?: React.ReactNode }) => (
    <div className="relative group my-3">
      <pre
        className="p-4 rounded-lg overflow-x-auto text-[12px] leading-relaxed"
        style={{
          background: "var(--bg-0)",
          border: "1px solid var(--surface-border)",
          fontFamily: "var(--font-mono)",
          borderRadius: 8,
        }}
      >
        {children}
      </pre>
      <button
        className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity px-2 py-1 rounded text-xs"
        style={{ background: "var(--bg-3)", color: "var(--text-2)" }}
        onClick={() => {
          const text = extractTextFromChildren(children);
          navigator.clipboard.writeText(text).then(
            () => toast.success("Copied!"),
            () => toast.error("Failed to copy"),
          );
        }}
      >
        <Copy size={12} className="inline mr-1" />
        Copy
      </button>
    </div>
  ),
  code: ({ className, children }: { className?: string; children?: React.ReactNode }) => {
    if (className) {
      return <code className={className}>{children}</code>;
    }
    return (
      <code
        className="px-1 py-0.5 rounded text-[11px]"
        style={{
          background: "var(--bg-3)",
          color: "var(--accent)",
          fontFamily: "var(--font-mono)",
        }}
      >
        {children}
      </code>
    );
  },
  p: ({ children }: { children?: React.ReactNode }) => (
    <p className="mb-2 leading-relaxed">{children}</p>
  ),
  ul: ({ children }: { children?: React.ReactNode }) => (
    <ul className="mb-2 pl-4 space-y-0.5" style={{ listStyleType: "disc" }}>
      {children}
    </ul>
  ),
  ol: ({ children }: { children?: React.ReactNode }) => (
    <ol className="mb-2 pl-4 space-y-0.5" style={{ listStyleType: "decimal" }}>
      {children}
    </ol>
  ),
  strong: ({ children }: { children?: React.ReactNode }) => (
    <strong style={{ color: "var(--text-0)" }}>{children}</strong>
  ),
  a: ({ href, children }: { href?: string; children?: React.ReactNode }) => (
    <a href={href} style={{ color: "var(--accent)", textDecoration: "underline" }}>
      {children}
    </a>
  ),
  h3: ({ children }: { children?: React.ReactNode }) => (
    <h3 className="text-sm font-semibold mt-3 mb-1" style={{ color: "var(--text-0)" }}>
      {children}
    </h3>
  ),
};

