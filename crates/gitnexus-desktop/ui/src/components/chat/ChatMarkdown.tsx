import { Copy, GitBranch, RefreshCw, ChevronDown, ChevronRight, Pin, PinOff, Loader2, CheckCircle2, XCircle, Clock, Lightbulb, AlertTriangle, Info, AlertCircle } from "lucide-react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { toast } from "sonner";
import { useEffect, useRef, useState, useId, useMemo } from "react";
import { useAppStore } from "../../stores/app-store";
import { commands } from "../../lib/tauri-commands";
import type { Message, ToolCall, ToolCallStatus } from "../../stores/chat-session-store";
import { useChatSessionStore } from "../../stores/chat-session-store";
import { useShikiHighlighter } from "../../hooks/use-shiki-highlighter";

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

// ─── Smart Links Integration ─────────────────────────────────────────

function SmartInlineCode({ 
  text, 
  onNavigateToNode, 
  onFilePreview,
  children 
}: { 
  text: string; 
  onNavigateToNode?: (id: string) => void; 
  onFilePreview?: (file: { path: string; startLine?: number; endLine?: number }) => void;
  children: React.ReactNode 
}) {
  const [isHovered, setIsHovered] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const isFilePath = useMemo(() => {
    return /^[a-zA-Z0-9_/.-]+\.[a-z0-9]+$/.test(text) || text.includes('/') || text.includes('\\');
  }, [text]);

  const handleClick = async () => {
    if (isFilePath && onFilePreview) {
      onFilePreview({ path: text });
      return;
    }

    if (!onNavigateToNode || !text) return;
    if (text.length < 3) return; // avoid searching for very short generic terms
    
    setIsLoading(true);
    try {
      const results = await commands.searchSymbols(text, 1);
      if (results && results.length > 0) {
        onNavigateToNode(results[0].nodeId);
      } else {
        toast.info(`Symbol '${text}' not found in the graph.`);
      }
    } catch (e) {
      toast.error(`Search failed: ${String(e)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const isInteractive = !!onNavigateToNode || (isFilePath && !!onFilePreview);

  return (
    <code
      onClick={handleClick}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className="px-1.5 py-0.5 rounded text-[11px] transition-all"
      style={{
        background: isHovered && isInteractive ? "var(--accent-subtle)" : "var(--bg-3)",
        color: isHovered && isInteractive ? "var(--accent)" : "var(--accent)",
        fontFamily: "var(--font-mono)",
        cursor: isInteractive ? "pointer" : "default",
        borderBottom: isHovered && isInteractive ? (isFilePath ? "1px solid var(--accent)" : "1px dashed var(--accent)") : "1px solid transparent",
        opacity: isLoading ? 0.7 : 1,
      }}
      title={isInteractive ? (isFilePath ? `Preview file '${text}'` : `Locate '${text}' in graph`) : undefined}
    >
      {children}
    </code>
  );
}

// ─── Mermaid Integration ─────────────────────────────────────────────

let mermaidPromise: Promise<typeof import("mermaid")["default"]> | null = null;

async function loadMermaid() {
  if (!mermaidPromise) {
    mermaidPromise = import("mermaid").then((m) => m.default);
  }
  return mermaidPromise;
}

async function initMermaid(isDark: boolean) {
  const mermaid = await loadMermaid();
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: "strict",
    theme: isDark ? "dark" : "default",
    themeVariables: isDark
      ? {
          darkMode: true,
          background: "#0e1118",
          primaryColor: "#1c2233",
          primaryTextColor: "#e2e8f0",
          primaryBorderColor: "#5b9cf6",
          lineColor: "#5b9cf6",
          textColor: "#c1cad8",
          mainBkg: "#1c2233",
          nodeBorder: "#5b9cf6",
        }
      : {
          darkMode: false,
          background: "#f0f2f7",
          primaryColor: "#dde1eb",
          primaryTextColor: "#1a1d26",
          primaryBorderColor: "#4a85e0",
          lineColor: "#4a85e0",
          textColor: "#2e3341",
          mainBkg: "#dde1eb",
          nodeBorder: "#4a85e0",
        },
    flowchart: {
      htmlLabels: true,
      curve: "basis",
      padding: 12,
    },
    fontFamily: "'DM Sans', system-ui, sans-serif",
    fontSize: 13,
  });
}

function MermaidDiagram({ chart }: { chart: string }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [svg, setSvg] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const reactId = useId();
  const mermaidId = `mermaid-chat-${reactId.replace(/:/g, "")}`;
  const theme = useAppStore((s) => s.theme);
  
  // Resolve system theme if needed
  const isDark = theme === "dark" || (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);

  useEffect(() => {
    let cancelled = false;

    async function render() {
      try {
        const mermaid = await loadMermaid();
        await initMermaid(isDark);
        const { svg: rendered } = await mermaid.render(mermaidId, chart.trim());
        if (!cancelled) {
          setSvg(rendered);
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          setError((err as Error).message || "Failed to render diagram");
          const errEl = document.getElementById("d" + mermaidId);
          if (errEl) errEl.remove();
        }
      }
    }

    render();
    return () => { cancelled = true; };
  }, [chart, mermaidId, isDark]);

  // Render via DOM ref instead of dangerouslySetInnerHTML.
  // Mermaid's securityLevel:"strict" already strips event handlers;
  // we add defence-in-depth by removing <script> elements.
  useEffect(() => {
    const el = containerRef.current;
    if (!el || !svg) return;
    el.textContent = "";
    const wrapper = document.createElement("div");
    wrapper.textContent = "";
    const parser = new DOMParser();
    const doc = parser.parseFromString(svg, "image/svg+xml");
    doc.querySelectorAll("script,iframe,object,embed").forEach((n) => n.remove());
    const svgEl = doc.documentElement;
    if (svgEl) el.appendChild(document.importNode(svgEl, true));
  }, [svg]);

  if (error) {
    return (
      <div
        className="my-3 p-3 rounded-lg text-[11px] overflow-x-auto"
        style={{ background: "var(--rose-subtle)", color: "var(--rose)", border: "1px solid var(--rose)" }}
      >
        <p className="font-medium mb-1">Failed to render Mermaid diagram</p>
        <pre className="whitespace-pre-wrap">{error}</pre>
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      className="my-4 flex justify-center overflow-x-auto rounded-lg p-4"
      style={{
        background: "var(--bg-1)",
        border: "1px solid var(--surface-border)",
      }}
    />
  );
}

// ─── Callout detection ───────────────────────────────────────────────

type CalloutType = "tip" | "warning" | "note" | "danger";
const CALLOUT_MAP: Record<string, { type: CalloutType; label: string }> = {
  "[!TIP]":       { type: "tip",     label: "Tip" },
  "[!NOTE]":      { type: "note",    label: "Note" },
  "[!INFO]":      { type: "note",    label: "Info" },
  "[!WARNING]":   { type: "warning", label: "Warning" },
  "[!CAUTION]":   { type: "warning", label: "Caution" },
  "[!DANGER]":    { type: "danger",  label: "Danger" },
  "[!IMPORTANT]": { type: "danger",  label: "Important" },
};
const CALLOUT_STYLES: Record<CalloutType, { bg: string; border: string }> = {
  tip:     { bg: "rgba(34,197,94,0.08)",  border: "rgba(34,197,94,0.5)"  },
  note:    { bg: "rgba(99,179,237,0.08)", border: "rgba(99,179,237,0.5)" },
  warning: { bg: "rgba(251,191,36,0.08)", border: "rgba(251,191,36,0.5)" },
  danger:  { bg: "rgba(239,68,68,0.08)",  border: "rgba(239,68,68,0.5)"  },
};
const CALLOUT_ICONS: Record<CalloutType, React.ReactNode> = {
  tip:     <Lightbulb size={13} />,
  note:    <Info size={13} />,
  warning: <AlertTriangle size={13} />,
  danger:  <AlertCircle size={13} />,
};

function detectCallout(children: React.ReactNode): (typeof CALLOUT_MAP)[string] | null {
  const text = extractTextFromChildren(children).trimStart();
  for (const [marker, meta] of Object.entries(CALLOUT_MAP)) {
    if (text.startsWith(marker)) return meta;
  }
  return null;
}

// ─── Shiki code block (token-based, no innerHTML) ────────────────────

function ShikiTokens({ code, langHint }: { code: string; langHint: string }) {
  const { tokenize, ready } = useShikiHighlighter();
  const [lines, setLines] = useState<ReturnType<typeof tokenize>>(null);

  useEffect(() => {
    if (ready) setLines(tokenize(code, langHint));
  }, [code, langHint, tokenize, ready]);

  if (!lines) return null;

  return (
    <code style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}>
      {lines.map((line, li) => (
        <span key={li} style={{ display: "block" }}>
          {line.tokens.map((tok, ti) => (
            <span key={ti} style={{ color: tok.color, fontStyle: tok.fontStyle === 2 ? "italic" : undefined }}>
              {tok.content}
            </span>
          ))}
        </span>
      ))}
    </code>
  );
}

function ShikiCodeBlock({ code, langHint, rawChildren }: { code: string; langHint: string; rawChildren: React.ReactNode }) {
  return (
    <div className="relative group my-3">
      <pre
        className="p-4 rounded-lg overflow-x-auto text-[12px] leading-relaxed"
        style={{ background: "#1a1b26", border: "1px solid var(--surface-border)", fontFamily: "var(--font-mono)" }}
      >
        <ShikiTokens code={code} langHint={langHint} />
        {/* Fallback plain text (hidden once tokens render) */}
        <span style={{ display: "none" }}>{rawChildren}</span>
      </pre>
      <button
        className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity px-2 py-1 rounded text-[11px]"
        style={{ background: "var(--bg-3)", color: "var(--text-2)" }}
        onClick={() => { navigator.clipboard.writeText(code).then(() => toast.success("Copied!"), () => toast.error("Failed")); }}
      >
        <Copy size={12} className="inline mr-1" />Copy
      </button>
    </div>
  );
}

// ─── Markdown Components ─────────────────────────────────────────────

const createMarkdownComponents = (
  onNavigateToNode?: (id: string) => void,
  onFilePreview?: (file: { path: string; startLine?: number; endLine?: number }) => void,
): Partial<Components> => ({
  pre: ({ children }: { children?: React.ReactNode }) => {
    const child = children as React.ReactElement<{ className?: string; children?: React.ReactNode }>;
    const className = child?.props?.className ?? "";
    if (className === "language-mermaid") {
      const code = String(child.props.children ?? "").replace(/\n$/, "");
      return <MermaidDiagram chart={code} />;
    }
    const langHint = className.replace("language-", "") || "text";
    const code = extractTextFromChildren(children);
    return <ShikiCodeBlock code={code} langHint={langHint} rawChildren={children} />;
  },
  code: ({ className, children }: { className?: string; children?: React.ReactNode }) => {
    if (className && className !== "language-mermaid") {
      return <code className={className}>{children}</code>;
    }
    if (className === "language-mermaid") {
      return null; // Handled by pre
    }
    const text = extractTextFromChildren(children);
    return (
      <SmartInlineCode 
        text={text} 
        onNavigateToNode={onNavigateToNode}
        onFilePreview={onFilePreview}
      >
        {children}
      </SmartInlineCode>
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
  a: ({ href, children }: { href?: string; children?: React.ReactNode }) => {
    const safe = href && !/^javascript:/i.test(href) ? href : undefined;
    return (
      <a href={safe} target="_blank" rel="noopener noreferrer" style={{ color: "var(--accent)", textDecoration: "underline" }}>
        {children}
      </a>
    );
  },
  h1: ({ children }: { children?: React.ReactNode }) => (
    <h1 className="text-lg font-bold mt-4 mb-2 pb-1" style={{ color: "var(--text-0)", borderBottom: "1px solid var(--surface-border)" }}>{children}</h1>
  ),
  h2: ({ children }: { children?: React.ReactNode }) => (
    <h2 className="text-base font-semibold mt-4 mb-2 pb-1" style={{ color: "var(--text-0)", borderBottom: "1px solid var(--surface-border)" }}>{children}</h2>
  ),
  h3: ({ children }: { children?: React.ReactNode }) => (
    <h3 className="text-sm font-semibold mt-3 mb-1" style={{ color: "var(--text-0)" }}>{children}</h3>
  ),
  h4: ({ children }: { children?: React.ReactNode }) => (
    <h4 className="text-sm font-medium mt-2 mb-1" style={{ color: "var(--text-1)" }}>{children}</h4>
  ),
  hr: () => <hr className="my-3" style={{ borderColor: "var(--surface-border)" }} />,
  blockquote: ({ children }: { children?: React.ReactNode }) => {
    const meta = detectCallout(children);
    if (meta) {
      const cs = CALLOUT_STYLES[meta.type];
      return (
        <div className="my-2 px-3 py-2 rounded-lg flex gap-2 text-sm" style={{ background: cs.bg, borderLeft: `3px solid ${cs.border}` }}>
          <span className="mt-0.5 flex-shrink-0" style={{ color: cs.border }}>{CALLOUT_ICONS[meta.type]}</span>
          <div>
            <span className="font-semibold text-xs uppercase tracking-wide mr-2" style={{ color: cs.border }}>{meta.label}</span>
            {children}
          </div>
        </div>
      );
    }
    return (
      <blockquote className="my-2 pl-3 text-sm italic" style={{ borderLeft: "3px solid var(--accent)", color: "var(--text-2)" }}>
        {children}
      </blockquote>
    );
  },
  table: ({ children }: { children?: React.ReactNode }) => (
    <div className="my-3 overflow-x-auto rounded-lg" style={{ border: "1px solid var(--surface-border)" }}>
      <table className="w-full text-sm border-collapse">{children}</table>
    </div>
  ),
  thead: ({ children }: { children?: React.ReactNode }) => <thead style={{ background: "var(--bg-1)" }}>{children}</thead>,
  tbody: ({ children }: { children?: React.ReactNode }) => <tbody>{children}</tbody>,
  tr: ({ children }: { children?: React.ReactNode }) => (
    <tr style={{ borderBottom: "1px solid var(--surface-border)" }}>{children}</tr>
  ),
  th: ({ children }: { children?: React.ReactNode }) => (
    <th className="px-3 py-2 text-left text-xs font-semibold uppercase tracking-wide" style={{ color: "var(--text-1)" }}>{children}</th>
  ),
  td: ({ children }: { children?: React.ReactNode }) => (
    <td className="px-3 py-2" style={{ color: "var(--text-0)" }}>{children}</td>
  ),
});

export interface ChatMarkdownProps {
  content: string;
  onNavigateToNode?: (id: string) => void;
  onFilePreview?: (file: { path: string; startLine?: number; endLine?: number }) => void;
  /** Theme B — when supplied, enables fork/pin buttons + tool-call renderer. */
  message?: Message;
  /** Theme B — parent session id; required to fork from a message. */
  sessionId?: string;
}

export function ChatMarkdown({
  content,
  onNavigateToNode,
  onFilePreview,
  message,
  sessionId,
}: ChatMarkdownProps) {
  const components = useMemo(() => createMarkdownComponents(onNavigateToNode, onFilePreview), [onNavigateToNode, onFilePreview]);
  return (
    <div>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {content}
      </ReactMarkdown>
      {message && (message.toolCalls?.length ?? 0) > 0 && (
        <ToolCallList
          toolCalls={message.toolCalls ?? []}
          messageId={message.id}
          sessionId={sessionId}
        />
      )}
      {message && sessionId && <MessageActions message={message} sessionId={sessionId} />}
    </div>
  );
}

// ─── Theme B: message-level actions (fork / pin) ────────────────────

function MessageActions({ message, sessionId }: { message: Message; sessionId: string }) {
  const forkSession = useChatSessionStore((s) => s.forkSession);
  const pinMessage = useChatSessionStore((s) => s.pinMessage);
  const pinned = !!message.pinned;

  const handleFork = () => {
    const newId = forkSession(sessionId, message.id);
    if (newId) {
      toast.success("Forked — new chat created from this message.");
    } else {
      toast.error("Could not fork from this message.");
    }
  };

  const handlePin = () => {
    pinMessage(sessionId, message.id);
    toast.success(pinned ? "Unpinned" : "Pinned");
  };

  return (
    <div
      className="mt-2 flex items-center gap-1 opacity-0 hover:opacity-100 focus-within:opacity-100 transition-opacity"
      style={{ fontSize: 11, color: "var(--text-3)" }}
    >
      <button
        onClick={handleFork}
        className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded"
        style={{ border: "1px solid var(--surface-border)", background: "var(--bg-1)", cursor: "pointer" }}
        title="Fork a new chat from this message"
      >
        <GitBranch size={10} /> Fork from here
      </button>
      <button
        onClick={handlePin}
        className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded"
        style={{ border: "1px solid var(--surface-border)", background: pinned ? "var(--accent-subtle)" : "var(--bg-1)", cursor: "pointer", color: pinned ? "var(--accent)" : "var(--text-3)" }}
        title={pinned ? "Unpin this message" : "Pin this message"}
      >
        {pinned ? <PinOff size={10} /> : <Pin size={10} />} {pinned ? "Pinned" : "Pin"}
      </button>
    </div>
  );
}

// ─── Theme B: tool-call renderer + retry ────────────────────────────

function statusBadge(status: ToolCallStatus) {
  const common = { display: "inline-flex", alignItems: "center", gap: 4, fontSize: 10, padding: "1px 6px", borderRadius: 999 } as const;
  switch (status) {
    case "pending":
      return <span style={{ ...common, background: "var(--bg-3)", color: "var(--text-3)" }}><Clock size={10} /> pending</span>;
    case "running":
      return <span style={{ ...common, background: "color-mix(in srgb, var(--orange) 15%, transparent)", color: "var(--orange)" }}><Loader2 size={10} className="animate-spin" /> running</span>;
    case "success":
      return <span style={{ ...common, background: "color-mix(in srgb, var(--green) 15%, transparent)", color: "var(--green)" }}><CheckCircle2 size={10} /> success</span>;
    case "error":
      return <span style={{ ...common, background: "color-mix(in srgb, var(--rose, #f7768e) 15%, transparent)", color: "var(--rose, #f7768e)" }}><XCircle size={10} /> error</span>;
  }
}

function ToolCallList({
  toolCalls,
  messageId,
  sessionId,
}: {
  toolCalls: ToolCall[];
  messageId: string;
  sessionId?: string;
}) {
  if (!toolCalls || toolCalls.length === 0) return null;
  return (
    <div className="mt-3 space-y-2">
      {toolCalls.map((tc) => (
        <ToolCallBlock key={tc.id} toolCall={tc} messageId={messageId} sessionId={sessionId} />
      ))}
    </div>
  );
}

function ToolCallBlock({
  toolCall,
  messageId,
  sessionId,
}: {
  toolCall: ToolCall;
  messageId: string;
  sessionId?: string;
}) {
  const [open, setOpen] = useState(false);
  const [editingArgs, setEditingArgs] = useState<string | null>(null);
  const [isRetrying, setIsRetrying] = useState(false);
  const updateToolCall = useChatSessionStore((s) => s.updateToolCall);

  const handleRetry = async () => {
    if (!sessionId) {
      toast.error("Cannot retry: missing session id.");
      return;
    }
    const newArgs = editingArgs ?? toolCall.args;
    // Validate JSON client-side so the user gets a quick signal before the
    // backend round-trip.
    try {
      JSON.parse(newArgs);
    } catch (e) {
      toast.error(`Invalid JSON: ${(e as Error).message}`);
      return;
    }

    setIsRetrying(true);
    updateToolCall(sessionId, messageId, toolCall.id, {
      status: "running",
      invokedAt: Date.now(),
    });

    try {
      const result = await commands.chatRetryTool({
        sessionId,
        messageId,
        toolCallId: toolCall.id,
        name: toolCall.name,
        newArgs: editingArgs ?? undefined,
        priorArgs: toolCall.args,
      });
      updateToolCall(sessionId, messageId, toolCall.id, {
        args: result.args,
        result: result.result,
        durationMs: result.durationMs,
        status: result.status === "error" ? "error" : "success",
        error: result.status === "error" ? result.result : undefined,
      });
      setEditingArgs(null);
      toast.success(`Tool '${toolCall.name}' re-executed (${result.durationMs} ms).`);
    } catch (e) {
      updateToolCall(sessionId, messageId, toolCall.id, {
        status: "error",
        error: (e as Error).message,
      });
      toast.error(`Retry failed: ${(e as Error).message}`);
    } finally {
      setIsRetrying(false);
    }
  };

  return (
    <div
      style={{
        border: "1px solid var(--surface-border)",
        borderRadius: 8,
        background: "var(--bg-1)",
      }}
    >
      <button
        onClick={() => setOpen((o) => !o)}
        style={{
          width: "100%",
          padding: "6px 10px",
          background: "transparent",
          border: "none",
          display: "flex",
          alignItems: "center",
          gap: 8,
          cursor: "pointer",
          color: "var(--text-1)",
        }}
      >
        {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        <code
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--accent)",
            background: "var(--bg-3)",
            padding: "1px 6px",
            borderRadius: 4,
          }}
        >
          {toolCall.name}
        </code>
        {statusBadge(toolCall.status)}
        {typeof toolCall.durationMs === "number" && (
          <span style={{ fontSize: 10, color: "var(--text-3)" }}>{toolCall.durationMs} ms</span>
        )}
        <span style={{ marginLeft: "auto", fontSize: 10, color: "var(--text-3)" }}>
          {open ? "hide" : "details"}
        </span>
      </button>

      {open && (
        <div style={{ padding: "0 10px 10px 10px", display: "flex", flexDirection: "column", gap: 8 }}>
          <div>
            <div style={{ fontSize: 10, color: "var(--text-3)", marginBottom: 3 }}>Arguments (JSON)</div>
            <textarea
              value={editingArgs ?? toolCall.args}
              onChange={(e) => setEditingArgs(e.target.value)}
              rows={Math.min(10, Math.max(3, (toolCall.args.match(/\n/g)?.length ?? 0) + 2))}
              style={{
                width: "100%",
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                padding: 6,
                background: "var(--bg-0)",
                border: "1px solid var(--surface-border)",
                borderRadius: 4,
                color: "var(--text-1)",
                resize: "vertical",
              }}
            />
          </div>

          {toolCall.result && (
            <div>
              <div style={{ fontSize: 10, color: "var(--text-3)", marginBottom: 3 }}>Result</div>
              <pre
                style={{
                  fontSize: 11,
                  margin: 0,
                  padding: 6,
                  background: "var(--bg-0)",
                  border: "1px solid var(--surface-border)",
                  borderRadius: 4,
                  color: "var(--text-2)",
                  maxHeight: 240,
                  overflow: "auto",
                  whiteSpace: "pre-wrap",
                }}
              >
                {toolCall.result}
              </pre>
            </div>
          )}

          {toolCall.error && !toolCall.result && (
            <div style={{ fontSize: 11, color: "var(--rose, #f7768e)" }}>
              {toolCall.error}
            </div>
          )}

          <div style={{ display: "flex", gap: 6 }}>
            <button
              onClick={handleRetry}
              disabled={isRetrying || !sessionId}
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
                fontSize: 11,
                padding: "3px 8px",
                background: "var(--accent)",
                color: "#fff",
                border: "none",
                borderRadius: 4,
                cursor: isRetrying ? "not-allowed" : "pointer",
                opacity: isRetrying ? 0.7 : 1,
              }}
              title="Re-execute with the current arguments"
            >
              {isRetrying ? <Loader2 size={11} className="animate-spin" /> : <RefreshCw size={11} />}
              Retry
            </button>
            {editingArgs !== null && editingArgs !== toolCall.args && (
              <button
                onClick={() => setEditingArgs(null)}
                style={{
                  fontSize: 11,
                  padding: "3px 8px",
                  background: "transparent",
                  color: "var(--text-3)",
                  border: "1px solid var(--surface-border)",
                  borderRadius: 4,
                  cursor: "pointer",
                }}
              >
                Reset args
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}