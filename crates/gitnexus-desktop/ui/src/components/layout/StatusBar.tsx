import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { useAppStore } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";
import { commands } from "../../lib/tauri-commands";

/** Separator — extracted outside StatusBar to satisfy react-hooks/static-components. */
function Sep() {
  return <div style={{ width: "1px", height: "12px", background: "var(--surface-border)" }} />;
}

export function StatusBar() {
  const { t } = useI18n();
  const activeRepo = useAppStore((s) => s.activeRepo);
  const sidebarTab = useAppStore((s) => s.sidebarTab);
  const zoomLevel = useAppStore((s) => s.zoomLevel);

  const TAB_LABELS: Record<string, string> = useMemo(() => ({
    repos: t("sidebar.repositories"),
    files: t("sidebar.fileExplorer"),
    graph: t("sidebar.graphExplorer"),
    impact: t("sidebar.impactAnalysis"),
    docs: t("sidebar.documentation"),
    search: t("commandBar.tab.search"),
    export: t("sidebar.export"),
  }), [t]);

  const { data: chatConfig } = useQuery({
    queryKey: ["chat-config"],
    queryFn: () => commands.chatGetConfig(),
    staleTime: 30_000,
  });

  const llmConnected = useMemo(() => {
    if (!chatConfig) return false;
    if (chatConfig.provider === "ollama") return true;
    return chatConfig.apiKey.length > 0;
  }, [chatConfig]);

  const llmModelName = chatConfig?.model || null;

  /** Contextual info that changes per page */
  const ctxInfo = useMemo(() => {
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
      case "export":
        return (
          <span style={{ color: "var(--text-3)" }}>
            <span style={{ fontWeight: 500, color: "var(--text-2)" }}>Export:</span> DOCX & ASP.NET
          </span>
        );
      default:
        return null;
    }
  }, [activeRepo, sidebarTab, zoomLevel, t]);

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
          {ctxInfo && (
            <>
              <Sep />
              {ctxInfo}
            </>
          )}
        </>
      ) : (
        <span style={{ color: "var(--text-3)" }}>{t("status.noRepo")}</span>
      )}

      {/* Right: LLM status + version */}
      <div className="flex items-center gap-3" style={{ marginLeft: "auto" }}>
        {/* LLM status indicator */}
        <div className="flex items-center gap-1.5">
          <span
            className="rounded-full shrink-0"
            style={{
              width: 7,
              height: 7,
              background: llmConnected ? "var(--green)" : "var(--rose)",
            }}
          />
          {llmModelName && (
            <span style={{ color: "var(--text-3)" }}>{llmModelName}</span>
          )}
        </div>

        <Sep />

        <span style={{ color: "var(--text-3)" }}>
          <span style={{ fontWeight: 500, color: "var(--text-2)" }}>GitNexus</span> v0.1.0
        </span>
      </div>
    </div>
  );
}
