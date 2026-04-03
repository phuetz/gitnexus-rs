import { memo, useState, useEffect } from "react";
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

export const AnalyzeNav = memo(function AnalyzeNav() {
  const analyzeView = useAppStore((s) => s.analyzeView);
  const setAnalyzeView = useAppStore((s) => s.setAnalyzeView);

  const [isCompact, setIsCompact] = useState(window.innerWidth < 900);

  useEffect(() => {
    const handler = () => setIsCompact(window.innerWidth < 900);
    window.addEventListener("resize", handler);
    return () => window.removeEventListener("resize", handler);
  }, []);

  return (
    <div
      className="flex flex-col h-full shrink-0 py-3"
      style={{
        width: isCompact ? 48 : 160,
        background: "var(--glass-bg)",
        backdropFilter: "blur(var(--glass-blur))",
        borderRight: "1px solid var(--glass-border)",
        transition: "width 0.2s ease",
      }}
    >
      {!isCompact && (
        <div className="px-3 mb-3">
          <h2 className="text-xs font-semibold uppercase tracking-wider" style={{ color: "var(--text-3)", fontFamily: "var(--font-display)" }}>
            Analytics
          </h2>
        </div>
      )}
      <nav className="flex flex-col gap-0.5 px-2">
        {NAV_ITEMS.map(({ view, icon: Icon, label }) => (
          <button
            key={view}
            onClick={() => setAnalyzeView(view)}
            aria-current={analyzeView === view ? "page" : undefined}
            aria-label={isCompact ? label : undefined}
            title={isCompact ? label : undefined}
            className="flex items-center rounded-md text-sm transition-colors text-left"
            style={{
              gap: isCompact ? 0 : 10,
              padding: isCompact ? "8px 0" : "8px 10px",
              justifyContent: isCompact ? "center" : "flex-start",
              background: analyzeView === view ? "var(--accent-subtle)" : "transparent",
              color: analyzeView === view ? "var(--accent)" : "var(--text-2)",
              fontFamily: "var(--font-body)",
              fontWeight: analyzeView === view ? 500 : 400,
              boxShadow: analyzeView === view ? "inset 2px 0 0 var(--accent)" : "none",
            }}
          >
            <Icon size={16} />
            {!isCompact && <span>{label}</span>}
          </button>
        ))}
      </nav>
    </div>
  );
});
