import {
  GitBranch,
  FolderTree,
  Network,
  Zap,
  FileText,
  Settings,
  PanelLeftClose,
  PanelLeft,
} from "lucide-react";
import { useAppStore, type SidebarTab } from "../../stores/app-store";

const WORKSPACE_TABS: { id: SidebarTab; icon: typeof GitBranch; label: string }[] = [
  { id: "repos", icon: GitBranch, label: "Repositories" },
  { id: "files", icon: FolderTree, label: "File Explorer" },
];

const TOOL_TABS: { id: SidebarTab; icon: typeof Network; label: string }[] = [
  { id: "graph", icon: Network, label: "Graph Explorer" },
  { id: "impact", icon: Zap, label: "Impact Analysis" },
  { id: "docs", icon: FileText, label: "Documentation" },
];

export function Sidebar() {
  const sidebarTab = useAppStore((s) => s.sidebarTab);
  const setSidebarTab = useAppStore((s) => s.setSidebarTab);
  const collapsed = useAppStore((s) => s.sidebarCollapsed);
  const toggle = useAppStore((s) => s.toggleSidebar);

  return (
    <div
      className="flex flex-col h-full border-r transition-all duration-200 ease-out shrink-0"
      style={{
        width: collapsed ? 52 : 220,
        background: "var(--bg-1)",
        borderColor: "var(--surface-border)",
      }}
    >
      {/* Logo + collapse */}
      <div className="flex items-center gap-2.5 px-3 h-[52px] shrink-0">
        <div
          className="w-7 h-7 rounded-lg flex items-center justify-center shrink-0"
          style={{
            background: "linear-gradient(135deg, var(--accent), #8b5cf6)",
            color: "white",
            fontFamily: "var(--font-display)",
            fontWeight: 700,
            fontSize: 14,
          }}
        >
          G
        </div>
        {!collapsed && (
          <span
            className="text-sm font-semibold tracking-tight truncate"
            style={{ color: "var(--text-0)", fontFamily: "var(--font-display)" }}
          >
            GitNexus
          </span>
        )}
        <button
          onClick={toggle}
          className="ml-auto p-1 rounded-md transition-colors shrink-0"
          style={{ color: "var(--text-3)" }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = "var(--surface-hover)";
            e.currentTarget.style.color = "var(--text-2)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = "transparent";
            e.currentTarget.style.color = "var(--text-3)";
          }}
        >
          {collapsed ? <PanelLeft size={16} /> : <PanelLeftClose size={16} />}
        </button>
      </div>

      {/* Nav sections */}
      <div className="flex-1 overflow-y-auto px-2 py-1">
        <SectionLabel collapsed={collapsed}>Workspace</SectionLabel>
        {WORKSPACE_TABS.map((tab) => (
          <NavItem
            key={tab.id}
            icon={tab.icon}
            label={tab.label}
            active={sidebarTab === tab.id}
            collapsed={collapsed}
            onClick={() => setSidebarTab(tab.id)}
          />
        ))}

        <div className="my-3" />

        <SectionLabel collapsed={collapsed}>Tools</SectionLabel>
        {TOOL_TABS.map((tab) => (
          <NavItem
            key={tab.id}
            icon={tab.icon}
            label={tab.label}
            active={sidebarTab === tab.id}
            collapsed={collapsed}
            onClick={() => setSidebarTab(tab.id)}
          />
        ))}
      </div>

      {/* Bottom */}
      <div className="px-2 py-2 shrink-0" style={{ borderTop: "1px solid var(--surface-border)" }}>
        <NavItem
          icon={Settings}
          label="Settings"
          active={false}
          collapsed={collapsed}
          onClick={() => {}}
        />
      </div>
    </div>
  );
}

function SectionLabel({ collapsed, children }: { collapsed: boolean; children: React.ReactNode }) {
  if (collapsed) return <div className="h-3" />;
  return (
    <div
      className="px-2 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-widest select-none"
      style={{ color: "var(--text-3)", fontFamily: "var(--font-display)" }}
    >
      {children}
    </div>
  );
}

function NavItem({
  icon: Icon,
  label,
  active,
  collapsed,
  onClick,
}: {
  icon: typeof GitBranch;
  label: string;
  active: boolean;
  collapsed: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      title={collapsed ? label : undefined}
      className="w-full flex items-center gap-2.5 rounded-lg transition-all duration-150 group relative"
      style={{
        padding: collapsed ? "8px" : "7px 10px",
        justifyContent: collapsed ? "center" : "flex-start",
        background: active ? "var(--accent-subtle)" : "transparent",
        color: active ? "var(--accent)" : "var(--text-2)",
      }}
      onMouseEnter={(e) => {
        if (!active) {
          e.currentTarget.style.background = "var(--surface-hover)";
          e.currentTarget.style.color = "var(--text-1)";
        }
      }}
      onMouseLeave={(e) => {
        if (!active) {
          e.currentTarget.style.background = "transparent";
          e.currentTarget.style.color = "var(--text-2)";
        }
      }}
    >
      {active && (
        <div
          className="absolute left-0 top-1/2 -translate-y-1/2 w-[2.5px] rounded-r-full"
          style={{ height: 16, background: "var(--accent)" }}
        />
      )}
      <Icon size={16} className="shrink-0" />
      {!collapsed && (
        <span className="text-[13px] font-medium truncate">{label}</span>
      )}
    </button>
  );
}
