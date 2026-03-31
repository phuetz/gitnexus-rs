/**
 * Floating Action Button for running ad-hoc Cypher queries against the knowledge graph.
 */

import { useState, useRef, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Terminal, Play, X, ChevronRight } from "lucide-react";
import { commands } from "../../lib/tauri-commands";

const EXAMPLES = [
  {
    label: "All Functions",
    query: "MATCH (n:Function) RETURN n.name, n.filePath LIMIT 20",
  },
  {
    label: "Call Graph",
    query: "MATCH (n)-[:CALLS]->(m) RETURN n.name, m.name LIMIT 30",
  },
  {
    label: "Inheritance",
    query: "MATCH (n:Class)-[:INHERITS]->(m) RETURN n.name, m.name",
  },
  {
    label: "Controllers",
    query:
      "MATCH (n:Controller)-[:DEFINES]->(a:ControllerAction) RETURN n.name, a.name LIMIT 30",
  },
  {
    label: "Communities",
    query: "MATCH (n:Community) RETURN n.name LIMIT 20",
  },
];

export function CypherQueryFAB() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<unknown[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (open && inputRef.current) {
      inputRef.current.focus();
    }
  }, [open]);

  async function runQuery() {
    if (!query.trim()) return;
    setRunning(true);
    setError(null);
    setResults(null);
    try {
      const res = await commands.executeCypher(query.trim());
      setResults(res);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setRunning(false);
    }
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      runQuery();
    }
    if (e.key === "Escape") {
      setOpen(false);
    }
  }

  return (
    <>
      {/* FAB button */}
      <motion.button
        onClick={() => setOpen(!open)}
        whileHover={{ scale: 1.1 }}
        whileTap={{ scale: 0.95 }}
        style={{
          position: "absolute",
          bottom: 52,
          right: 16,
          zIndex: 20,
          width: 44,
          height: 44,
          borderRadius: 12,
          border: "1px solid var(--border)",
          background: open ? "var(--accent)" : "var(--bg-1)",
          color: open ? "#fff" : "var(--text-1)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          cursor: "pointer",
          boxShadow: "0 4px 16px rgba(0,0,0,0.3)",
        }}
        title="Cypher Query (Ctrl+Enter to run)"
      >
        {open ? <X size={18} /> : <Terminal size={18} />}
      </motion.button>

      {/* Query panel */}
      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, y: 20, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 20, scale: 0.95 }}
            transition={{ duration: 0.15 }}
            style={{
              position: "absolute",
              bottom: 104,
              right: 16,
              zIndex: 20,
              width: 420,
              maxHeight: 480,
              borderRadius: 12,
              border: "1px solid var(--border)",
              background: "var(--bg-0)",
              boxShadow: "0 8px 32px rgba(0,0,0,0.4)",
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
                fontSize: 12,
                fontWeight: 600,
                color: "var(--text-1)",
                display: "flex",
                alignItems: "center",
                gap: 6,
              }}
            >
              <Terminal size={14} />
              Cypher Query
              <span
                style={{
                  marginLeft: "auto",
                  fontSize: 10,
                  color: "var(--text-3)",
                }}
              >
                Ctrl+Enter to run
              </span>
            </div>

            {/* Query input */}
            <textarea
              ref={inputRef}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="MATCH (n:Function) RETURN n.name LIMIT 10"
              rows={3}
              style={{
                width: "100%",
                padding: "10px 14px",
                background: "var(--bg-1)",
                border: "none",
                borderBottom: "1px solid var(--border)",
                color: "var(--text-0)",
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                resize: "none",
                outline: "none",
              }}
            />

            {/* Example chips */}
            <div
              style={{
                padding: "8px 14px",
                display: "flex",
                flexWrap: "wrap",
                gap: 6,
                borderBottom: "1px solid var(--border)",
              }}
            >
              {EXAMPLES.map((ex) => (
                <button
                  key={ex.label}
                  onClick={() => {
                    setQuery(ex.query);
                    setResults(null);
                    setError(null);
                  }}
                  style={{
                    padding: "3px 8px",
                    borderRadius: 6,
                    border: "1px solid var(--border)",
                    background: "var(--bg-2)",
                    color: "var(--text-2)",
                    fontSize: 10,
                    cursor: "pointer",
                    display: "flex",
                    alignItems: "center",
                    gap: 3,
                  }}
                >
                  <ChevronRight size={10} />
                  {ex.label}
                </button>
              ))}
            </div>

            {/* Run button */}
            <div style={{ padding: "8px 14px" }}>
              <button
                onClick={runQuery}
                disabled={running || !query.trim()}
                style={{
                  width: "100%",
                  padding: "6px 12px",
                  borderRadius: 6,
                  border: "none",
                  background: running ? "var(--bg-2)" : "var(--accent)",
                  color: "#fff",
                  fontSize: 12,
                  fontWeight: 600,
                  cursor: running ? "wait" : "pointer",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  gap: 6,
                  opacity: !query.trim() ? 0.5 : 1,
                }}
              >
                <Play size={12} />
                {running ? "Running..." : "Run Query"}
              </button>
            </div>

            {/* Error */}
            {error && (
              <div
                style={{
                  padding: "8px 14px",
                  color: "#f7768e",
                  fontSize: 11,
                  borderTop: "1px solid var(--border)",
                }}
              >
                {error}
              </div>
            )}

            {/* Results */}
            {results && (
              <div
                style={{
                  maxHeight: 200,
                  overflow: "auto",
                  borderTop: "1px solid var(--border)",
                  padding: "8px 14px",
                }}
              >
                <div
                  style={{
                    fontSize: 10,
                    color: "var(--text-3)",
                    marginBottom: 6,
                  }}
                >
                  {results.length} result{results.length !== 1 ? "s" : ""}
                </div>
                <pre
                  style={{
                    fontSize: 11,
                    color: "var(--text-1)",
                    fontFamily: "var(--font-mono)",
                    whiteSpace: "pre-wrap",
                    wordBreak: "break-all",
                    lineHeight: 1.5,
                  }}
                >
                  {JSON.stringify(results, null, 2)}
                </pre>
              </div>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}
