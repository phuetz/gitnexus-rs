import { Search, ChevronRight } from "lucide-react";
import { useAppStore } from "../../stores/app-store";

export function CommandBar() {
  const activeRepo = useAppStore((s) => s.activeRepo);
  const sidebarTab = useAppStore((s) => s.sidebarTab);
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const setSearchOpen = useAppStore((s) => s.setSearchOpen);

  const tabLabels: Record<string, string> = {
    repos: "Repositories",
    search: "Search",
    files: "Files",
    graph: "Graph Explorer",
    impact: "Impact Analysis",
    docs: "Documentation",
  };

  return (
    <div
      className="flex items-center h-[40px] px-3 shrink-0 select-none"
      style={{
        background: "var(--bg-1)",
        borderBottom: "1px solid var(--surface-border)",
      }}
      data-tauri-drag-region
    >
      {/* Breadcrumb */}
      <div className="flex items-center gap-1 text-xs min-w-0">
        {activeRepo ? (
          <>
            <span style={{ color: "var(--text-2)" }}>{activeRepo}</span>
            <ChevronRight size={12} style={{ color: "var(--text-3)" }} />
            <span style={{ color: "var(--text-1)", fontWeight: 500 }}>
              {tabLabels[sidebarTab] || sidebarTab}
            </span>
            {selectedNodeId && (
              <>
                <ChevronRight size={12} style={{ color: "var(--text-3)" }} />
                <span
                  className="truncate max-w-[200px] font-mono text-[11px]"
                  style={{ color: "var(--accent)" }}
                >
                  {selectedNodeId}
                </span>
              </>
            )}
          </>
        ) : (
          <span
            style={{
              color: "var(--text-0)",
              fontFamily: "var(--font-display)",
              fontWeight: 600,
              fontSize: 13,
            }}
          >
            GitNexus
          </span>
        )}
      </div>

      {/* Center: search trigger */}
      <button
        onClick={() => setSearchOpen(true)}
        className="mx-auto flex items-center gap-2 px-3 py-1 rounded-lg transition-all"
        style={{
          background: "var(--bg-3)",
          border: "1px solid var(--surface-border)",
          color: "var(--text-3)",
          fontSize: 12,
          minWidth: 220,
        }}
        onMouseEnter={(e) => {
          e.currentTarget.style.borderColor = "var(--surface-border-hover)";
          e.currentTarget.style.color = "var(--text-2)";
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.borderColor = "var(--surface-border)";
          e.currentTarget.style.color = "var(--text-3)";
        }}
      >
        <Search size={13} />
        <span>Search symbols...</span>
        <kbd
          className="ml-auto font-mono text-[10px] px-1.5 py-0.5 rounded"
          style={{ background: "var(--bg-2)", color: "var(--text-3)" }}
        >
          Ctrl K
        </kbd>
      </button>

      {/* Right: spacer */}
      <div className="w-[100px]" />
    </div>
  );
}
