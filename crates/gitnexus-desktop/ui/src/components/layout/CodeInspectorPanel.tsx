/**
 * Code Inspector — Left panel showing stacked code snippets for the selected node,
 * its callers, and callees. Matches the competitor's 3-panel "Code Inspector" layout.
 */

import { useState } from "react";
import { Code2, ChevronDown, ChevronRight, FileCode } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import { useSymbolContext, useFileContent } from "../../hooks/use-tauri-query";

/** A single collapsible code section with file path header + source code */
function CodeSection({
  title,
  label,
  filePath,
  startLine,
  endLine,
  color,
  defaultExpanded,
}: {
  title: string;
  label?: string;
  filePath: string;
  startLine?: number;
  endLine?: number;
  color: string;
  defaultExpanded: boolean;
}) {
  const [expanded, setExpanded] = useState(defaultExpanded);
  const { data } = useFileContent(
    expanded ? filePath : null,
    startLine,
    endLine ? endLine + 3 : undefined
  );

  return (
    <div
      style={{
        borderBottom: "1px solid var(--border)",
      }}
    >
      {/* Header */}
      <button
        onClick={() => setExpanded(!expanded)}
        style={{
          width: "100%",
          padding: "8px 12px",
          background: "var(--bg-1)",
          border: "none",
          borderLeft: `3px solid ${color}`,
          color: "var(--text-1)",
          fontSize: 11,
          cursor: "pointer",
          display: "flex",
          alignItems: "center",
          gap: 6,
          textAlign: "left",
        }}
      >
        {expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        <FileCode size={12} style={{ color, flexShrink: 0 }} />
        <span
          style={{
            flex: 1,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            fontWeight: 500,
          }}
        >
          {title}
        </span>
        {label && (
          <span
            style={{
              fontSize: 9,
              padding: "1px 5px",
              borderRadius: 3,
              background: "var(--bg-3)",
              color: "var(--text-3)",
              flexShrink: 0,
            }}
          >
            {label}
          </span>
        )}
      </button>

      {/* Code content */}
      {expanded && (
        <div style={{ background: "var(--bg-0)" }}>
          {/* File path */}
          <div
            style={{
              padding: "3px 12px",
              fontSize: 10,
              color: "var(--text-3)",
              fontFamily: "var(--font-mono)",
              borderBottom: "1px solid var(--border)",
              background: "var(--bg-1)",
            }}
          >
            {filePath.replace(/\\/g, "/")}
            {startLine != null && `:${startLine}`}
            {endLine != null && `-${endLine}`}
          </div>

          {/* Source */}
          {data ? (
            <pre
              style={{
                padding: "8px 12px",
                margin: 0,
                fontSize: 11,
                lineHeight: 1.6,
                fontFamily: "var(--font-mono)",
                color: "var(--text-1)",
                maxHeight: 220,
                overflow: "auto",
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
              }}
            >
              {data.content}
            </pre>
          ) : (
            <div
              style={{
                padding: "12px",
                color: "var(--text-3)",
                fontSize: 11,
                textAlign: "center",
              }}
            >
              Loading...
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function CodeInspectorPanel() {
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const selectedNodeName = useAppStore((s) => s.selectedNodeName);
  const { data: context } = useSymbolContext(selectedNodeId);

  const node = context?.node;
  const callers = context?.callers || [];
  const callees = context?.callees || [];

  if (!selectedNodeId) {
    return (
      <div
        style={{
          height: "100%",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: 20,
          background: "var(--bg-0)",
          borderRight: "1px solid var(--border)",
        }}
      >
        <Code2
          size={32}
          style={{ color: "var(--text-4)", marginBottom: 8 }}
        />
        <p style={{ color: "var(--text-3)", fontSize: 12, textAlign: "center" }}>
          Select a node in the graph to inspect its code
        </p>
      </div>
    );
  }

  return (
    <div
      style={{
        height: "100%",
        display: "flex",
        flexDirection: "column",
        background: "rgba(9, 11, 16, 0.85)",
        backdropFilter: "blur(12px)",
        borderRight: "1px solid var(--border)",
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "10px 12px",
          borderBottom: "1px solid var(--border)",
          display: "flex",
          alignItems: "center",
          gap: 8,
          flexShrink: 0,
        }}
      >
        <Code2 size={14} style={{ color: "var(--accent)" }} />
        <span
          style={{
            fontSize: 12,
            fontWeight: 600,
            color: "var(--text-0)",
          }}
        >
          Code Inspector
        </span>
      </div>

      {/* Scrollable content */}
      <div style={{ flex: 1, overflow: "auto" }}>
        {/* Primary: selected node */}
        {node && node.filePath && (
          <CodeSection
            title={selectedNodeName || node.name}
            label={node.label}
            filePath={node.filePath}
            startLine={node.startLine}
            endLine={node.endLine}
            color="var(--accent)"
            defaultExpanded={true}
          />
        )}

        {/* Callers */}
        {callers.length > 0 && (
          <>
            <div
              style={{
                padding: "6px 12px",
                fontSize: 10,
                fontWeight: 600,
                color: "var(--text-3)",
                textTransform: "uppercase",
                letterSpacing: "0.05em",
                background: "var(--bg-2)",
              }}
            >
              Callers ({callers.length})
            </div>
            {callers.slice(0, 5).map((c) => (
              <CodeSection
                key={c.id}
                title={c.name}
                label={c.label}
                filePath={c.filePath}
                startLine={undefined}
                endLine={undefined}
                color="#9ece6a"
                defaultExpanded={false}
              />
            ))}
          </>
        )}

        {/* Callees */}
        {callees.length > 0 && (
          <>
            <div
              style={{
                padding: "6px 12px",
                fontSize: 10,
                fontWeight: 600,
                color: "var(--text-3)",
                textTransform: "uppercase",
                letterSpacing: "0.05em",
                background: "var(--bg-2)",
              }}
            >
              Callees ({callees.length})
            </div>
            {callees.slice(0, 5).map((c) => (
              <CodeSection
                key={c.id}
                title={c.name}
                label={c.label}
                filePath={c.filePath}
                startLine={undefined}
                endLine={undefined}
                color="#bb9af7"
                defaultExpanded={false}
              />
            ))}
          </>
        )}
      </div>
    </div>
  );
}
