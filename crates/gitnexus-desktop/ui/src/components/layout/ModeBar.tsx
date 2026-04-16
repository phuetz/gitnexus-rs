import { memo } from "react";
import { Compass, BarChart3, MessageSquare, Settings, Search, ChevronsLeft, ChevronsRight } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useAppStore } from "../../stores/app-store";
import type { AppMode } from "../../stores/app-store";
import { commands } from "../../lib/tauri-commands";
import { Tooltip } from "../shared/Tooltip";
import { useI18n } from "../../hooks/use-i18n";

const modes: { mode: AppMode; icon: typeof Compass; i18nKey: string; shortcut: string }[] = [
  { mode: "explorer", icon: Compass, i18nKey: "mode.explorer", shortcut: "Ctrl+1" },
  { mode: "analyze", icon: BarChart3, i18nKey: "mode.analyze", shortcut: "Ctrl+2" },
  { mode: "chat", icon: MessageSquare, i18nKey: "mode.chat", shortcut: "Ctrl+3" },
  { mode: "manage", icon: Settings, i18nKey: "mode.manage", shortcut: "Ctrl+4" },
];

function Badge({ value, color }: { value: string; color: string }) {
  return (
    <span
      style={{
        fontSize: 9,
        fontWeight: 700,
        padding: "1px 5px",
        borderRadius: 8,
        background: `${color}20`,
        color,
        lineHeight: "14px",
        flexShrink: 0,
      }}
    >
      {value}
    </span>
  );
}

export const ModeBar = memo(function ModeBar() {
  const { t } = useI18n();
  const mode = useAppStore((s) => s.mode);
  const setMode = useAppStore((s) => s.setMode);
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen);
  const expanded = useAppStore((s) => s.sidebarExpanded);
  const toggleSidebar = useAppStore((s) => s.toggleSidebar);
  const activeRepo = useAppStore((s) => s.activeRepo);

  // Fetch health score for badge.
  // Scope by `activeRepo` so the badge refetches after a repo switch instead
  // of flashing the stale grade of the previously-open repo for up to 60 s.
  const { data: health } = useQuery({
    queryKey: ["code-health-badge", activeRepo],
    queryFn: () => commands.getCodeHealth(),
    staleTime: 60_000,
    enabled: !!activeRepo,
  });

  // Fetch LLM config for chat badge
  const { data: chatConfig } = useQuery({
    queryKey: ["chat-config"],
    queryFn: () => commands.chatGetConfig(),
    staleTime: 30_000,
  });

  const llmConnected = chatConfig?.apiKey
    ? chatConfig.apiKey.length > 0
    : chatConfig?.provider === "ollama";

  const healthGrade = health?.grade;

  return (
    <div
      className="flex flex-col items-start py-2 h-full shrink-0"
      style={{
        width: expanded ? 180 : 48,
        background: "var(--mode-bar-bg)",
        borderRight: "1px solid var(--surface-border)",
        transition: "width var(--transition-slow)",
        overflow: "hidden",
      }}
    >
      {/* Mode buttons */}
      {modes.map(({ mode: m, icon: Icon, i18nKey, shortcut }) => {
        const active = mode === m;
        const label = t(i18nKey);

        // Per-mode badge
        let badge = null;
        if (m === "analyze" && healthGrade) {
          const color = healthGrade.startsWith("A") ? "#4ade80" : healthGrade.startsWith("B") ? "#fbbf24" : "#fb7185";
          badge = <Badge value={healthGrade} color={color} />;
        }
        if (m === "chat") {
          badge = (
            <span
              style={{
                width: 7,
                height: 7,
                borderRadius: "50%",
                background: llmConnected ? "#4ade80" : "#fb7185",
                flexShrink: 0,
              }}
            />
          );
        }

        const button = (
          <button
            key={m}
            onClick={() => setMode(m)}
            className="relative flex items-center w-full mb-0.5 rounded-lg transition-colors focus-visible:ring-2 focus-visible:ring-[var(--accent)] focus-visible:outline-none"
            style={{
              height: 40,
              paddingLeft: expanded ? 14 : 0,
              paddingRight: expanded ? 10 : 0,
              justifyContent: expanded ? "flex-start" : "center",
              gap: 10,
              color: active ? "var(--accent)" : "var(--text-3)",
              background: active ? "var(--accent-subtle)" : "transparent",
              boxShadow: active ? "var(--glow-accent, 0 0 8px var(--accent))" : "none",
              border: "none",
              cursor: "pointer",
              marginLeft: expanded ? 4 : 4,
              marginRight: expanded ? 4 : 4,
            }}
            aria-label={label}
            aria-current={active ? "page" : undefined}
          >
            {active && (
              <div
                className="absolute left-0 top-1/2 -translate-y-1/2 rounded-r"
                style={{ width: 3, height: 20, background: "var(--accent)" }}
              />
            )}
            <Icon size={20} style={{ flexShrink: 0 }} />
            {expanded && (
              <>
                <span
                  style={{
                    fontSize: 13,
                    fontWeight: active ? 600 : 500,
                    fontFamily: "var(--font-body)",
                    whiteSpace: "nowrap",
                    overflow: "hidden",
                    flex: 1,
                    textAlign: "left",
                  }}
                >
                  {label}
                </span>
                {badge}
              </>
            )}
          </button>
        );

        return expanded ? (
          <div key={m}>{button}</div>
        ) : (
          <Tooltip key={m} content={`${label} (${shortcut})`}>
            <div style={{ position: "relative" }}>
              {button}
              {/* Collapsed badge overlay */}
              {badge && (
                <div style={{ position: "absolute", top: 4, right: 4, pointerEvents: "none" }}>
                  {badge}
                </div>
              )}
            </div>
          </Tooltip>
        );
      })}

      <div className="flex-1" />

      {/* Command palette */}
      <Tooltip content={`${t("mode.commandPalette")} (Ctrl+K)`} placement={expanded ? "right" : "right"}>
        <button
          onClick={() => setCommandPaletteOpen(true)}
          className="flex items-center w-full rounded-lg transition-colors"
          style={{
            height: 36,
            paddingLeft: expanded ? 14 : 0,
            paddingRight: expanded ? 10 : 0,
            justifyContent: expanded ? "flex-start" : "center",
            gap: 10,
            color: "var(--text-3)",
            background: "transparent",
            border: "none",
            cursor: "pointer",
            marginLeft: 4,
            marginRight: 4,
            marginBottom: 2,
          }}
          aria-label={t("mode.commandPalette")}
        >
          <Search size={18} style={{ flexShrink: 0 }} />
          {expanded && (
            <span style={{ fontSize: 12, whiteSpace: "nowrap", color: "var(--text-3)" }}>
              Ctrl+K
            </span>
          )}
        </button>
      </Tooltip>

      {/* Toggle expand/collapse */}
      <button
        onClick={toggleSidebar}
        className="flex items-center w-full rounded-lg transition-colors"
        style={{
          height: 32,
          justifyContent: expanded ? "flex-start" : "center",
          paddingLeft: expanded ? 14 : 0,
          gap: 8,
          color: "var(--text-4)",
          background: "transparent",
          border: "none",
          cursor: "pointer",
          marginLeft: 4,
          marginRight: 4,
          marginBottom: 4,
        }}
        aria-label={expanded ? t("sidebar.collapse") : t("sidebar.expand")}
      >
        {expanded ? <ChevronsLeft size={16} /> : <ChevronsRight size={16} />}
        {expanded && (
          <span style={{ fontSize: 11, color: "var(--text-4)", whiteSpace: "nowrap" }}>
            {t("mode.collapse")}
          </span>
        )}
      </button>
    </div>
  );
});
