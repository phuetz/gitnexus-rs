/**
 * DocsNav — hierarchical wiki navigation for the docs viewer.
 * Renders the _index.json structure as an expandable tree.
 */

import { useState, useMemo } from "react";
import {
  Home,
  GitBranch,
  BookOpen,
  Layers,
  Box,
  ChevronRight,
  ChevronDown,
  RefreshCw,
  FileText,
} from "lucide-react";
import { useI18n } from "../../hooks/use-i18n";

// ─── Types (mirror Rust DocIndex / DocPage) ─────────────────────────

export interface DocPage {
  id: string;
  title: string;
  path?: string;
  icon?: string;
  children?: DocPage[];
}

export interface DocStats {
  files: number;
  nodes: number;
  edges: number;
  modules: number;
}

export interface DocIndex {
  title: string;
  generatedAt: string;
  stats: DocStats;
  pages: DocPage[];
}

// ─── Icon mapping ───────────────────────────────────────────────────

const ICON_MAP: Record<string, React.ComponentType<{ size?: number }>> = {
  home: Home,
  "git-branch": GitBranch,
  "book-open": BookOpen,
  layers: Layers,
  box: Box,
  "file-text": FileText,
};

function PageIcon({ icon, size = 14 }: { icon?: string; size?: number }) {
  const IconComponent = icon ? ICON_MAP[icon] : FileText;
  const Comp = IconComponent || FileText;
  return <Comp size={size} />;
}

// ─── Props ──────────────────────────────────────────────────────────

interface DocsNavProps {
  index: DocIndex;
  activePath: string | null;
  onNavigate: (path: string) => void;
  onRegenerate: () => void;
  isRegenerating: boolean;
}

// ─── Component ──────────────────────────────────────────────────────

export function DocsNav({ index, activePath, onNavigate, onRegenerate, isRegenerating }: DocsNavProps) {
  const { t } = useI18n();
  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-4 pt-4 pb-3">
        <div className="flex items-center justify-between mb-1">
          <h3
            className="text-sm font-semibold truncate"
            style={{ fontFamily: "var(--font-display)", color: "var(--text-0)" }}
          >
            {index.title}
          </h3>
          <button
            onClick={onRegenerate}
            disabled={isRegenerating}
            className="p-1 rounded transition-colors"
            style={{ color: "var(--text-3)" }}
            title={t("docs.regenerateTitle")}
          >
            <RefreshCw size={13} className={isRegenerating ? "animate-spin" : ""} />
          </button>
        </div>
        <div className="flex gap-3 text-[11px]" style={{ color: "var(--text-3)" }}>
          <span>{index.stats.files} {t("docs.statsFiles")}</span>
          <span>{index.stats.modules} {t("docs.statsModules")}</span>
        </div>
      </div>

      {/* Separator */}
      <div className="mx-3 mb-1" style={{ borderBottom: "1px solid var(--surface-border)" }} />

      {/* Navigation tree */}
      <nav className="flex-1 overflow-y-auto px-2 pb-4">
        {index.pages.map((page) => (
          <NavItem
            key={page.id}
            page={page}
            depth={0}
            activePath={activePath}
            onNavigate={onNavigate}
          />
        ))}
      </nav>
    </div>
  );
}

// ─── NavItem (recursive) ────────────────────────────────────────────

function NavItem({
  page,
  depth,
  activePath,
  onNavigate,
}: {
  page: DocPage;
  depth: number;
  activePath: string | null;
  onNavigate: (path: string) => void;
}) {
  const hasChildren = page.children && page.children.length > 0;
  const [expanded, setExpanded] = useState(true); // default expanded

  const isActive = activePath === page.path;
  const isInActiveSubtree = useMemo(() => {
    if (!activePath || !hasChildren) return false;
    return page.children!.some(
      (c) => c.path === activePath || c.children?.some((cc) => cc.path === activePath)
    );
  }, [activePath, hasChildren, page.children]);

  const handleClick = () => {
    if (page.path) {
      onNavigate(page.path);
    }
    if (hasChildren) {
      setExpanded((prev) => !prev);
    }
  };

  return (
    <div>
      <button
        onClick={handleClick}
        className="w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-left text-[13px] transition-colors group"
        style={{
          paddingLeft: `${8 + depth * 12}px`,
          background: isActive ? "var(--accent-subtle)" : "transparent",
          color: isActive ? "var(--accent)" : isInActiveSubtree ? "var(--text-1)" : "var(--text-2)",
        }}
      >
        {/* Expand/collapse icon for parents */}
        {hasChildren ? (
          <span className="flex-shrink-0 w-3.5">
            {expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
          </span>
        ) : (
          <span className="flex-shrink-0 w-3.5" />
        )}

        {/* Page icon */}
        <span className="flex-shrink-0 opacity-60">
          <PageIcon icon={page.icon} size={14} />
        </span>

        {/* Label */}
        <span className="truncate">{page.title}</span>
      </button>

      {/* Children */}
      {hasChildren && expanded && (
        <div>
          {page.children!.map((child) => (
            <NavItem
              key={child.id}
              page={child}
              depth={depth + 1}
              activePath={activePath}
              onNavigate={onNavigate}
            />
          ))}
        </div>
      )}
    </div>
  );
}
