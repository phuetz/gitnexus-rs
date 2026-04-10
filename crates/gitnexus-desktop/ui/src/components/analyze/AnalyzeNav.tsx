import { memo, useState, useEffect } from "react";
import { LayoutDashboard, Flame, Link2, Users, Shield, GitBranch, FileText, Heart, Workflow } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import type { AnalyzeView } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";

const NAV_ITEMS: { view: AnalyzeView; icon: typeof LayoutDashboard; i18nKey: string }[] = [
  { view: "overview", icon: LayoutDashboard, i18nKey: "analyze.nav.overview" },
  { view: "processes", icon: Workflow, i18nKey: "analyze.nav.processes" },
  { view: "hotspots", icon: Flame, i18nKey: "analyze.nav.hotspots" },
  { view: "coupling", icon: Link2, i18nKey: "analyze.nav.coupling" },
  { view: "ownership", icon: Users, i18nKey: "analyze.nav.ownership" },
  { view: "coverage", icon: Shield, i18nKey: "analyze.nav.coverage" },
  { view: "diagram", icon: GitBranch, i18nKey: "analyze.nav.diagrams" },
  { view: "report", icon: FileText, i18nKey: "analyze.nav.report" },
  { view: "health", icon: Heart, i18nKey: "analyze.nav.health" },
];

export const AnalyzeNav = memo(function AnalyzeNav() {
  const { t } = useI18n();
  const analyzeView = useAppStore((s) => s.analyzeView);
  const setAnalyzeView = useAppStore((s) => s.setAnalyzeView);

  const [isCompact, setIsCompact] = useState(window.innerWidth < 900);

  useEffect(() => {
    let timeout: ReturnType<typeof setTimeout>;
    const handler = () => {
      clearTimeout(timeout);
      timeout = setTimeout(() => setIsCompact(window.innerWidth < 900), 150);
    };
    window.addEventListener("resize", handler);
    return () => { window.removeEventListener("resize", handler); clearTimeout(timeout); };
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
            {t("analyze.nav.title")}
          </h2>
        </div>
      )}
      <nav className="flex flex-col gap-0.5 px-2" aria-label={t("analyze.nav.title")}>
        {NAV_ITEMS.map(({ view, icon: Icon, i18nKey }) => {
          const label = t(i18nKey);
          return (
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
          );
        })}
      </nav>
    </div>
  );
});
