import {
  LayoutDashboard,
  GitBranch,
  FolderTree,
  Network,
  Zap,
  FileText,
  Download,
  Settings,
  PanelLeftClose,
  PanelLeft,
} from "lucide-react";
import { useAppStore, type SidebarTab } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";
import { Tooltip } from "../shared/Tooltip";

const WORKSPACE_TABS: { id: SidebarTab; icon: typeof GitBranch; labelKey: string }[] = [
  { id: "overview", icon: LayoutDashboard, labelKey: "sidebar.overview" },
  { id: "repos", icon: GitBranch, labelKey: "sidebar.repositories" },
  { id: "files", icon: FolderTree, labelKey: "sidebar.fileExplorer" },
];

const TOOL_TABS: { id: SidebarTab; icon: typeof Network; labelKey: string }[] = [
  { id: "graph", icon: Network, labelKey: "sidebar.graphExplorer" },
  { id: "impact", icon: Zap, labelKey: "sidebar.impactAnalysis" },
  { id: "docs", icon: FileText, labelKey: "sidebar.documentation" },
  { id: "export", icon: Download, labelKey: "sidebar.export" },
];

export function Sidebar() {
  const { t, tt } = useI18n();
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
      <div
        className="flex items-center shrink-0"
        style={{ gap: 10, paddingLeft: 16, paddingRight: 16, height: 52 }}
      >
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
        {(() => {
          const toggleText = tt(collapsed ? "sidebar.expand" : "sidebar.collapse");
          return (
            <Tooltip content={toggleText.tip}>
              <button
                onClick={toggle}
                title={toggleText.label}
                aria-label={toggleText.label}
                className="rounded-md transition-colors shrink-0"
                style={{ marginLeft: "auto", padding: 4, color: "var(--text-3)" }}
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
            </Tooltip>
          );
        })()}
      </div>

      {/* Separator below logo */}
      <div
        style={{
          height: "1px",
          background: "var(--surface-border)",
          margin: "0 12px",
        }}
      />

      {/* Nav sections */}
      <div
        className="flex-1 overflow-y-auto"
        style={{ padding: "8px 12px" }}
      >
        <SectionLabel collapsed={collapsed}>{t("sidebar.workspace")}</SectionLabel>
        {WORKSPACE_TABS.map((tab) => (
          <NavItem
            key={tab.id}
            icon={tab.icon}
            labelKey={tab.labelKey}
            active={sidebarTab === tab.id}
            collapsed={collapsed}
            onClick={() => setSidebarTab(tab.id)}
          />
        ))}

        {/* Section divider */}
        <div
          style={{
            height: "1px",
            background: "var(--surface-border)",
            margin: "12px 0",
          }}
        />

        <SectionLabel collapsed={collapsed}>{t("sidebar.analysis")}</SectionLabel>
        {TOOL_TABS.map((tab) => (
          <NavItem
            key={tab.id}
            icon={tab.icon}
            labelKey={tab.labelKey}
            active={sidebarTab === tab.id}
            collapsed={collapsed}
            onClick={() => setSidebarTab(tab.id)}
          />
        ))}
      </div>

      {/* Separator above settings */}
      <div
        style={{
          height: "1px",
          background: "var(--surface-border)",
          margin: "0 12px",
        }}
      />

      {/* Bottom */}
      <div className="shrink-0" style={{ padding: "10px 12px" }}>
        <NavItem
          icon={Settings}
          labelKey="sidebar.settings"
          active={false}
          collapsed={collapsed}
          onClick={() => useAppStore.getState().setSettingsOpen(true)}
        />
      </div>
    </div>
  );
}

function SectionLabel({ collapsed, children }: { collapsed: boolean; children: React.ReactNode }) {
  if (collapsed) return <div className="h-3" />;
  return (
    <div
      className="text-[9px] font-semibold uppercase tracking-wider select-none"
      style={{
        color: "var(--text-4)",
        fontFamily: "var(--font-display)",
        paddingLeft: 12,
        paddingRight: 12,
        paddingTop: 12,
        paddingBottom: 6,
      }}
    >
      {children}
    </div>
  );
}

function NavItem({
  icon: Icon,
  labelKey,
  active,
  collapsed,
  onClick,
}: {
  icon: typeof GitBranch;
  labelKey: string;
  active: boolean;
  collapsed: boolean;
  onClick: () => void;
}) {
  const { t } = useI18n();
  const label = t(labelKey);
  return (
    <button
      onClick={onClick}
      title={collapsed ? label : undefined}
      className="w-full flex items-center rounded-lg transition-all duration-150 group relative overflow-hidden"
      style={{
        gap: 10,
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
        <>
          {/* Left indicator with gradient glow */}
          <div
            className="absolute left-0 top-1/2 -translate-y-1/2 w-[2.5px] rounded-r-full transition-all duration-200"
            style={{
              height: 20,
              background: "linear-gradient(180deg, transparent, var(--accent), transparent)",
              boxShadow: "0 0 12px var(--accent-glow)",
            }}
          />
          {/* Subtle background glow */}
          <div
            className="absolute inset-0 opacity-50"
            style={{
              background: "radial-gradient(100px circle at left, var(--accent-glow), transparent)",
              pointerEvents: "none",
            }}
          />
        </>
      )}
      <Icon size={16} className="shrink-0 relative z-10" />
      {!collapsed && (
        <span className="text-[13px] font-medium truncate relative z-10">{label}</span>
      )}
    </button>
  );
}
