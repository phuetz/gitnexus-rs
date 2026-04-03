import { Compass, BarChart3, MessageSquare, Settings, Search } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import type { AppMode } from "../../stores/app-store";
import { Tooltip } from "../shared/Tooltip";

const modes: { mode: AppMode; icon: typeof Compass; label: string; shortcut: string }[] = [
  { mode: "explorer", icon: Compass, label: "Explorer", shortcut: "Ctrl+1" },
  { mode: "analyze", icon: BarChart3, label: "Analyze", shortcut: "Ctrl+2" },
  { mode: "chat", icon: MessageSquare, label: "Chat", shortcut: "Ctrl+3" },
  { mode: "manage", icon: Settings, label: "Manage", shortcut: "Ctrl+4" },
];

export function ModeBar() {
  const mode = useAppStore((s) => s.mode);
  const setMode = useAppStore((s) => s.setMode);
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen);

  return (
    <div
      className="flex flex-col items-center py-2 h-full shrink-0"
      style={{
        width: "var(--mode-bar-width)",
        background: "var(--mode-bar-bg)",
        borderRight: "1px solid var(--surface-border)",
      }}
    >
      {modes.map(({ mode: m, icon: Icon, label, shortcut }) => (
        <Tooltip key={m} content={`${label} (${shortcut})`} side="right">
          <button
            onClick={() => setMode(m)}
            className="relative flex items-center justify-center w-10 h-10 mb-1 rounded-lg transition-colors"
            style={{
              color: mode === m ? "var(--accent)" : "var(--text-3)",
              background: mode === m ? "var(--accent-subtle)" : "transparent",
              boxShadow: mode === m ? "var(--glow-accent, 0 0 8px var(--accent))" : "none",
            }}
            aria-label={label}
            aria-current={mode === m ? "page" : undefined}
          >
            {mode === m && (
              <div
                className="absolute left-0 top-1/2 -translate-y-1/2 rounded-r"
                style={{ width: 3, height: 20, background: "var(--accent)" }}
              />
            )}
            <Icon size={20} />
          </button>
        </Tooltip>
      ))}

      <div className="flex-1" />

      {/* Command palette trigger */}
      <Tooltip content="Command Palette (Ctrl+K)" side="right">
        <button
          onClick={() => setCommandPaletteOpen(true)}
          className="flex items-center justify-center w-10 h-10 mb-2 rounded-lg transition-colors"
          style={{ color: "var(--text-3)" }}
          aria-label="Command Palette"
        >
          <Search size={18} />
        </button>
      </Tooltip>
    </div>
  );
}
