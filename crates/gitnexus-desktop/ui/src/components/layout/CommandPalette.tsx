import { Command } from "cmdk";
import {
  GitBranch,
  FolderTree,
  Network,
  Zap,
  FileText,
  Download,
  PanelLeftClose,
  Settings,
  Sparkles,
} from "lucide-react";
import { AnimatedModal } from "../shared/motion";
import { useAppStore, type SidebarTab } from "../../stores/app-store";
import { useChatStore } from "../../stores/chat-store";

const NAV_ITEMS: { id: SidebarTab; label: string; icon: typeof GitBranch; shortcut: string }[] = [
  { id: "repos", label: "Repositories", icon: GitBranch, shortcut: "Ctrl+1" },
  { id: "files", label: "File Explorer", icon: FolderTree, shortcut: "Ctrl+2" },
  { id: "graph", label: "Graph Explorer", icon: Network, shortcut: "Ctrl+3" },
  { id: "impact", label: "Impact Analysis", icon: Zap, shortcut: "Ctrl+4" },
  { id: "docs", label: "Documentation", icon: FileText, shortcut: "Ctrl+5" },
  { id: "export", label: "Export", icon: Download, shortcut: "" },
];

const ACTION_ITEMS: { id: string; label: string; icon: typeof Settings; shortcut: string; action: () => void }[] = [
  {
    id: "toggle-sidebar",
    label: "Toggle Sidebar",
    icon: PanelLeftClose,
    shortcut: "Ctrl+B",
    action: () => useAppStore.getState().toggleSidebar(),
  },
  {
    id: "open-settings",
    label: "Open Settings",
    icon: Settings,
    shortcut: "",
    action: () => useAppStore.getState().setSettingsOpen(true),
  },
  {
    id: "toggle-deep-research",
    label: "Toggle Deep Research",
    icon: Sparkles,
    shortcut: "Ctrl+Shift+D",
    action: () => useChatStore.getState().toggleDeepResearch(),
  },
];

export function CommandPalette() {
  const commandPaletteOpen = useAppStore((s) => s.commandPaletteOpen);
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen);
  const setSidebarTab = useAppStore((s) => s.setSidebarTab);

  const close = () => setCommandPaletteOpen(false);

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
        <Command
          label="Command Palette"
          style={{ background: "transparent" }}
        >
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
              maxHeight: 360,
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

            {/* Navigation Group */}
            <Command.Group
              heading="Navigation"
              style={{ marginBottom: 8 }}
            >
              <div
                style={{
                  fontSize: 11,
                  fontWeight: 600,
                  color: "var(--text-3)",
                  textTransform: "uppercase",
                  letterSpacing: "0.05em",
                  padding: "8px 12px 4px",
                  fontFamily: "var(--font-display)",
                }}
                aria-hidden="true"
              />
              {NAV_ITEMS.map((item) => (
                <Command.Item
                  key={item.id}
                  value={`${item.label} ${item.id}`}
                  onSelect={() => {
                    setSidebarTab(item.id);
                    close();
                  }}
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
                    if (!e.currentTarget.getAttribute("aria-selected") || e.currentTarget.getAttribute("aria-selected") !== "true") {
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
              ))}
            </Command.Group>

            {/* Actions Group */}
            <Command.Group
              heading="Actions"
            >
              <div
                style={{
                  fontSize: 11,
                  fontWeight: 600,
                  color: "var(--text-3)",
                  textTransform: "uppercase",
                  letterSpacing: "0.05em",
                  padding: "8px 12px 4px",
                  fontFamily: "var(--font-display)",
                }}
                aria-hidden="true"
              />
              {ACTION_ITEMS.map((item) => (
                <Command.Item
                  key={item.id}
                  value={item.label}
                  onSelect={() => {
                    item.action();
                    close();
                  }}
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
                    if (!e.currentTarget.getAttribute("aria-selected") || e.currentTarget.getAttribute("aria-selected") !== "true") {
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
              ))}
            </Command.Group>
          </Command.List>

          {/* Footer with keyboard hints */}
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
          </div>
        </Command>
      </div>
    </AnimatedModal>
  );
}
