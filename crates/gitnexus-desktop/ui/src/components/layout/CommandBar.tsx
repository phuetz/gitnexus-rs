import { useEffect } from "react";
import { Search, ChevronRight, ArrowLeft, ArrowRight, Sun, Moon } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";

export function CommandBar() {
  const { t } = useI18n();
  const activeRepo = useAppStore((s) => s.activeRepo);
  const sidebarTab = useAppStore((s) => s.sidebarTab);
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const selectedNodeName = useAppStore((s) => s.selectedNodeName);
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen);
  const canGoBack = useAppStore((s) => s.canGoBack);
  const canGoForward = useAppStore((s) => s.canGoForward);
  const goBack = useAppStore((s) => s.goBack);
  const goForward = useAppStore((s) => s.goForward);
  const theme = useAppStore((s) => s.theme);
  const setTheme = useAppStore((s) => s.setTheme);

  // Keyboard shortcuts: Alt+Left / Alt+Right for navigation
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.altKey && e.key === "ArrowLeft") {
        e.preventDefault();
        goBack();
      } else if (e.altKey && e.key === "ArrowRight") {
        e.preventDefault();
        goForward();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [goBack, goForward]);

  const tabLabels: Record<string, string> = {
    repos: t("commandBar.tab.repos"),
    search: t("commandBar.tab.search"),
    files: t("commandBar.tab.files"),
    graph: t("commandBar.tab.graph"),
    impact: t("commandBar.tab.impact"),
    docs: t("commandBar.tab.docs"),
  };

  return (
    <div
      className="flex items-center shrink-0 select-none"
      style={{
        height: 46,
        paddingLeft: 16,
        paddingRight: 16,
        gap: 16,
        background: "var(--bg-1)",
        borderBottom: "1px solid var(--surface-border)",
      }}
      data-tauri-drag-region
    >
      {/* Navigation Back/Forward */}
      {activeRepo && (
        <div className="flex items-center" style={{ gap: 2 }}>
          <button
            onClick={goBack}
            disabled={!canGoBack}
            aria-label="Go back (Alt+←)"
            className="rounded-md flex items-center justify-center transition-colors"
            style={{
              width: 28,
              height: 28,
              color: canGoBack ? "var(--text-1)" : "var(--text-4)",
              background: "transparent",
              border: "none",
              cursor: canGoBack ? "pointer" : "default",
              opacity: canGoBack ? 1 : 0.3,
            }}
          >
            <ArrowLeft size={14} />
          </button>
          <button
            onClick={goForward}
            disabled={!canGoForward}
            aria-label="Go forward (Alt+→)"
            className="rounded-md flex items-center justify-center transition-colors"
            style={{
              width: 28,
              height: 28,
              color: canGoForward ? "var(--text-1)" : "var(--text-4)",
              background: "transparent",
              border: "none",
              cursor: canGoForward ? "pointer" : "default",
              opacity: canGoForward ? 1 : 0.3,
            }}
          >
            <ArrowRight size={14} />
          </button>
        </div>
      )}

      {/* Breadcrumb */}
      <div className="flex items-center text-xs min-w-0 flex-1" style={{ gap: 8 }}>
        {activeRepo ? (
          <>
            {/* Repo indicator with dot */}
            <div className="flex items-center gap-1.5">
              <span
                className="w-1.5 h-1.5 rounded-full"
                style={{
                  background: "var(--green)",
                  boxShadow: "0 0 8px var(--green)",
                }}
              />
              <span
                style={{
                  color: "var(--text-2)",
                  fontWeight: 500,
                  maxWidth: "clamp(80px, 15vw, 200px)",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {activeRepo}
              </span>
            </div>

            {/* Chevron separator */}
            <ChevronRight size={12} style={{ color: "var(--text-3)" }} />

            {/* Tab chip */}
            <div
              className="rounded-md"
              style={{
                paddingLeft: 8,
                paddingRight: 8,
                paddingTop: 4,
                paddingBottom: 4,
                background: "var(--accent-subtle)",
                color: "var(--accent)",
                fontSize: 11,
                fontWeight: 500,
              }}
            >
              {tabLabels[sidebarTab] || sidebarTab}
            </div>

            {/* Selected node */}
            {selectedNodeId && (
              <>
                <ChevronRight size={12} style={{ color: "var(--text-3)" }} />
                <div
                  className="rounded-md max-w-[200px] truncate text-[11px]"
                  style={{
                    paddingLeft: 8,
                    paddingRight: 8,
                    paddingTop: 4,
                    paddingBottom: 4,
                    background: "var(--purple)",
                    color: "var(--bg-0)",
                    fontWeight: 600,
                    fontFamily: "var(--font-mono)",
                  }}
                >
                  {selectedNodeName || selectedNodeId}
                </div>
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
        onClick={() => setCommandPaletteOpen(true)}
        aria-label={t("search.ariaLabel")}
        className="flex items-center rounded-lg shrink-0 hover-cmd-search"
        style={{
          gap: 8,
          paddingLeft: 12,
          paddingRight: 12,
          paddingTop: 6,
          paddingBottom: 6,
          background: "var(--bg-3)",
          border: "1px solid var(--surface-border)",
          color: "var(--text-3)",
          fontSize: 12,
          minWidth: 220,
        }}
      >
        <Search size={13} />
        <span>{t("search.placeholder")}</span>
        <kbd
          className="font-mono text-[10px] rounded"
          style={{ marginLeft: "auto", paddingLeft: 6, paddingRight: 6, paddingTop: 2, paddingBottom: 2, background: "var(--bg-2)", color: "var(--text-3)" }}
        >
          {t("search.shortcut")}
        </kbd>
      </button>

      {/* Theme toggle */}
      <button
        onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
        className="p-1.5 rounded-md transition-colors"
        style={{ color: "var(--text-3)" }}
        title={`Switch to ${theme === "dark" ? "light" : "dark"} mode`}
      >
        {theme === "dark" ? <Sun size={14} /> : <Moon size={14} />}
      </button>

      {/* Right: spacer */}
      <div className="w-[80px] shrink-0" />
    </div>
  );
}
