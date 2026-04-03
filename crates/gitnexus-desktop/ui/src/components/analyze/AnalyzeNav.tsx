import { LayoutDashboard, Flame, Link2, Users, Shield, GitBranch, FileText, Heart } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import type { AnalyzeView } from "../../stores/app-store";

const NAV_ITEMS: { view: AnalyzeView; icon: typeof LayoutDashboard; label: string }[] = [
  { view: "overview", icon: LayoutDashboard, label: "Overview" },
  { view: "hotspots", icon: Flame, label: "Hotspots" },
  { view: "coupling", icon: Link2, label: "Coupling" },
  { view: "ownership", icon: Users, label: "Ownership" },
  { view: "coverage", icon: Shield, label: "Coverage" },
  { view: "diagram", icon: GitBranch, label: "Diagrams" },
  { view: "report", icon: FileText, label: "Report" },
  { view: "health", icon: Heart, label: "Health" },
];

export function AnalyzeNav() {
  const analyzeView = useAppStore((s) => s.analyzeView);
  const setAnalyzeView = useAppStore((s) => s.setAnalyzeView);

  return (
    <div
      className="flex flex-col h-full shrink-0 py-3"
      style={{
        width: 160,
        background: "var(--glass-bg)",
        backdropFilter: "blur(var(--glass-blur))",
        borderRight: "1px solid var(--glass-border)",
      }}
    >
      <div className="px-3 mb-3">
        <h2 className="text-xs font-semibold uppercase tracking-wider" style={{ color: "var(--text-3)", fontFamily: "var(--font-display)" }}>
          Analytics
        </h2>
      </div>
      <nav className="flex flex-col gap-0.5 px-2">
        {NAV_ITEMS.map(({ view, icon: Icon, label }) => (
          <button
            key={view}
            onClick={() => setAnalyzeView(view)}
            className="flex items-center gap-2.5 px-2.5 py-2 rounded-md text-sm transition-colors text-left"
            style={{
              background: analyzeView === view ? "var(--accent-subtle)" : "transparent",
              color: analyzeView === view ? "var(--accent)" : "var(--text-2)",
              fontFamily: "var(--font-body)",
              fontWeight: analyzeView === view ? 500 : 400,
              boxShadow: analyzeView === view ? "inset 2px 0 0 var(--accent)" : "none",
            }}
          >
            <Icon size={16} />
            <span>{label}</span>
          </button>
        ))}
      </nav>
    </div>
  );
}
