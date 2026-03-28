import { Search, ChevronRight } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";

export function CommandBar() {
  const { t } = useI18n();
  const activeRepo = useAppStore((s) => s.activeRepo);
  const sidebarTab = useAppStore((s) => s.sidebarTab);
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const selectedNodeName = useAppStore((s) => s.selectedNodeName);
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen);

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
        className="flex items-center rounded-lg transition-all shrink-0"
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
        onMouseEnter={(e) => {
          e.currentTarget.style.borderColor = "var(--accent)";
          e.currentTarget.style.borderImage = "linear-gradient(135deg, var(--accent), var(--purple)) 1";
          e.currentTarget.style.color = "var(--text-2)";
          e.currentTarget.style.background = "var(--bg-4)";
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.borderColor = "var(--surface-border)";
          e.currentTarget.style.borderImage = "none";
          e.currentTarget.style.color = "var(--text-3)";
          e.currentTarget.style.background = "var(--bg-3)";
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

      {/* Right: spacer */}
      <div className="w-[80px] shrink-0" />
    </div>
  );
}
