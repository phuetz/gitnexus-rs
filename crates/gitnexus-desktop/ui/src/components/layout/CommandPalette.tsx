import { useMemo } from "react";
import { Command } from "cmdk";
import {
  Compass,
  BarChart3,
  MessageSquare,
  Settings,
  LayoutDashboard,
  Flame,
  Link2,
  Users,
  Shield,
  GitBranch,
  FileText,
  Heart,
  Network,
  Zap,
  Eye,
  Code2,
  Layers,
  Sparkles,
  Workflow,
} from "lucide-react";
import { AnimatedModal } from "../shared/motion";
import { useAppStore } from "../../stores/app-store";
import { useChatStore } from "../../stores/chat-store";
import { useI18n } from "../../hooks/use-i18n";

type CommandItem = {
  id: string;
  label: string;
  group: string;
  icon: React.ComponentType<{ className?: string; size?: number; style?: React.CSSProperties }>;
  shortcut?: string;
  action: () => void;
};

function buildCommands(t: (key: string) => string): CommandItem[] {
  const store = useAppStore.getState();
  const chatStore = useChatStore.getState();

  return [
    // Mode switching
    {
      id: "mode-explorer",
      label: t("cmd.switchTo") + " " + t("mode.explorer"),
      group: t("cmd.group.modes"),
      icon: Compass,
      shortcut: "Ctrl+1",
      action: () => store.setMode("explorer"),
    },
    {
      id: "mode-analyze",
      label: t("cmd.switchTo") + " " + t("mode.analyze"),
      group: t("cmd.group.modes"),
      icon: BarChart3,
      shortcut: "Ctrl+2",
      action: () => store.setMode("analyze"),
    },
    {
      id: "mode-chat",
      label: t("cmd.switchTo") + " " + t("mode.chat"),
      group: t("cmd.group.modes"),
      icon: MessageSquare,
      shortcut: "Ctrl+3",
      action: () => store.setMode("chat"),
    },
    {
      id: "mode-manage",
      label: t("cmd.switchTo") + " " + t("mode.manage"),
      group: t("cmd.group.modes"),
      icon: Settings,
      shortcut: "Ctrl+4",
      action: () => store.setMode("manage"),
    },

    // Analyze sub-views
    {
      id: "view-overview",
      label: t("cmd.view") + " " + t("analyze.nav.overview"),
      group: t("cmd.group.analyzeViews"),
      icon: LayoutDashboard,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("overview"); },
    },
    {
      id: "view-processes",
      label: t("cmd.view") + " " + t("analyze.nav.processes"),
      group: t("cmd.group.analyzeViews"),
      icon: Workflow,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("processes"); },
    },
    {
      id: "view-hotspots",
      label: t("cmd.view") + " " + t("analyze.nav.hotspots"),
      group: t("cmd.group.analyzeViews"),
      icon: Flame,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("hotspots"); },
    },
    {
      id: "view-coupling",
      label: t("cmd.view") + " " + t("analyze.nav.coupling"),
      group: t("cmd.group.analyzeViews"),
      icon: Link2,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("coupling"); },
    },
    {
      id: "view-ownership",
      label: t("cmd.view") + " " + t("analyze.nav.ownership"),
      group: t("cmd.group.analyzeViews"),
      icon: Users,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("ownership"); },
    },
    {
      id: "view-coverage",
      label: t("cmd.view") + " " + t("analyze.nav.coverage"),
      group: t("cmd.group.analyzeViews"),
      icon: Shield,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("coverage"); },
    },
    {
      id: "view-diagram",
      label: t("cmd.view") + " " + t("analyze.nav.diagrams"),
      group: t("cmd.group.analyzeViews"),
      icon: GitBranch,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("diagram"); },
    },
    {
      id: "view-report",
      label: t("cmd.view") + " " + t("analyze.nav.report"),
      group: t("cmd.group.analyzeViews"),
      icon: FileText,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("report"); },
    },
    {
      id: "view-health",
      label: t("cmd.view") + " " + t("analyze.nav.health"),
      group: t("cmd.group.analyzeViews"),
      icon: Heart,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("health"); },
    },

    // Lens switching
    {
      id: "lens-all",
      label: t("cmd.lens") + " " + t("lens.all"),
      group: t("cmd.group.lenses"),
      icon: Eye,
      action: () => store.setActiveLens("all"),
    },
    {
      id: "lens-calls",
      label: t("cmd.lens") + " " + t("lens.calls"),
      group: t("cmd.group.lenses"),
      icon: Network,
      action: () => store.setActiveLens("calls"),
    },
    {
      id: "lens-structure",
      label: t("cmd.lens") + " " + t("lens.structure"),
      group: t("cmd.group.lenses"),
      icon: Layers,
      action: () => store.setActiveLens("structure"),
    },
    {
      id: "lens-heritage",
      label: t("cmd.lens") + " " + t("lens.heritage"),
      group: t("cmd.group.lenses"),
      icon: GitBranch,
      action: () => store.setActiveLens("heritage"),
    },
    {
      id: "lens-impact",
      label: t("cmd.lens") + " " + t("lens.impact"),
      group: t("cmd.group.lenses"),
      icon: Zap,
      action: () => store.setActiveLens("impact"),
    },
    {
      id: "lens-dead-code",
      label: t("cmd.lens") + " " + t("lens.deadCode"),
      group: t("cmd.group.lenses"),
      icon: Code2,
      action: () => store.setActiveLens("dead-code"),
    },
    {
      id: "lens-tracing",
      label: t("cmd.lens") + " " + t("lens.tracing"),
      group: t("cmd.group.lenses"),
      icon: Sparkles,
      action: () => store.setActiveLens("tracing"),
    },
    {
      id: "lens-hotspots",
      label: t("cmd.lens") + " " + t("lens.hotspots"),
      group: t("cmd.group.lenses"),
      icon: Flame,
      action: () => store.setActiveLens("hotspots"),
    },

    // Actions
    {
      id: "open-settings",
      label: t("cmd.openSettings"),
      group: t("cmd.group.actions"),
      icon: Settings,
      action: () => store.setSettingsOpen(true),
    },
    {
      id: "toggle-deep-research",
      label: t("cmd.toggleDeepResearch"),
      group: t("cmd.group.actions"),
      icon: Sparkles,
      shortcut: "Ctrl+Shift+D",
      action: () => chatStore.toggleDeepResearch(),
    },
  ];
}

function CommandPaletteItem({ item, onSelect }: { item: CommandItem; onSelect: () => void }) {
  return (
    <Command.Item
      key={item.id}
      value={`${item.label} ${item.group}`}
      onSelect={onSelect}
      className="command-palette-item hover:bg-[var(--surface-hover)] hover:text-[var(--text-0)]"
      style={{
        display: "flex",
        alignItems: "center",
        gap: 12,
        padding: "10px 12px",
        borderRadius: 8,
        cursor: "pointer",
        color: "var(--text-1)",
        fontSize: 13,
        fontWeight: 500,
        transition: "background 0.1s, color 0.1s",
      }}
    >
      <item.icon size={16} style={{ color: "var(--text-3)", flexShrink: 0 }} />
      <span style={{ flex: 1 }}>{item.label}</span>
      {item.shortcut && (
        <kbd
          style={{
            fontSize: 10,
            fontFamily: "var(--font-mono)",
            padding: "2px 6px",
            borderRadius: 4,
            background: "var(--bg-3)",
            color: "var(--text-3)",
            border: "1px solid var(--surface-border)",
          }}
        >
          {item.shortcut}
        </kbd>
      )}
    </Command.Item>
  );
}

export function CommandPalette() {
  const { t } = useI18n();
  const commandPaletteOpen = useAppStore((s) => s.commandPaletteOpen);
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen);

  const close = () => setCommandPaletteOpen(false);

  // commandPaletteOpen is an intentional cache-bust dep — rebuild commands each time the palette opens
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const cmds = useMemo(() => buildCommands(t), [commandPaletteOpen, t]);

  // Group items — use translated group labels
  const groups = [t("cmd.group.modes"), t("cmd.group.analyzeViews"), t("cmd.group.lenses"), t("cmd.group.actions")];

  return (
    <AnimatedModal isOpen={commandPaletteOpen} onClose={close}>
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
        className="rounded-xl overflow-hidden"
        style={{
          width: "min(560px, calc(100vw - 32px))",
          background: "var(--bg-1)",
          border: "1px solid var(--surface-border)",
          backdropFilter: "blur(16px)",
          boxShadow: "0 25px 50px -12px rgba(0, 0, 0, 0.5)",
        }}
      >
        <Command label={t("mode.commandPalette")} style={{ background: "transparent" }}>
          {/* Input */}
          <div
            style={{
              padding: "12px 16px",
              borderBottom: "1px solid var(--surface-border)",
            }}
          >
            <Command.Input
              placeholder={t("cmd.placeholder")}
              aria-label={t("cmd.placeholder")}
              autoFocus
              style={{
                width: "100%",
                background: "transparent",
                border: "none",
                color: "var(--text-0)",
                fontSize: 15,
                fontFamily: "var(--font-body)",
                caretColor: "var(--accent)",
              }}
            />
          </div>

          {/* List */}
          <Command.List
            style={{
              maxHeight: 400,
              overflowY: "auto",
              padding: "8px",
            }}
          >
            <Command.Empty
              style={{
                padding: "32px 16px",
                textAlign: "center",
                color: "var(--text-3)",
                fontSize: 13,
              }}
            >
              {t("search.noResults")}
            </Command.Empty>

            {groups.map((group) => {
              const items = cmds.filter((c) => c.group === group);
              if (items.length === 0) return null;
              return (
                <Command.Group key={group} heading={group} style={{ marginBottom: 4 }}>
                  {items.map((item) => (
                    <CommandPaletteItem
                      key={item.id}
                      item={item}
                      onSelect={() => {
                        item.action();
                        close();
                      }}
                    />
                  ))}
                </Command.Group>
              );
            })}
          </Command.List>

          {/* Footer */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              gap: 16,
              padding: "10px 16px",
              borderTop: "1px solid var(--surface-border)",
              color: "var(--text-3)",
              fontSize: 11,
              fontFamily: "var(--font-mono)",
            }}
          >
            <span>
              <kbd style={{ padding: "1px 4px", borderRadius: 3, background: "var(--bg-3)", border: "1px solid var(--surface-border)" }}>↑↓</kbd>
              {" "}{t("search.navigate")}
            </span>
            <span>
              <kbd style={{ padding: "1px 4px", borderRadius: 3, background: "var(--bg-3)", border: "1px solid var(--surface-border)" }}>↵</kbd>
              {" "}{t("search.open")}
            </span>
            <span>
              <kbd style={{ padding: "1px 4px", borderRadius: 3, background: "var(--bg-3)", border: "1px solid var(--surface-border)" }}>Esc</kbd>
              {" "}{t("search.close")}
            </span>
            <span style={{ marginLeft: "auto" }}>
              <kbd style={{ padding: "1px 4px", borderRadius: 3, background: "var(--bg-3)", border: "1px solid var(--surface-border)" }}>Ctrl+K</kbd>
              {" "}{t("mode.commandPalette")}
            </span>
          </div>
        </Command>
      </div>
    </AnimatedModal>
  );
}
