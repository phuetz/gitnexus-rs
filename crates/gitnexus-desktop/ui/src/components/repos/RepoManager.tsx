import { useState } from "react";
import { Database, RefreshCw, BookOpen, FileText, MoreHorizontal } from "lucide-react";
import { useRepos, useOpenRepo } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";
import { commands } from "../../lib/tauri-commands";

export function RepoManager() {
  const { data: repos, isLoading, error, refetch } = useRepos();
  const openRepo = useOpenRepo();
  const setActiveRepo = useAppStore((s) => s.setActiveRepo);
  const setSidebarTab = useAppStore((s) => s.setSidebarTab);

  const handleOpen = async (name: string) => {
    try {
      await openRepo.mutateAsync(name);
      setActiveRepo(name);
      setSidebarTab("graph");
    } catch (e) {
      console.error("Failed to open repo:", e);
    }
  };

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="pulse-subtle" style={{ color: "var(--text-3)" }}>Loading repositories...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex items-center justify-center" style={{ color: "var(--rose)" }}>
        Error: {String(error)}
      </div>
    );
  }

  if (!repos || repos.length === 0) {
    return (
      <div className="h-full flex flex-col items-center justify-center gap-5 fade-in">
        <div
          className="w-16 h-16 rounded-2xl flex items-center justify-center"
          style={{ background: "var(--bg-3)", color: "var(--text-3)" }}
        >
          <Database size={28} />
        </div>
        <div className="text-center">
          <p className="text-lg font-semibold mb-1" style={{ fontFamily: "var(--font-display)", color: "var(--text-1)" }}>
            No repositories indexed
          </p>
          <p className="text-sm" style={{ color: "var(--text-3)" }}>
            Run{" "}
            <code className="px-1.5 py-0.5 rounded-md font-mono text-xs" style={{ background: "var(--bg-3)", color: "var(--accent)" }}>
              gitnexus analyze &lt;path&gt;
            </code>{" "}
            to get started
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto p-6 fade-in">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1
            className="text-xl font-semibold"
            style={{ fontFamily: "var(--font-display)", color: "var(--text-0)" }}
          >
            Repositories
          </h1>
          <p className="text-xs mt-0.5" style={{ color: "var(--text-3)" }}>
            {repos.length} indexed {repos.length === 1 ? "repository" : "repositories"}
          </p>
        </div>
        <button
          onClick={() => refetch()}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all"
          style={{ background: "var(--bg-3)", color: "var(--text-2)", border: "1px solid var(--surface-border)" }}
          onMouseEnter={(e) => { e.currentTarget.style.borderColor = "var(--surface-border-hover)"; }}
          onMouseLeave={(e) => { e.currentTarget.style.borderColor = "var(--surface-border)"; }}
        >
          <RefreshCw size={13} /> Refresh
        </button>
      </div>

      {/* Cards */}
      <div className="grid gap-3 stagger">
        {repos.map((repo) => (
          <RepoCard
            key={repo.name}
            repo={repo}
            onOpen={() => handleOpen(repo.name)}
            isOpening={openRepo.isPending}
          />
        ))}
      </div>
    </div>
  );
}

function RepoCard({
  repo,
  onOpen,
  isOpening,
}: {
  repo: {
    name: string;
    path: string;
    indexedAt: string;
    files?: number;
    nodes?: number;
    edges?: number;
    communities?: number;
  };
  onOpen: () => void;
  isOpening: boolean;
}) {
  const [showMenu, setShowMenu] = useState(false);
  const [status, setStatus] = useState<{ text: string; type: "success" | "error" } | null>(null);
  const [busy, setBusy] = useState(false);

  const runAction = async (action: () => Promise<unknown>, label: string) => {
    setBusy(true);
    setStatus(null);
    setShowMenu(false);
    try {
      await action();
      setStatus({ text: `${label} completed`, type: "success" });
    } catch (e) {
      setStatus({ text: String(e), type: "error" });
    } finally {
      setBusy(false);
    }
  };

  // Generate a color from repo name
  const hue = repo.name.split("").reduce((a, c) => a + c.charCodeAt(0), 0) % 360;

  return (
    <div
      className="rounded-xl overflow-hidden transition-all duration-200 group"
      style={{
        background: "var(--surface)",
        border: "1px solid var(--surface-border)",
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.borderColor = "var(--surface-border-hover)";
        e.currentTarget.style.boxShadow = "var(--shadow-md)";
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.borderColor = "var(--surface-border)";
        e.currentTarget.style.boxShadow = "none";
      }}
    >
      <button
        onClick={onOpen}
        disabled={isOpening || busy}
        className="flex items-start gap-3.5 w-full p-4 text-left transition-colors"
        onMouseEnter={(e) => { e.currentTarget.style.background = "var(--surface-hover)"; }}
        onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}
      >
        {/* Avatar */}
        <div
          className="w-10 h-10 rounded-lg flex items-center justify-center text-white font-bold shrink-0"
          style={{
            background: `linear-gradient(135deg, hsl(${hue}, 60%, 45%), hsl(${hue + 40}, 50%, 35%))`,
            fontFamily: "var(--font-display)",
            fontSize: 16,
          }}
        >
          {repo.name.charAt(0).toUpperCase()}
        </div>

        <div className="flex-1 min-w-0">
          <h3 className="font-semibold text-sm" style={{ color: "var(--text-0)", fontFamily: "var(--font-display)" }}>
            {repo.name}
          </h3>
          <p className="text-[11px] truncate mt-0.5" style={{ color: "var(--text-3)", fontFamily: "var(--font-mono)" }}>
            {repo.path}
          </p>

          {/* Stats badges */}
          <div className="flex flex-wrap gap-1.5 mt-2.5">
            {repo.files != null && (
              <StatBadge value={repo.files} label="files" color="var(--accent)" />
            )}
            {repo.nodes != null && (
              <StatBadge value={formatNumber(repo.nodes)} label="nodes" color="var(--purple)" />
            )}
            {repo.edges != null && (
              <StatBadge value={formatNumber(repo.edges)} label="edges" color="var(--cyan)" />
            )}
            {repo.communities != null && (
              <StatBadge value={repo.communities} label="communities" color="var(--green)" />
            )}
          </div>
        </div>

        {/* Menu button */}
        <div className="relative shrink-0">
          <button
            onClick={(e) => {
              e.stopPropagation();
              setShowMenu(!showMenu);
            }}
            className="p-1.5 rounded-md opacity-0 group-hover:opacity-100 transition-opacity"
            style={{ color: "var(--text-3)" }}
            onMouseEnter={(e) => { e.currentTarget.style.background = "var(--bg-4)"; }}
            onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}
          >
            <MoreHorizontal size={16} />
          </button>

          {showMenu && (
            <div
              className="absolute right-0 top-8 w-48 py-1 rounded-lg z-20 fade-in"
              style={{
                background: "var(--bg-3)",
                border: "1px solid var(--surface-border-hover)",
                boxShadow: "var(--shadow-lg)",
              }}
              onMouseLeave={() => setShowMenu(false)}
            >
              <MenuItem
                icon={<RefreshCw size={13} />}
                label="Re-index"
                onClick={() => runAction(() => commands.analyzeRepo(repo.path), "Re-index")}
              />
              <div className="my-1" style={{ borderTop: "1px solid var(--surface-border)" }} />
              <MenuItem
                icon={<BookOpen size={13} />}
                label="Generate Wiki"
                onClick={() => runAction(() => commands.generateDocs("wiki", repo.path), "Wiki")}
              />
              <MenuItem
                icon={<FileText size={13} />}
                label="Generate AGENTS.md"
                onClick={() => runAction(() => commands.generateDocs("context", repo.path), "AGENTS.md")}
              />
              <MenuItem
                icon={<FileText size={13} />}
                label="Generate All"
                onClick={() => runAction(() => commands.generateDocs("all", repo.path), "All docs")}
              />
            </div>
          )}
        </div>
      </button>

      {/* Status */}
      {(status || busy) && (
        <div
          className="px-4 py-2 text-[11px] font-medium"
          style={{
            borderTop: "1px solid var(--surface-border)",
            color: busy ? "var(--text-3)" : status?.type === "success" ? "var(--green)" : "var(--rose)",
            background: busy
              ? "var(--bg-2)"
              : status?.type === "success"
                ? "var(--green-subtle)"
                : "var(--rose-subtle)",
          }}
        >
          {busy ? "Processing..." : status?.text}
        </div>
      )}
    </div>
  );
}

function MenuItem({
  icon,
  label,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={(e) => {
        e.stopPropagation();
        onClick();
      }}
      className="w-full flex items-center gap-2.5 px-3 py-1.5 text-[12px] text-left transition-colors"
      style={{ color: "var(--text-2)" }}
      onMouseEnter={(e) => {
        e.currentTarget.style.background = "var(--surface-hover)";
        e.currentTarget.style.color = "var(--text-0)";
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.background = "transparent";
        e.currentTarget.style.color = "var(--text-2)";
      }}
    >
      {icon}
      {label}
    </button>
  );
}

function StatBadge({ value, label, color }: { value: string | number; label: string; color: string }) {
  return (
    <span
      className="badge"
      style={{ background: `${color}12`, color, fontSize: 10, fontWeight: 500 }}
    >
      {value} {label}
    </span>
  );
}

function formatNumber(n: number): string {
  if (n >= 1000) return (n / 1000).toFixed(1) + "k";
  return String(n);
}
