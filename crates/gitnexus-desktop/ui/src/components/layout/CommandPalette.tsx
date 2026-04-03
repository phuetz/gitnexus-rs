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
} from "lucide-react";
import { AnimatedModal } from "../shared/motion";
import { useAppStore } from "../../stores/app-store";
import { useChatStore } from "../../stores/chat-store";

type CommandItem = {
  id: string;
  label: string;
  group: string;
  icon: typeof Compass;
  shortcut?: string;
  action: () => void;
};

function buildCommands(): CommandItem[] {
  const store = useAppStore.getState();
  const chatStore = useChatStore.getState();

  return [
    // Mode switching
    {
      id: "mode-explorer",
      label: "Switch to Explorer",
      group: "Modes",
      icon: Compass,
      shortcut: "Ctrl+1",
      action: () => store.setMode("explorer"),
    },
    {
      id: "mode-analyze",
      label: "Switch to Analyze",
      group: "Modes",
      icon: BarChart3,
      shortcut: "Ctrl+2",
      action: () => store.setMode("analyze"),
    },
    {
      id: "mode-chat",
      label: "Switch to Chat",
      group: "Modes",
      icon: MessageSquare,
      shortcut: "Ctrl+3",
      action: () => store.setMode("chat"),
    },
    {
      id: "mode-manage",
      label: "Switch to Manage",
      group: "Modes",
      icon: Settings,
      shortcut: "Ctrl+4",
      action: () => store.setMode("manage"),
    },

    // Analyze sub-views
    {
      id: "view-overview",
      label: "View Overview",
      group: "Analyze Views",
      icon: LayoutDashboard,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("overview"); },
    },
    {
      id: "view-hotspots",
      label: "View Hotspots",
      group: "Analyze Views",
      icon: Flame,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("hotspots"); },
    },
    {
      id: "view-coupling",
      label: "View Coupling",
      group: "Analyze Views",
      icon: Link2,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("coupling"); },
    },
    {
      id: "view-ownership",
      label: "View Ownership",
      group: "Analyze Views",
      icon: Users,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("ownership"); },
    },
    {
      id: "view-coverage",
      label: "View Coverage",
      group: "Analyze Views",
      icon: Shield,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("coverage"); },
    },
    {
      id: "view-diagram",
      label: "View Diagrams",
      group: "Analyze Views",
      icon: GitBranch,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("diagram"); },
    },
    {
      id: "view-report",
      label: "View Report",
      group: "Analyze Views",
      icon: FileText,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("report"); },
    },
    {
      id: "view-health",
      label: "View Health",
      group: "Analyze Views",
      icon: Heart,
      action: () => { store.setMode("analyze"); store.setAnalyzeView("health"); },
    },

    // Lens switching
    {
      id: "lens-all",
      label: "Lens: All",
      group: "Lenses",
      icon: Eye,
      action: () => store.setActiveLens("all"),
    },
    {
      id: "lens-calls",
      label: "Lens: Call Graph",
      group: "Lenses",
      icon: Network,
      action: () => store.setActiveLens("calls"),
    },
    {
      id: "lens-structure",
      label: "Lens: Structure",
      group: "Lenses",
      icon: Layers,
      action: () => store.setActiveLens("structure"),
    },
    {
      id: "lens-heritage",
      label: "Lens: Heritage",
      group: "Lenses",
      icon: GitBranch,
      action: () => store.setActiveLens("heritage"),
    },
    {
      id: "lens-impact",
      label: "Lens: Impact",
      group: "Lenses",
      icon: Zap,
      action: () => store.setActiveLens("impact"),
    },
    {
      id: "lens-dead-code",
      label: "Lens: Dead Code",
      group: "Lenses",
      icon: Code2,
      action: () => store.setActiveLens("dead-code"),
    },
    {
      id: "lens-tracing",
      label: "Lens: Tracing",
      group: "Lenses",
      icon: Sparkles,
      action: () => store.setActiveLens("tracing"),
    },

    // Actions
    {
      id: "open-settings",
      label: "Open Settings",
      group: "Actions",
      icon: Settings,
      action: () => store.setSettingsOpen(true),
    },
    {
      id: "toggle-deep-research",
      label: "Toggle Deep Research",
      group: "Actions",
      icon: Sparkles,
      shortcut: "Ctrl+Shift+D",
      action: () => chatStore.toggleDeepResearch(),
    },
  ];
}

function CommandItem({ item, onSelect }: { item: CommandItem; onSelect: () => void }) {
  return (
    <Command.Item
      key={item.id}
      value={`${item.label} ${item.group}`}
      onSelect={onSelect}
      className="command-palette-item"
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
      onMouseEnter={(e) => {
        e.currentTarget.style.background = "var(--surface-hover)";
        e.currentTarget.style.color = "var(--text-0)";
      }}
      onMouseLeave={(e) => {
        if (e.currentTarget.getAttribute("aria-selected") !== "true") {
          e.currentTarget.style.background = "transparent";
          e.currentTarget.style.color = "var(--text-1)";
        }
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

const GROUP_HEADING_STYLE: React.CSSProperties = {
  fontSize: 11,
  fontWeight: 600,
  color: "var(--text-3)",
  textTransform: "uppercase",
  letterSpacing: "0.05em",
  padding: "8px 12px 4px",
  fontFamily: "var(--font-display)",
};

export function CommandPalette() {
  const commandPaletteOpen = useAppStore((s) => s.commandPaletteOpen);
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen);

  const close = () => setCommandPaletteOpen(false);

  const commands = useMemo(() => buildCommands(), []);

  // Group items
  const groups = ["Modes", "Analyze Views", "Lenses", "Actions"] as const;

  return (
    <AnimatedModal isOpen={commandPaletteOpen} onClose={close}>
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
        className="w-[560px] rounded-xl overflow-hidden"
        style={{
          background: "var(--bg-1)",
          border: "1px solid var(--surface-border)",
          backdropFilter: "blur(16px)",
          boxShadow: "0 25px 50px -12px rgba(0, 0, 0, 0.5)",
        }}
      >
        <Command label="Command Palette" style={{ background: "transparent" }}>
          {/* Input */}
          <div
            style={{
              padding: "12px 16px",
              borderBottom: "1px solid var(--surface-border)",
            }}
          >
            <Command.Input
              placeholder="Type a command or search..."
              aria-label="Type a command or search"
              autoFocus
              style={{
                width: "100%",
                background: "transparent",
                border: "none",
                outline: "none",
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
              No results found.
            </Command.Empty>

            {groups.map((group) => {
              const items = commands.filter((c) => c.group === group);
              if (items.length === 0) return null;
              return (
                <Command.Group key={group} heading={group} style={{ marginBottom: 4 }}>
                  <div style={GROUP_HEADING_STYLE} aria-hidden="true" />
                  {items.map((item) => (
                    <CommandItem
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
              {" "}Navigate
            </span>
            <span>
              <kbd style={{ padding: "1px 4px", borderRadius: 3, background: "var(--bg-3)", border: "1px solid var(--surface-border)" }}>↵</kbd>
              {" "}Select
            </span>
            <span>
              <kbd style={{ padding: "1px 4px", borderRadius: 3, background: "var(--bg-3)", border: "1px solid var(--surface-border)" }}>Esc</kbd>
              {" "}Close
            </span>
            <span style={{ marginLeft: "auto" }}>
              <kbd style={{ padding: "1px 4px", borderRadius: 3, background: "var(--bg-3)", border: "1px solid var(--surface-border)" }}>Ctrl+K</kbd>
              {" "}Palette
            </span>
          </div>
        </Command>
      </div>
    </AnimatedModal>
  );
}
