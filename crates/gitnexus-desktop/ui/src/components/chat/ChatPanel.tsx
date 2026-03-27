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
import {
  Send,
  MessageSquare,
  Loader2,
  Bot,
  User,
  Settings2,
  Sparkles,
  Microscope,
  Zap,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { commands } from "../../lib/tauri-commands";
import type {
  ChatSource as ChatSourceType,
  ChatSmartResponse,
  ResearchPlan,
  QueryComplexity,
} from "../../lib/tauri-commands";
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
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const {
    filters,
    deepResearchEnabled,
    activeModal,
    closeModal,
    hasActiveFilters,
    setActivePlan,
  } = useChatStore();

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

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
  const askMutation = useMutation({
    mutationFn: async (question: string) => {
      const history = messages.map((m) => ({
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

      // Standard chat for simple unfiltered queries
      const response = await commands.chatAsk({ question, history });
      return {
        answer: response.answer,
        sources: response.sources,
        model: response.model,
        complexity: "simple" as QueryComplexity,
      } as ChatSmartResponse;
    },
    onSuccess: (response) => {
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
      const errorMessage: Message = {
        id: `msg-${Date.now()}`,
        role: "assistant",
        content: `**Error:** ${(error as Error).message}\n\nMake sure an LLM provider is configured in Settings, or use Ollama for local inference.`,
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

        {/* Empty state */}
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center max-w-md px-4">
            <div
              className="w-14 h-14 rounded-2xl flex items-center justify-center mx-auto mb-5"
              style={{ background: "var(--purple-subtle)", color: "var(--purple)" }}
            >
              <MessageSquare size={24} />
            </div>
            <h3
              className="text-lg mb-2"
              style={{ fontFamily: "var(--font-display)", color: "var(--text-0)" }}
            >
              Ask about this codebase
            </h3>
            <p className="text-sm mb-4" style={{ color: "var(--text-2)" }}>
              Ask questions in natural language. Use the filters above to scope your search,
              or enable <strong>Deep Research</strong> for multi-step analysis.
            </p>

            {/* Keyboard shortcuts hint */}
            <div className="flex justify-center gap-4 mb-6 text-[11px]" style={{ color: "var(--text-3)" }}>
              <span>
                <kbd className="px-1 rounded" style={{ background: "var(--bg-3)" }}>Ctrl+P</kbd> Files
              </span>
              <span>
                <kbd className="px-1 rounded" style={{ background: "var(--bg-3)" }}>Ctrl+Shift+O</kbd> Symbols
              </span>
              <span>
                <kbd className="px-1 rounded" style={{ background: "var(--bg-3)" }}>Ctrl+Shift+M</kbd> Modules
              </span>
            </div>

            {/* Suggested questions */}
            <div className="space-y-2">
              {SUGGESTED_QUESTIONS.map((q, i) => (
                <button
                  key={i}
                  onClick={() => {
                    setInput(q);
                    inputRef.current?.focus();
                  }}
                  className="w-full text-left px-3 py-2 rounded-lg text-[13px] transition-all"
                  style={{
                    background: "var(--surface)",
                    color: "var(--text-1)",
                    border: "1px solid var(--surface-border)",
                  }}
                >
                  <Sparkles size={12} className="inline mr-2 opacity-50" />
                  {q}
                </button>
              ))}
            </div>
          </div>
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
      {/* Context filter bar */}
      <ChatContextBar />

      {/* Messages */}
      <div className="flex-1 overflow-y-auto px-4 py-4 space-y-4">
        {messages.map((msg) => (
          <MessageBubble
            key={msg.id}
            message={msg}
            onNavigateToNode={onNavigateToNode}
          />
        ))}

        {/* Loading indicator */}
        {askMutation.isPending && (
          <div className="flex items-start gap-3 fade-in">
            <div
              className="w-7 h-7 rounded-lg flex items-center justify-center flex-shrink-0 mt-0.5"
              style={{ background: "var(--purple-subtle)", color: "var(--purple)" }}
            >
              <Bot size={14} />
            </div>
            <div className="flex items-center gap-2 py-2" style={{ color: "var(--text-3)" }}>
              <Loader2 size={14} className="animate-spin" />
              <span className="text-[13px]">
                {deepResearchEnabled
                  ? "Executing research plan..."
                  : hasActiveFilters()
                  ? "Searching filtered context..."
                  : "Searching knowledge graph & generating answer..."}
              </span>
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

// ─── MessageBubble ──────────────────────────────────────────────────

function MessageBubble({
  message,
  onNavigateToNode,
}: {
  message: Message;
  onNavigateToNode?: (nodeId: string) => void;
}) {
  if (message.role === "user") {
    return (
      <div className="flex items-start gap-3 justify-end">
        <div
          className="max-w-[80%] px-4 py-2.5 rounded-2xl rounded-br-md text-[13px]"
          style={{
            background: "var(--accent)",
            color: "#fff",
          }}
        >
          {message.content}
        </div>
        <div
          className="w-7 h-7 rounded-lg flex items-center justify-center flex-shrink-0 mt-0.5"
          style={{ background: "var(--bg-3)", color: "var(--text-2)" }}
        >
          <User size={14} />
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-start gap-3 fade-in">
      <div
        className="w-7 h-7 rounded-lg flex items-center justify-center flex-shrink-0 mt-0.5"
        style={{ background: "var(--purple-subtle)", color: "var(--purple)" }}
      >
        <Bot size={14} />
      </div>
      <div className="flex-1 min-w-0">
        {/* Complexity badge */}
        {message.complexity && (
          <div className="flex items-center gap-1.5 mb-1.5">
            <ComplexityIndicator complexity={message.complexity} />
          </div>
        )}

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
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            components={markdownComponents as any}
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
    </div>
  );
}

// ─── ComplexityIndicator ────────────────────────────────────────────

function ComplexityIndicator({ complexity }: { complexity: QueryComplexity }) {
  const config = {
    simple: { label: "Quick answer", color: "var(--green)", icon: Zap },
    medium: { label: "Multi-source", color: "var(--orange)", icon: Sparkles },
    complex: { label: "Deep research", color: "var(--purple)", icon: Microscope },
  }[complexity];

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
    const placeholder = deepResearch
      ? "Ask a complex question (deep research mode)..."
      : hasFilters
      ? "Ask about filtered context..."
      : "Ask about this codebase...";

    return (
      <div
        className="flex-shrink-0 px-4 py-3"
        style={{ borderTop: "1px solid var(--surface-border)" }}
      >
        <div
          className="flex items-end gap-2 rounded-xl px-3 py-2"
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
            ref={ref}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder={placeholder}
            rows={1}
            className="flex-1 bg-transparent resize-none text-[13px] outline-none min-h-[24px] max-h-[120px]"
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
                title="Chat Settings"
              >
                <Settings2 size={14} />
              </button>
            )}
            <button
              onClick={onSend}
              disabled={!value.trim() || isPending}
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

const markdownComponents = {
  pre: ({ children }: { children: React.ReactNode }) => (
    <pre
      className="my-3 p-3 rounded-lg overflow-x-auto text-[12px] leading-relaxed"
      style={{
        background: "var(--bg-1)",
        border: "1px solid var(--surface-border)",
        fontFamily: "var(--font-mono)",
      }}
    >
      {children}
    </pre>
  ),
  code: ({ className, children }: { className?: string; children: React.ReactNode }) => {
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
  p: ({ children }: { children: React.ReactNode }) => (
    <p className="mb-2 leading-relaxed">{children}</p>
  ),
  ul: ({ children }: { children: React.ReactNode }) => (
    <ul className="mb-2 pl-4 space-y-0.5" style={{ listStyleType: "disc" }}>
      {children}
    </ul>
  ),
  ol: ({ children }: { children: React.ReactNode }) => (
    <ol className="mb-2 pl-4 space-y-0.5" style={{ listStyleType: "decimal" }}>
      {children}
    </ol>
  ),
  strong: ({ children }: { children: React.ReactNode }) => (
    <strong style={{ color: "var(--text-0)" }}>{children}</strong>
  ),
  a: ({ href, children }: { href?: string; children: React.ReactNode }) => (
    <a href={href} style={{ color: "var(--accent)", textDecoration: "underline" }}>
      {children}
    </a>
  ),
  h3: ({ children }: { children: React.ReactNode }) => (
    <h3 className="text-sm font-semibold mt-3 mb-1" style={{ color: "var(--text-0)" }}>
      {children}
    </h3>
  ),
};

// ─── Suggested Questions ────────────────────────────────────────────

const SUGGESTED_QUESTIONS = [
  "What is the high-level architecture of this project?",
  "What are the main entry points?",
  "How do the modules depend on each other?",
  "What are the key data structures?",
];
