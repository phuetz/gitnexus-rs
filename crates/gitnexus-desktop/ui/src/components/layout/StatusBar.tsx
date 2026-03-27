import { useAppStore } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";

export function StatusBar() {
  const { t } = useI18n();
  const activeRepo = useAppStore((s) => s.activeRepo);
  const sidebarTab = useAppStore((s) => s.sidebarTab);
  const zoomLevel = useAppStore((s) => s.zoomLevel);

  const TAB_LABELS: Record<string, string> = {
    repos: t("sidebar.repositories"),
    files: t("sidebar.fileExplorer"),
    graph: t("sidebar.graphExplorer"),
    impact: t("sidebar.impactAnalysis"),
    docs: t("sidebar.documentation"),
    search: t("commandBar.tab.search"),
  };

  const Sep = () => (
    <div style={{ width: "1px", height: "12px", background: "var(--surface-border)" }} />
  );

  /** Contextual info that changes per page */
  const contextInfo = () => {
    if (!activeRepo) return null;
    switch (sidebarTab) {
      case "graph":
      case "search": {
        const levelName = {
          "package": t("status.packageLevel"),
          "module": t("status.moduleLevel"),
          "symbol": t("status.symbolLevel"),
        }[zoomLevel] || (zoomLevel.charAt(0).toUpperCase() + zoomLevel.slice(1) + " level");
        return (
          <span style={{ color: "var(--text-3)" }}>
            <span style={{ fontWeight: 500, color: "var(--text-2)" }}>{t("status.view")}:</span>{" "}
            {levelName}
          </span>
        );
      }
      case "files":
        return (
          <span style={{ color: "var(--text-3)" }}>
            <span style={{ fontWeight: 500, color: "var(--text-2)" }}>{t("status.browse")}:</span> {t("status.browseSourceTree")}
          </span>
        );
      case "impact":
        return (
          <span style={{ color: "var(--text-3)" }}>
            <span style={{ fontWeight: 500, color: "var(--text-2)" }}>{t("status.mode")}:</span> {t("status.modeDependencyAnalysis")}
          </span>
        );
      case "docs":
        return (
          <span style={{ color: "var(--text-3)" }}>
            <span style={{ fontWeight: 500, color: "var(--text-2)" }}>{t("status.docs")}:</span> {t("status.docsWikiViewer")}
          </span>
        );
      default:
        return null;
    }
  };

  return (
    <div
      className="flex items-center text-[10px] shrink-0 select-none"
      style={{
        height: 26,
        paddingLeft: 16,
        paddingRight: 16,
        gap: 16,
        background: "var(--bg-1)",
        borderTop: "1px solid var(--surface-border)",
        color: "var(--text-3)",
        fontFamily: "var(--font-mono)",
      }}
    >
      {activeRepo ? (
        <>
          {/* Repo status indicator */}
          <div className="flex items-center gap-1.5">
            <span
              className="w-1.5 h-1.5 rounded-full"
              style={{
                background: "var(--green)",
                animation: "pulse-subtle 2s ease-in-out infinite",
              }}
            />
            <span style={{ fontWeight: 500, color: "var(--text-2)" }}>
              {activeRepo}
            </span>
          </div>

          <Sep />

          {/* Current page */}
          <span style={{ color: "var(--text-3)" }}>
            {TAB_LABELS[sidebarTab] || sidebarTab}
          </span>

          {/* Contextual info per page */}
          {contextInfo() && (
            <>
              <Sep />
              {contextInfo()}
            </>
          )}
        </>
      ) : (
        <span style={{ color: "var(--text-3)" }}>{t("status.noRepo")}</span>
      )}

      {/* Right: version */}
      <span style={{ marginLeft: "auto", color: "var(--text-3)" }}>
        <span style={{ fontWeight: 500, color: "var(--text-2)" }}>GitNexus</span> v0.1.0
      </span>
    </div>
  );
}
