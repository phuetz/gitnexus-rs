/**
 * Code Inspector — Left panel showing stacked code snippets for the selected node,
 * its callers, and callees. Matches the competitor's 3-panel "Code Inspector" layout.
 */

import { useState } from "react";
import { Code2, ChevronDown, ChevronRight, FileCode, Package } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";
import { useSymbolContext, useFileContent } from "../../hooks/use-tauri-query";
import { useShikiTokens } from "../../hooks/use-shiki";
import type { RelatedNode } from "../../lib/tauri-commands";

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
  const { tokens } = useShikiTokens(
    expanded ? data?.content : undefined,
    data?.language
  );
  const baseLineNum = startLine ?? 1;

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
            <span>{filePath.replace(/\\/g, "/")}</span>
            {startLine != null && <span>:{startLine}</span>}
            {endLine != null && <span>-{endLine}</span>}
            {data?.language && (
              <span
                style={{
                  marginLeft: 8,
                  padding: "1px 5px",
                  borderRadius: 3,
                  background: "var(--bg-3)",
                  color: "var(--text-2)",
                  fontSize: 9,
                }}
              >
                {data.language}
              </span>
            )}
          </div>

          {/* Source with syntax highlighting */}
          {data ? (
            <pre
              style={{
                margin: 0,
                fontSize: 11,
                lineHeight: 1.6,
                fontFamily: "var(--font-mono)",
                color: "var(--text-1)",
                maxHeight: 220,
                overflow: "auto",
              }}
            >
              <code>
                {tokens
                  ? tokens.map((lineTokens, i) => (
                      <div
                        key={i}
                        style={{
                          display: "flex",
                          minHeight: "1.6em",
                        }}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.backgroundColor = "var(--bg-2)";
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.backgroundColor = "transparent";
                        }}
                      >
                        <span
                          style={{
                            paddingLeft: 8,
                            paddingRight: 12,
                            color: "var(--text-4)",
                            width: "3.5em",
                            textAlign: "right",
                            userSelect: "none",
                            flexShrink: 0,
                            fontSize: 10,
                          }}
                        >
                          {baseLineNum + i}
                        </span>
                        <span style={{ flex: 1, whiteSpace: "pre", paddingRight: 8 }}>
                          {lineTokens.map((token, j) => (
                            <span key={j} style={{ color: token.color }}>
                              {token.content}
                            </span>
                          ))}
                        </span>
                      </div>
                    ))
                  : data.content.split("\n").map((line, i) => (
                      <div
                        key={i}
                        style={{
                          display: "flex",
                          minHeight: "1.6em",
                        }}
                      >
                        <span
                          style={{
                            paddingLeft: 8,
                            paddingRight: 12,
                            color: "var(--text-4)",
                            width: "3.5em",
                            textAlign: "right",
                            userSelect: "none",
                            flexShrink: 0,
                            fontSize: 10,
                          }}
                        >
                          {baseLineNum + i}
                        </span>
                        <span style={{ flex: 1, whiteSpace: "pre", paddingRight: 8 }}>
                          {line}
                        </span>
                      </div>
                    ))}
              </code>
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

/** Collapsible list of dependency nodes (imports) with click-to-navigate */
function DependenciesSection({
  items,
  color,
}: {
  items: RelatedNode[];
  color: string;
}) {
  const [expanded, setExpanded] = useState(true);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);

  return (
    <div style={{ borderBottom: "1px solid var(--border)" }}>
      {/* Section header */}
      <button
        onClick={() => setExpanded(!expanded)}
        style={{
          width: "100%",
          padding: "6px 12px",
          background: "var(--bg-2)",
          border: "none",
          borderLeft: `3px solid ${color}`,
          color: "var(--text-3)",
          fontSize: 10,
          fontWeight: 600,
          cursor: "pointer",
          display: "flex",
          alignItems: "center",
          gap: 6,
          textAlign: "left",
          textTransform: "uppercase",
          letterSpacing: "0.05em",
        }}
      >
        {expanded ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
        <Package size={10} style={{ color, flexShrink: 0 }} />
        Dependencies ({items.length})
      </button>

      {/* Item list */}
      {expanded && (
        <div style={{ background: "var(--bg-0)" }}>
          {items.map((dep) => (
            <button
              key={dep.id}
              onClick={() => setSelectedNodeId(dep.id, dep.name)}
              style={{
                width: "100%",
                padding: "6px 12px",
                background: "transparent",
                border: "none",
                borderBottom: "1px solid var(--border)",
                color: "var(--text-1)",
                fontSize: 11,
                cursor: "pointer",
                display: "flex",
                flexDirection: "column",
                gap: 2,
                textAlign: "left",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = "var(--bg-2)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = "transparent";
              }}
            >
              <span
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                }}
              >
                <FileCode size={11} style={{ color, flexShrink: 0 }} />
                <span style={{ fontWeight: 500 }}>{dep.name}</span>
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
                  {dep.label}
                </span>
              </span>
              {dep.filePath && (
                <span
                  style={{
                    fontSize: 10,
                    color: "var(--text-4)",
                    fontFamily: "var(--font-mono)",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                    paddingLeft: 17,
                  }}
                >
                  {dep.filePath.replace(/\\/g, "/")}
                </span>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export function CodeInspectorPanel() {
  const { t } = useI18n();
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const selectedNodeName = useAppStore((s) => s.selectedNodeName);
  const { data: context } = useSymbolContext(selectedNodeId);

  const node = context?.node;
  const imports = context?.imports || [];
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
          {t("codeInspector.selectNode")}
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
        background: "var(--glass-bg)",
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
          {t("codeInspector.title")}
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

        {/* Dependencies (imports) */}
        {imports.length > 0 && (
          <DependenciesSection items={imports} color="#3b82f6" />
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
              {t("detail.callers")} ({callers.length})
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
              {t("detail.callees")} ({callees.length})
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
