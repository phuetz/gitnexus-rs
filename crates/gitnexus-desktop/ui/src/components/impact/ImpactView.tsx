import { useState } from "react";
import { Zap, ArrowUp, ArrowDown, FileText } from "lucide-react";
import { useImpactAnalysis, useSearchSymbols } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";

type Direction = "upstream" | "downstream" | "both";

export function ImpactView() {
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const [targetId, setTargetId] = useState<string | null>(selectedNodeId);
  const [direction, setDirection] = useState<Direction>("both");
  const [searchQuery, setSearchQuery] = useState("");

  const { data: searchResults } = useSearchSymbols(searchQuery, searchQuery.length >= 2);
  const { data: impact, isLoading, error } = useImpactAnalysis(
    targetId,
    direction,
    5
  );

  const handleSelectTarget = (nodeId: string) => {
    setTargetId(nodeId);
    setSearchQuery("");
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="px-4 py-3 border-b border-[var(--border)] bg-[var(--bg-secondary)]">
        <h2 className="text-sm font-semibold text-[var(--text-primary)] mb-2 flex items-center gap-2">
          <Zap size={16} className="text-[var(--warning)]" />
          Impact Analysis
        </h2>

        {/* Target search */}
        <div className="relative mb-2">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search symbol to analyze..."
            className="w-full px-3 py-1.5 rounded border border-[var(--border)] bg-[var(--bg-primary)] text-[var(--text-primary)] placeholder-[var(--text-muted)] text-xs focus:outline-none focus:border-[var(--accent)]"
          />
          {searchResults && searchResults.length > 0 && searchQuery.length >= 2 && (
            <div className="absolute z-10 top-full left-0 right-0 mt-1 rounded border border-[var(--border)] bg-[var(--bg-secondary)] max-h-40 overflow-y-auto shadow-lg">
              {searchResults.map((r) => (
                <button
                  key={r.nodeId}
                  onClick={() => handleSelectTarget(r.nodeId)}
                  className="flex items-center gap-2 w-full px-3 py-1.5 text-xs text-left hover:bg-[var(--bg-tertiary)]"
                >
                  <span className="px-1 rounded bg-[var(--bg-tertiary)] text-[var(--accent)]">
                    {r.label}
                  </span>
                  <span className="truncate">{r.name}</span>
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Direction toggle */}
        <div className="flex rounded border border-[var(--border)] overflow-hidden">
          {(["upstream", "both", "downstream"] as Direction[]).map((d) => (
            <button
              key={d}
              onClick={() => setDirection(d)}
              className={`flex-1 px-2 py-1 text-[11px] transition-colors ${
                direction === d
                  ? "bg-[var(--accent)] text-white"
                  : "text-[var(--text-muted)] hover:bg-[var(--bg-tertiary)]"
              }`}
            >
              {d === "upstream" && <ArrowUp size={11} className="inline mr-1" />}
              {d === "downstream" && <ArrowDown size={11} className="inline mr-1" />}
              {d.charAt(0).toUpperCase() + d.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {/* Results */}
      <div className="flex-1 overflow-y-auto p-3">
        {!targetId && (
          <p className="text-center text-[var(--text-muted)] py-8">
            Search and select a symbol to analyze its blast radius
          </p>
        )}

        {isLoading && (
          <p className="text-center text-[var(--text-muted)] py-8">
            Analyzing impact...
          </p>
        )}

        {error && (
          <p className="text-center text-[var(--danger)] py-4">
            Error: {String(error)}
          </p>
        )}

        {impact && (
          <div className="space-y-4">
            {/* Summary */}
            <div className="grid grid-cols-3 gap-2">
              <StatCard
                label="Upstream"
                value={impact.summary.upstreamCount}
                icon={<ArrowUp size={14} />}
                color="var(--accent)"
              />
              <StatCard
                label="Downstream"
                value={impact.summary.downstreamCount}
                icon={<ArrowDown size={14} />}
                color="var(--warning)"
              />
              <StatCard
                label="Files"
                value={impact.summary.affectedFilesCount}
                icon={<FileText size={14} />}
                color="var(--success)"
              />
            </div>

            {/* Target info */}
            <div className="p-2 rounded border border-[var(--accent)] bg-[var(--accent)]/10">
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--accent)] text-white">
                {impact.target.label}
              </span>
              <span className="ml-2 font-medium">{impact.target.name}</span>
              <p className="text-[10px] text-[var(--text-muted)] mt-0.5">
                {impact.target.filePath}
              </p>
            </div>

            {/* Upstream list */}
            {impact.upstream.length > 0 && (
              <ImpactSection
                title="Upstream (callers)"
                nodes={impact.upstream}
                onSelect={(id) => {
                  setSelectedNodeId(id);
                }}
              />
            )}

            {/* Downstream list */}
            {impact.downstream.length > 0 && (
              <ImpactSection
                title="Downstream (callees)"
                nodes={impact.downstream}
                onSelect={(id) => {
                  setSelectedNodeId(id);
                }}
              />
            )}

            {/* Affected files */}
            {impact.affectedFiles.length > 0 && (
              <div>
                <h3 className="text-xs font-semibold text-[var(--text-secondary)] uppercase tracking-wider mb-1">
                  Affected Files ({impact.affectedFiles.length})
                </h3>
                <ul className="space-y-0.5 text-xs">
                  {impact.affectedFiles.map((f) => (
                    <li
                      key={f}
                      className="px-2 py-1 rounded bg-[var(--bg-secondary)] text-[var(--text-muted)] truncate"
                    >
                      {f}
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function StatCard({
  label,
  value,
  icon,
  color,
}: {
  label: string;
  value: number;
  icon: React.ReactNode;
  color: string;
}) {
  return (
    <div className="p-2 rounded border border-[var(--border)] bg-[var(--bg-secondary)] text-center">
      <div className="flex items-center justify-center gap-1 mb-1" style={{ color }}>
        {icon}
        <span className="text-lg font-bold">{value}</span>
      </div>
      <span className="text-[10px] text-[var(--text-muted)]">{label}</span>
    </div>
  );
}

function ImpactSection({
  title,
  nodes,
  onSelect,
}: {
  title: string;
  nodes: { node: { id: string; name: string; label: string; filePath: string }; depth: number }[];
  onSelect: (id: string) => void;
}) {
  return (
    <div>
      <h3 className="text-xs font-semibold text-[var(--text-secondary)] uppercase tracking-wider mb-1">
        {title} ({nodes.length})
      </h3>
      <ul className="space-y-0.5">
        {nodes.map((item) => (
          <li
            key={item.node.id}
            onClick={() => onSelect(item.node.id)}
            className="flex items-center gap-2 px-2 py-1 rounded cursor-pointer hover:bg-[var(--bg-tertiary)] transition-colors text-xs"
          >
            <span
              className="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold shrink-0"
              style={{
                backgroundColor: `hsl(${Math.max(0, 200 - item.depth * 40)}, 70%, 50%)`,
                color: "white",
              }}
            >
              {item.depth}
            </span>
            <span className="px-1 rounded bg-[var(--bg-tertiary)] text-[var(--text-muted)] text-[10px]">
              {item.node.label}
            </span>
            <span className="truncate">{item.node.name}</span>
            <span className="ml-auto text-[10px] text-[var(--text-muted)] truncate max-w-[100px]">
              {item.node.filePath}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
