import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { Search, X, CornerDownLeft, ArrowUp, ArrowDown } from "lucide-react";
import { useSearchSymbols } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";
import { NodeIcon } from "../shared/NodeIcon";
import type { SearchResult } from "../../lib/tauri-commands";

/** Re-rank results: exact name > name-contains > path-only, preserving score within tiers */
function rankResults(results: SearchResult[], query: string): SearchResult[] {
  if (!query) return results;
  const q = query.toLowerCase();
  return [...results].sort((a, b) => {
    const aExact = a.name.toLowerCase() === q;
    const bExact = b.name.toLowerCase() === q;
    if (aExact !== bExact) return aExact ? -1 : 1;
    const aName = a.name.toLowerCase().includes(q);
    const bName = b.name.toLowerCase().includes(q);
    if (aName !== bName) return aName ? -1 : 1;
    return b.score - a.score;
  });
}

export function SearchModal() {
  const { t } = useI18n();
  const isOpen = useAppStore((s) => s.searchOpen);
  const setSearchOpen = useAppStore((s) => s.setSearchOpen);
  const storeQuery = useAppStore((s) => s.searchQuery);
  const setStoreQuery = useAppStore((s) => s.setSearchQuery);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const setSidebarTab = useAppStore((s) => s.setSidebarTab);

  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const { data: rawResults } = useSearchSymbols(query, query.length >= 1);
  const results = useMemo(
    () => (rawResults ? rankResults(rawResults, query) : undefined),
    [rawResults, query]
  );

  // Sync query from store and reset on open/close (render-time state adjustment)
  const [prevIsOpen, setPrevIsOpen] = useState(isOpen);
  if (isOpen !== prevIsOpen) {
    setPrevIsOpen(isOpen);
    if (isOpen) {
      setQuery(storeQuery || "");
      setSelectedIndex(0);
    } else {
      setStoreQuery("");
    }
  }

  // Also update when storeQuery changes while open
  const [prevStoreQuery, setPrevStoreQuery] = useState(storeQuery);
  if (storeQuery !== prevStoreQuery) {
    setPrevStoreQuery(storeQuery);
    if (isOpen && storeQuery) {
      setQuery(storeQuery);
    }
  }

  // Focus input after opening
  useEffect(() => {
    if (isOpen) {
      const timer = setTimeout(() => inputRef.current?.focus(), 50);
      return () => clearTimeout(timer);
    }
  }, [isOpen]);

  // Reset selection on results change (render-time state adjustment)
  const [prevResults, setPrevResults] = useState(results);
  if (results !== prevResults) {
    setPrevResults(results);
    setSelectedIndex(0);
  }

  // Scroll selected result into view when selectedIndex changes
  useEffect(() => {
    const el = document.querySelector(`[data-search-index="${selectedIndex}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  const selectResult = useCallback(
    (nodeId: string, name?: string) => {
      setSelectedNodeId(nodeId, name);
      setSidebarTab("graph");
      setSearchOpen(false);
    },
    [setSelectedNodeId, setSidebarTab, setSearchOpen]
  );

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, Math.max((results?.length ?? 1) - 1, 0)));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" && results?.[selectedIndex]) {
      selectResult(results[selectedIndex].nodeId, results[selectedIndex].name);
    } else if (e.key === "Escape") {
      setSearchOpen(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]"
      onClick={() => setSearchOpen(false)}
    >
      {/* Backdrop */}
      <div className="absolute inset-0" style={{ background: "rgba(0,0,0,0.6)", backdropFilter: "blur(4px)" }} />

      {/* Modal */}
      <div
        className="relative w-full max-w-[560px] rounded-xl overflow-hidden fade-in"
        style={{
          background: "var(--bg-2)",
          border: "1px solid var(--surface-border-hover)",
          boxShadow: "var(--shadow-lg), 0 0 0 1px rgba(0,0,0,0.1)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Input */}
        <div className="flex items-center gap-3 px-4 py-3" style={{ borderBottom: "1px solid var(--surface-border)" }}>
          <Search size={18} className="shrink-0" style={{ color: "var(--text-3)" }} />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t("search.placeholder")}
            className="flex-1 bg-transparent outline-none text-[15px]"
            style={{ color: "var(--text-0)", fontFamily: "var(--font-body)" }}
          />
          <button
            onClick={() => setSearchOpen(false)}
            className="p-1 rounded-md"
            style={{ color: "var(--text-3)" }}
          >
            <X size={16} />
          </button>
        </div>

        {/* Results */}
        <div className="max-h-[400px] overflow-y-auto py-1">
          {results && results.length > 0 ? (
            <div>
              {results.map((r, i) => (
                <button
                  key={r.nodeId}
                  data-search-index={i}
                  onClick={() => selectResult(r.nodeId, r.name)}
                  className="w-full flex items-center gap-3 px-4 py-2.5 text-left transition-colors"
                  style={{
                    background: i === selectedIndex ? "var(--accent-subtle)" : "transparent",
                  }}
                  onMouseEnter={() => setSelectedIndex(i)}
                >
                  <NodeIcon label={r.label} size={18} />
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span
                        className="font-medium truncate text-[13px]"
                        style={{ color: i === selectedIndex ? "var(--text-0)" : "var(--text-1)" }}
                      >
                        {r.name}
                      </span>
                      <span
                        className="badge shrink-0"
                        style={{
                          background: "var(--bg-4)",
                          color: "var(--text-3)",
                          fontSize: 10,
                        }}
                      >
                        {r.label}
                      </span>
                    </div>
                    <span className="text-[11px] truncate block" style={{ color: "var(--text-3)" }}>
                      {r.filePath}
                      {r.startLine && `:${r.startLine}`}
                    </span>
                  </div>
                  {i === selectedIndex && (
                    <CornerDownLeft size={13} style={{ color: "var(--text-3)" }} />
                  )}
                </button>
              ))}
            </div>
          ) : query.length >= 1 ? (
            <div className="py-12 text-center" style={{ color: "var(--text-3)" }}>
              {t("search.noResults")}
            </div>
          ) : (
            <div className="py-12 text-center" style={{ color: "var(--text-3)" }}>
              {t("search.startTyping")}
            </div>
          )}
        </div>

        {/* Footer hints */}
        <div
          className="flex items-center gap-4 px-4 py-2 text-[11px]"
          style={{ borderTop: "1px solid var(--surface-border)", color: "var(--text-3)" }}
          aria-label="Keyboard shortcuts: Up/Down arrows to navigate, Enter to select, Escape to close"
        >
          <span className="flex items-center gap-1">
            <ArrowUp size={11} /><ArrowDown size={11} /> {t("search.navigate")}
          </span>
          <span className="flex items-center gap-1">
            <CornerDownLeft size={11} /> {t("search.open")}
          </span>
          <span className="flex items-center gap-1">
            <kbd className="font-mono text-[10px]">Esc</kbd> {t("search.close")}
          </span>
        </div>
      </div>
    </div>
  );
}
