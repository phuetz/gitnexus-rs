import { useEffect, useRef, useState } from "react";
import { useFileContent } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";
import { useSymbolContext } from "../../hooks/use-tauri-query";

export function CodePanel() {
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const { data: context } = useSymbolContext(selectedNodeId);
  const containerRef = useRef<HTMLDivElement>(null);
  const [highlightedHtml, setHighlightedHtml] = useState<string>("");

  const filePath = context?.node.filePath ?? null;
  const { data: fileContent, isLoading } = useFileContent(filePath);

  useEffect(() => {
    if (!fileContent?.content) {
      setHighlightedHtml("");
      return;
    }

    // Use Shiki for syntax highlighting
    let cancelled = false;

    (async () => {
      try {
        const shiki = await import("shiki");
        const highlighter = await shiki.createHighlighter({
          themes: ["tokyo-night"],
          langs: [
            fileContent.language ?? "text",
          ],
        });

        if (cancelled) {
          highlighter.dispose();
          return;
        }

        const html = highlighter.codeToHtml(fileContent.content, {
          lang: fileContent.language ?? "text",
          theme: "tokyo-night",
        });

        highlighter.dispose();
        if (!cancelled) {
          setHighlightedHtml(html);
        }
      } catch {
        // Fallback: plain text with line numbers
        if (!cancelled) {
          const lines = fileContent.content.split("\n");
          const html = lines
            .map(
              (line, i) =>
                `<span class="line-number">${i + 1}</span><span>${escapeHtml(line)}</span>`
            )
            .join("\n");
          setHighlightedHtml(`<pre class="fallback-code">${html}</pre>`);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [fileContent?.content, fileContent?.language]);

  // Scroll to highlighted lines when content loads
  useEffect(() => {
    if (!containerRef.current || !context?.node.startLine || !highlightedHtml)
      return;

    const lineEl = containerRef.current.querySelector(
      `.line:nth-child(${context.node.startLine})`
    );
    if (lineEl) {
      lineEl.scrollIntoView({ block: "center", behavior: "smooth" });
    }
  }, [highlightedHtml, context?.node.startLine]);

  if (!selectedNodeId) {
    return (
      <div className="h-full flex items-center justify-center text-[var(--text-muted)]">
        Select a symbol to view its code
      </div>
    );
  }

  if (!filePath) {
    return (
      <div className="h-full flex items-center justify-center text-[var(--text-muted)]">
        No file associated with this symbol
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center text-[var(--text-muted)]">
        Loading code...
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="px-3 py-1.5 border-b border-[var(--border)] bg-[var(--bg-secondary)] text-xs text-[var(--text-muted)] flex items-center gap-2">
        <span className="truncate">{filePath}</span>
        {context?.node.startLine && (
          <span>
            L{context.node.startLine}
            {context.node.endLine && `-${context.node.endLine}`}
          </span>
        )}
        {fileContent && (
          <span className="ml-auto">{fileContent.totalLines} lines</span>
        )}
      </div>
      {/* Code */}
      <div
        ref={containerRef}
        className="flex-1 overflow-auto text-[12px] leading-[1.6]"
        dangerouslySetInnerHTML={{ __html: highlightedHtml }}
        style={{
          fontFamily: "ui-monospace, Consolas, 'Courier New', monospace",
        }}
      />
    </div>
  );
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
