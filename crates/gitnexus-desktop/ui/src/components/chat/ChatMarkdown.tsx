import { Copy } from "lucide-react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { toast } from "sonner";
import { useEffect, useRef, useState, useId, useMemo } from "react";
import { useAppStore } from "../../stores/app-store";
import { commands } from "../../lib/tauri-commands";

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

function SmartInlineCode({ text, onNavigateToNode, children }: { text: string; onNavigateToNode?: (id: string) => void; children: React.ReactNode }) {
  const [isHovered, setIsHovered] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const handleClick = async () => {
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

  const isInteractive = !!onNavigateToNode;

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
        borderBottom: isHovered && isInteractive ? "1px dashed var(--accent)" : "1px solid transparent",
        opacity: isLoading ? 0.7 : 1,
      }}
      title={isInteractive ? `Locate '${text}' in graph` : undefined}
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

// ─── Markdown Components ─────────────────────────────────────────────

const createMarkdownComponents = (onNavigateToNode?: (id: string) => void): Partial<Components> => ({
  pre: ({ children }: { children?: React.ReactNode }) => {
    // Intercept mermaid code blocks
    const child = children as React.ReactElement<{ className?: string; children?: React.ReactNode }>;
    if (child?.props?.className === "language-mermaid") {
      const code = String(child.props.children ?? "").replace(/\n$/, "");
      return <MermaidDiagram chart={code} />;
    }

    return (
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
          className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity px-2 py-1 rounded text-[11px]"
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
    );
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
      <SmartInlineCode text={text} onNavigateToNode={onNavigateToNode}>
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
});

export function ChatMarkdown({ content, onNavigateToNode }: { content: string; onNavigateToNode?: (id: string) => void }) {
  const components = useMemo(() => createMarkdownComponents(onNavigateToNode), [onNavigateToNode]);
  return (
    <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
      {content}
    </ReactMarkdown>
  );
}