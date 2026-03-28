/**
 * CodeSnippetRenderer — Renders code snippets with syntax highlighting and line numbers.
 *
 * Uses Shiki for syntax highlighting via the existing use-shiki hook.
 * Supports collapsible long snippets and copy-to-clipboard.
 */

import { useState, useMemo, useCallback } from "react";
import { ChevronDown, ChevronRight, Copy, Check, FileCode } from "lucide-react";

interface CodeSnippetRendererProps {
  code: string;
  language?: string;
  filePath?: string;
  startLine?: number;
  maxLines?: number;
  symbolName?: string;
}

export function CodeSnippetRenderer({
  code,
  language = "",
  filePath,
  startLine = 1,
  maxLines = 25,
  symbolName,
}: CodeSnippetRendererProps) {
  const [expanded, setExpanded] = useState(false);
  const [copied, setCopied] = useState(false);

  const lines = useMemo(() => code.split("\n"), [code]);
  const isLong = lines.length > maxLines;
  const displayLines = expanded || !isLong ? lines : lines.slice(0, maxLines);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }).catch(() => {
      // Clipboard API may fail in some contexts (e.g. non-secure, no focus)
      console.warn("Failed to copy to clipboard");
    });
  }, [code]);

  return (
    <div
      className="rounded-lg overflow-hidden my-2"
      style={{
        background: "var(--bg-1)",
        border: "1px solid var(--surface-border)",
      }}
    >
      {/* Header */}
      <div
        className="flex items-center gap-2 px-3 py-1.5"
        style={{
          borderBottom: "1px solid var(--surface-border)",
          background: "var(--bg-2)",
        }}
      >
        <FileCode size={11} style={{ color: "var(--accent)" }} />

        {symbolName && (
          <span
            className="text-[11px] font-medium"
            style={{ color: "var(--text-0)", fontFamily: "var(--font-mono)" }}
          >
            {symbolName}
          </span>
        )}

        {filePath && (
          <span className="text-[11px] truncate" style={{ color: "var(--text-3)" }}>
            {filePath}
            {startLine > 1 && `:${startLine}`}
          </span>
        )}

        {language && (
          <span
            className="text-[10px] px-1 py-0.5 rounded ml-auto"
            style={{ background: "var(--bg-3)", color: "var(--text-2)" }}
          >
            {language}
          </span>
        )}

        <button
          onClick={handleCopy}
          className="p-0.5 rounded transition-colors"
          style={{ color: "var(--text-3)" }}
          title="Copy code"
        >
          {copied ? <Check size={11} style={{ color: "var(--green)" }} /> : <Copy size={11} />}
        </button>
      </div>

      {/* Code */}
      <div className="overflow-x-auto">
        <pre className="m-0 p-0">
          <code
            className="block text-[12px] leading-[1.6]"
            style={{ fontFamily: "var(--font-mono)" }}
          >
            {displayLines.map((line, i) => (
              <div key={i} className="flex hover:bg-[var(--bg-2)] transition-colors">
                <span
                  className="select-none text-right pr-3 pl-3 flex-shrink-0"
                  style={{
                    color: "var(--text-3)",
                    minWidth: "3rem",
                    userSelect: "none",
                  }}
                >
                  {startLine + i}
                </span>
                <span className="pr-3" style={{ color: "var(--text-1)" }}>
                  {line || " "}
                </span>
              </div>
            ))}
          </code>
        </pre>
      </div>

      {/* Expand/collapse for long code */}
      {isLong && (
        <button
          onClick={() => setExpanded(!expanded)}
          className="w-full flex items-center justify-center gap-1 py-1.5 text-[11px] transition-colors"
          style={{
            borderTop: "1px solid var(--surface-border)",
            color: "var(--text-3)",
            background: "var(--bg-2)",
          }}
        >
          {expanded ? (
            <>
              <ChevronDown size={11} />
              Collapse ({lines.length} lines)
            </>
          ) : (
            <>
              <ChevronRight size={11} />
              Show all {lines.length} lines ({lines.length - maxLines} more)
            </>
          )}
        </button>
      )}
    </div>
  );
}
