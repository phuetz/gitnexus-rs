/**
 * DocsContent — Markdown renderer with Mermaid diagram support.
 *
 * Renders documentation pages as rich HTML from Markdown source,
 * with automatic Mermaid diagram rendering for ```mermaid code blocks.
 */

import { useEffect, useRef, useMemo, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { Components } from "react-markdown";
import mermaid from "mermaid";
import { t as tRaw } from "../../lib/i18n";
import { useI18n } from "../../hooks/use-i18n";

// Initialize mermaid with dark theme matching Obsidian Observatory
mermaid.initialize({
  startOnLoad: false,
  theme: "dark",
  themeVariables: {
    darkMode: true,
    background: "#0e1118",
    primaryColor: "#1c2233",
    primaryTextColor: "#e2e8f0",
    primaryBorderColor: "#5b9cf6",
    secondaryColor: "#252b3b",
    secondaryTextColor: "#c1cad8",
    tertiaryColor: "#1c212d",
    lineColor: "#5b9cf6",
    textColor: "#c1cad8",
    mainBkg: "#1c2233",
    nodeBorder: "#5b9cf6",
    clusterBkg: "#151922",
    clusterBorder: "rgba(148, 163, 194, 0.15)",
    titleColor: "#e2e8f0",
    edgeLabelBackground: "#151922",
    nodeTextColor: "#e2e8f0",
  },
  flowchart: {
    htmlLabels: true,
    curve: "basis",
    padding: 12,
  },
  fontFamily: "'DM Sans', system-ui, sans-serif",
  fontSize: 13,
});

// ─── Props ──────────────────────────────────────────────────────────

interface DocsContentProps {
  content: string;
  title: string;
}

// ─── Mermaid Block ──────────────────────────────────────────────────

function MermaidDiagram({ chart }: { chart: string }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [svg, setSvg] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const idRef = useRef(`mermaid-${Math.random().toString(36).slice(2, 9)}`);

  useEffect(() => {
    let cancelled = false;

    async function render() {
      try {
        const { svg: rendered } = await mermaid.render(idRef.current, chart.trim());
        if (!cancelled) {
          setSvg(rendered);
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          setError((err as Error).message || "Failed to render diagram");
          // Clean up any failed render elements
          const errEl = document.getElementById("d" + idRef.current);
          if (errEl) errEl.remove();
        }
      }
    }

    render();
    return () => { cancelled = true; };
  }, [chart]);

  if (error) {
    return (
      <div
        className="my-4 p-4 rounded-lg text-xs overflow-x-auto"
        style={{ background: "var(--rose-subtle)", color: "var(--rose)", border: "1px solid var(--rose)" }}
      >
        <p className="font-medium mb-1">{tRaw("docs.diagramError")}</p>
        <pre className="whitespace-pre-wrap">{error}</pre>
      </div>
    );
  }

  return (
    <div
      ref={containerRef}
      className="my-6 flex justify-center overflow-x-auto rounded-xl p-6"
      style={{
        background: "var(--bg-1)",
        border: "1px solid var(--surface-border)",
      }}
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}

// ─── Markdown Components ────────────────────────────────────────────

function createMarkdownComponents(onNavigate?: (path: string) => void): Components {
  return {
    // Headings
    h1: ({ children }) => (
      <h1
        className="text-2xl font-bold mt-0 mb-4 pb-3"
        style={{
          fontFamily: "var(--font-display)",
          color: "var(--text-0)",
          borderBottom: "1px solid var(--surface-border)",
        }}
      >
        {children}
      </h1>
    ),
    h2: ({ children }) => (
      <h2
        className="text-lg font-semibold mt-8 mb-3"
        style={{ fontFamily: "var(--font-display)", color: "var(--text-0)" }}
      >
        {children}
      </h2>
    ),
    h3: ({ children }) => (
      <h3
        className="text-base font-semibold mt-6 mb-2"
        style={{ fontFamily: "var(--font-display)", color: "var(--text-0)" }}
      >
        {children}
      </h3>
    ),

    // Paragraphs
    p: ({ children }) => (
      <p className="mb-4 leading-relaxed" style={{ color: "var(--text-1)" }}>
        {children}
      </p>
    ),

    // Links — handle relative doc links
    a: ({ href, children }) => {
      const isInternal = href && !href.startsWith("http") && !href.startsWith("#");
      return (
        <a
          href={href}
          onClick={(e) => {
            if (isInternal && onNavigate) {
              e.preventDefault();
              onNavigate(href!);
            }
          }}
          className="transition-colors"
          style={{ color: "var(--accent)", textDecoration: "underline", textUnderlineOffset: "2px" }}
        >
          {children}
        </a>
      );
    },

    // Code blocks — special handling for mermaid
    pre: ({ children }) => {
      // Check if this is a code element with mermaid language
      const child = children as React.ReactElement<{ className?: string; children?: React.ReactNode }>;
      if (child?.props?.className === "language-mermaid") {
        const code = String(child.props.children ?? "").replace(/\n$/, "");
        return <MermaidDiagram chart={code} />;
      }

      return (
        <pre
          className="my-4 p-4 rounded-lg overflow-x-auto text-[13px] leading-relaxed"
          style={{
            background: "var(--bg-1)",
            border: "1px solid var(--surface-border)",
            fontFamily: "var(--font-mono)",
          }}
        >
          {children}
        </pre>
      );
    },

    // Inline code
    code: ({ className, children }) => {
      // If inside a pre (block), render as-is
      if (className) {
        return <code className={className}>{children}</code>;
      }
      return (
        <code
          className="px-1.5 py-0.5 rounded text-[12px]"
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

    // Tables
    table: ({ children }) => (
      <div className="my-4 overflow-x-auto rounded-lg" style={{ border: "1px solid var(--surface-border)" }}>
        <table className="w-full text-[13px]">{children}</table>
      </div>
    ),
    thead: ({ children }) => (
      <thead style={{ background: "var(--bg-2)" }}>{children}</thead>
    ),
    th: ({ children }) => (
      <th
        className="px-3 py-2 text-left font-medium text-xs"
        style={{ color: "var(--text-2)", borderBottom: "1px solid var(--surface-border)" }}
      >
        {children}
      </th>
    ),
    td: ({ children }) => (
      <td
        className="px-3 py-2"
        style={{ color: "var(--text-1)", borderBottom: "1px solid var(--surface-border)" }}
      >
        {children}
      </td>
    ),

    // Lists
    ul: ({ children }) => (
      <ul className="mb-4 pl-5 space-y-1" style={{ color: "var(--text-1)", listStyleType: "disc" }}>
        {children}
      </ul>
    ),
    ol: ({ children }) => (
      <ol className="mb-4 pl-5 space-y-1" style={{ color: "var(--text-1)", listStyleType: "decimal" }}>
        {children}
      </ol>
    ),
    li: ({ children }) => <li className="leading-relaxed">{children}</li>,

    // Blockquotes
    blockquote: ({ children }) => (
      <blockquote
        className="my-4 pl-4 py-1"
        style={{
          borderLeft: "3px solid var(--accent)",
          color: "var(--text-2)",
          background: "var(--accent-subtle)",
          borderRadius: "0 var(--radius-sm) var(--radius-sm) 0",
        }}
      >
        {children}
      </blockquote>
    ),

    // Horizontal rule
    hr: () => <hr className="my-8" style={{ border: "none", borderTop: "1px solid var(--surface-border)" }} />,

    // Strong / emphasis
    strong: ({ children }) => <strong style={{ color: "var(--text-0)", fontWeight: 600 }}>{children}</strong>,
    em: ({ children }) => <em style={{ color: "var(--text-1)" }}>{children}</em>,
  };
}

// ─── Main Component ─────────────────────────────────────────────────

export function DocsContent({ content, title: _title }: DocsContentProps) {
  const { t } = useI18n();
  const contentRef = useRef<HTMLDivElement>(null);
  const [toc, setToc] = useState<{ id: string; text: string; level: number }[]>([]);

  // Extract table of contents from markdown headings
  useEffect(() => {
    const headings: { id: string; text: string; level: number }[] = [];
    const lines = content.split("\n");
    for (const line of lines) {
      const match = line.match(/^(#{2,3})\s+(.+)$/);
      if (match) {
        const level = match[1].length;
        const text = match[2].trim();
        const id = text.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "");
        headings.push({ id, text, level });
      }
    }
    setToc(headings);
  }, [content]);

  // Scroll to top when content changes
  useEffect(() => {
    contentRef.current?.scrollTo(0, 0);
  }, [content]);

  const components = useMemo(() => createMarkdownComponents(), []);

  return (
    <div className="flex h-full">
      {/* Main content */}
      <div ref={contentRef} className="flex-1 overflow-y-auto">
        <div className="max-w-3xl mx-auto px-8 py-8">
          <article className="docs-prose">
            <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
              {content}
            </ReactMarkdown>
          </article>
        </div>
      </div>

      {/* Table of contents (right sidebar) */}
      {toc.length > 2 && (
        <div
          className="flex-shrink-0 overflow-y-auto py-8 px-4 hidden xl:block"
          style={{ width: 200, borderLeft: "1px solid var(--surface-border)" }}
        >
          <p
            className="text-[11px] font-medium uppercase tracking-wider mb-3"
            style={{ color: "var(--text-3)" }}
          >
            {t("docs.onThisPage")}
          </p>
          <nav className="space-y-1">
            {toc.map((heading) => (
              <a
                key={heading.id}
                href={`#${heading.id}`}
                className="block text-[12px] py-0.5 transition-colors truncate"
                style={{
                  color: "var(--text-3)",
                  paddingLeft: heading.level === 3 ? 12 : 0,
                }}
                onClick={(e) => {
                  e.preventDefault();
                  // Find heading by text content
                  const headings = contentRef.current?.querySelectorAll("h2, h3");
                  headings?.forEach((h) => {
                    if (h.textContent?.trim() === heading.text) {
                      h.scrollIntoView({ behavior: "smooth", block: "start" });
                    }
                  });
                }}
              >
                {heading.text}
              </a>
            ))}
          </nav>
        </div>
      )}
    </div>
  );
}
