/**
 * Resizable code overlay on the graph canvas showing the selected node's source code
 * and its immediate relationships (callers, callees).
 */

import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Code2, X, ChevronDown, ChevronRight } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import { useSymbolContext, useFileContent } from "../../hooks/use-tauri-query";

export function CodeReferencesOverlay() {
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const selectedNodeName = useAppStore((s) => s.selectedNodeName);
  const [visible, setVisible] = useState(false);
  const [callersOpen, setCallersOpen] = useState(true);
  const [calleesOpen, setCalleesOpen] = useState(true);

  const { data: context } = useSymbolContext(selectedNodeId);

  // Get file path and line range from context
  const node = context?.node;
  const filePath = node?.filePath ?? null;
  const startLine = node?.startLine;
  const endLine = node?.endLine;

  const { data: fileData } = useFileContent(
    visible && filePath ? filePath : null,
    startLine,
    endLine ? endLine + 5 : undefined
  );

  // Toggle button when no selection
  if (!selectedNodeId) {
    return null;
  }

  return (
    <>
      {/* Toggle button */}
      {!visible && (
        <motion.button
          initial={{ opacity: 0, scale: 0.9 }}
          animate={{ opacity: 1, scale: 1 }}
          onClick={() => setVisible(true)}
          style={{
            position: "absolute",
            top: 12,
            right: 12,
            zIndex: 20,
            padding: "6px 12px",
            borderRadius: 8,
            border: "1px solid var(--border)",
            background: "var(--bg-1)",
            color: "var(--text-1)",
            fontSize: 11,
            fontWeight: 500,
            cursor: "pointer",
            display: "flex",
            alignItems: "center",
            gap: 6,
            boxShadow: "0 2px 8px rgba(0,0,0,0.2)",
          }}
        >
          <Code2 size={14} />
          Code
        </motion.button>
      )}

      {/* Overlay panel */}
      <AnimatePresence>
        {visible && (
          <motion.div
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 20 }}
            transition={{ duration: 0.15 }}
            style={{
              position: "absolute",
              top: 12,
              right: 12,
              zIndex: 20,
              width: 380,
              maxHeight: "calc(100% - 80px)",
              borderRadius: 12,
              border: "1px solid var(--border)",
              background: "var(--bg-0)",
              boxShadow:
                "0 8px 32px rgba(0,0,0,0.4), 0 0 0 1px rgba(122,162,247,0.1)",
              display: "flex",
              flexDirection: "column",
              overflow: "hidden",
            }}
          >
            {/* Header */}
            <div
              style={{
                padding: "10px 14px",
                borderBottom: "1px solid var(--border)",
                display: "flex",
                alignItems: "center",
                gap: 8,
              }}
            >
              <Code2 size={14} style={{ color: "var(--accent)" }} />
              <span
                style={{
                  fontSize: 12,
                  fontWeight: 600,
                  color: "var(--text-0)",
                  flex: 1,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {selectedNodeName || "Selected"}
              </span>
              {node && (
                <span
                  style={{
                    fontSize: 10,
                    padding: "2px 6px",
                    borderRadius: 4,
                    background: "var(--bg-2)",
                    color: "var(--text-2)",
                  }}
                >
                  {node.label}
                </span>
              )}
              <button
                onClick={() => setVisible(false)}
                style={{
                  background: "none",
                  border: "none",
                  color: "var(--text-3)",
                  cursor: "pointer",
                  padding: 2,
                  display: "flex",
                }}
              >
                <X size={14} />
              </button>
            </div>

            {/* Source code */}
            {fileData && (
              <div
                style={{
                  maxHeight: 200,
                  overflow: "auto",
                  borderBottom: "1px solid var(--border)",
                }}
              >
                <div
                  style={{
                    padding: "4px 14px",
                    fontSize: 10,
                    color: "var(--text-3)",
                    background: "var(--bg-1)",
                    borderBottom: "1px solid var(--border)",
                  }}
                >
                  {filePath}
                  {startLine != null && `:${startLine}`}
                  {endLine != null && `-${endLine}`}
                </div>
                <pre
                  style={{
                    padding: "8px 14px",
                    margin: 0,
                    fontSize: 11,
                    lineHeight: 1.6,
                    fontFamily: "var(--font-mono)",
                    color: "var(--text-1)",
                    background: "var(--bg-1)",
                    whiteSpace: "pre-wrap",
                    wordBreak: "break-all",
                    overflow: "auto",
                  }}
                >
                  {fileData.content}
                </pre>
              </div>
            )}

            {/* Relationships */}
            <div style={{ overflow: "auto", flex: 1 }}>
              {/* Callers */}
              {context?.callers && context.callers.length > 0 && (
                <div>
                  <button
                    onClick={() => setCallersOpen(!callersOpen)}
                    style={{
                      width: "100%",
                      padding: "8px 14px",
                      background: "none",
                      border: "none",
                      borderBottom: "1px solid var(--border)",
                      color: "var(--text-1)",
                      fontSize: 11,
                      fontWeight: 600,
                      cursor: "pointer",
                      display: "flex",
                      alignItems: "center",
                      gap: 6,
                      textAlign: "left",
                    }}
                  >
                    {callersOpen ? (
                      <ChevronDown size={12} />
                    ) : (
                      <ChevronRight size={12} />
                    )}
                    Callers ({context.callers.length})
                  </button>
                  {callersOpen && (
                    <div style={{ padding: "4px 14px" }}>
                      {context.callers.slice(0, 10).map((c) => (
                        <div
                          key={c.id}
                          style={{
                            padding: "4px 0",
                            fontSize: 11,
                            color: "var(--text-2)",
                            display: "flex",
                            alignItems: "center",
                            gap: 6,
                          }}
                        >
                          <span
                            style={{
                              width: 6,
                              height: 6,
                              borderRadius: "50%",
                              background: "var(--accent)",
                              flexShrink: 0,
                            }}
                          />
                          <span style={{ color: "var(--text-1)" }}>
                            {c.name}
                          </span>
                          <span
                            style={{
                              fontSize: 10,
                              color: "var(--text-3)",
                              marginLeft: "auto",
                            }}
                          >
                            {c.label}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* Callees */}
              {context?.callees && context.callees.length > 0 && (
                <div>
                  <button
                    onClick={() => setCalleesOpen(!calleesOpen)}
                    style={{
                      width: "100%",
                      padding: "8px 14px",
                      background: "none",
                      border: "none",
                      borderBottom: "1px solid var(--border)",
                      color: "var(--text-1)",
                      fontSize: 11,
                      fontWeight: 600,
                      cursor: "pointer",
                      display: "flex",
                      alignItems: "center",
                      gap: 6,
                      textAlign: "left",
                    }}
                  >
                    {calleesOpen ? (
                      <ChevronDown size={12} />
                    ) : (
                      <ChevronRight size={12} />
                    )}
                    Callees ({context.callees.length})
                  </button>
                  {calleesOpen && (
                    <div style={{ padding: "4px 14px" }}>
                      {context.callees.slice(0, 10).map((c) => (
                        <div
                          key={c.id}
                          style={{
                            padding: "4px 0",
                            fontSize: 11,
                            color: "var(--text-2)",
                            display: "flex",
                            alignItems: "center",
                            gap: 6,
                          }}
                        >
                          <span
                            style={{
                              width: 6,
                              height: 6,
                              borderRadius: "50%",
                              background: "#9ece6a",
                              flexShrink: 0,
                            }}
                          />
                          <span style={{ color: "var(--text-1)" }}>
                            {c.name}
                          </span>
                          <span
                            style={{
                              fontSize: 10,
                              color: "var(--text-3)",
                              marginLeft: "auto",
                            }}
                          >
                            {c.label}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* No relationships */}
              {(!context?.callers || context.callers.length === 0) &&
                (!context?.callees || context.callees.length === 0) &&
                !fileData && (
                  <div
                    style={{
                      padding: "16px 14px",
                      textAlign: "center",
                      color: "var(--text-3)",
                      fontSize: 12,
                    }}
                  >
                    Select a node to view code and relationships
                  </div>
                )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}
