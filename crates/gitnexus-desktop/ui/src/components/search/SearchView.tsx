import { useState } from "react";
import { Search } from "lucide-react";
import { useSearchSymbols } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";

export function SearchView() {
  const [query, setQuery] = useState("");
  const { data: results, isLoading } = useSearchSymbols(query, query.length >= 2);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const setSidebarTab = useAppStore((s) => s.setSidebarTab);

  const handleSelect = (nodeId: string) => {
    setSelectedNodeId(nodeId);
    setSidebarTab("graph");
  };

  return (
    <div className="h-full flex flex-col p-4">
      {/* Search input */}
      <div className="relative mb-4">
        <Search
          size={16}
          className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--text-muted)]"
        />
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search symbols..."
          autoFocus
          className="w-full pl-9 pr-3 py-2 rounded-lg border border-[var(--border)] bg-[var(--bg-secondary)] text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--accent)]"
        />
      </div>

      {/* Results */}
      <div className="flex-1 overflow-y-auto space-y-0.5">
        {isLoading && (
          <p className="text-[var(--text-muted)] text-center py-4">
            Searching...
          </p>
        )}
        {results?.map((r) => (
          <button
            key={r.nodeId}
            onClick={() => handleSelect(r.nodeId)}
            className="flex items-center gap-2 w-full px-3 py-2 rounded hover:bg-[var(--bg-tertiary)] transition-colors text-left"
          >
            <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--bg-tertiary)] text-[var(--accent)] shrink-0">
              {r.label}
            </span>
            <span className="font-medium truncate">{r.name}</span>
            <span className="ml-auto text-[10px] text-[var(--text-muted)] truncate max-w-[200px]">
              {r.filePath}
              {r.startLine && `:${r.startLine}`}
            </span>
          </button>
        ))}
        {results && results.length === 0 && query.length >= 2 && (
          <p className="text-[var(--text-muted)] text-center py-4">
            No results found
          </p>
        )}
      </div>
    </div>
  );
}
