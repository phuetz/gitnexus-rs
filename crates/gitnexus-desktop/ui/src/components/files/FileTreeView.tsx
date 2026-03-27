import { useState } from "react";
import { ChevronRight, ChevronDown, File, Folder } from "lucide-react";
import { useFileTree } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";
import type { FileTreeNode } from "../../lib/tauri-commands";

const LANGUAGE_COLORS: Record<string, string> = {
  js: "var(--cyan)",
  jsx: "var(--cyan)",
  ts: "var(--purple)",
  tsx: "var(--purple)",
  py: "var(--blue)",
  java: "var(--rose)",
  c: "var(--cyan)",
  cpp: "var(--purple)",
  cxx: "var(--purple)",
  cc: "var(--purple)",
  cs: "var(--green)",
  go: "var(--cyan)",
  rs: "var(--rose)",
  php: "var(--purple)",
  rb: "var(--rose)",
  kt: "var(--purple)",
  swift: "var(--rose)",
  json: "var(--amber)",
  yaml: "var(--cyan)",
  yml: "var(--cyan)",
  xml: "var(--amber)",
  html: "var(--rose)",
  css: "var(--cyan)",
  scss: "var(--cyan)",
  sass: "var(--cyan)",
};

function getFileColor(filename: string): string {
  const ext = filename.split(".").pop()?.toLowerCase() || "";
  return LANGUAGE_COLORS[ext] || "var(--text-3)";
}

function countFiles(nodes: FileTreeNode[]): number {
  let count = 0;
  for (const node of nodes) {
    if (!node.isDir) {
      count++;
    } else {
      count += countFiles(node.children);
    }
  }
  return count;
}

export function FileTreeView() {
  const { t } = useI18n();
  const { data: tree, isLoading, error } = useFileTree(true);

  if (isLoading) {
    return (
      <div
        className="h-full flex items-center justify-center shimmer"
        style={{ color: "var(--text-3)" }}
      >
        <div className="space-y-2 w-full px-4">
          <div
            className="h-4 rounded"
            style={{ backgroundColor: "var(--bg-2)", animation: "shimmer 2s infinite" }}
          />
          <div
            className="h-4 rounded"
            style={{ backgroundColor: "var(--bg-2)", animation: "shimmer 2s infinite" }}
          />
          <div
            className="h-4 rounded"
            style={{ backgroundColor: "var(--bg-2)", animation: "shimmer 2s infinite" }}
          />
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div
        className="h-full flex items-center justify-center p-4 text-center"
        style={{ color: "var(--rose)" }}
      >
        {t("files.errorLoadingTree")}
      </div>
    );
  }

  if (!tree || tree.length === 0) {
    return (
      <div
        className="h-full flex items-center justify-center p-4 text-center"
        style={{ color: "var(--text-3)" }}
      >
        {t("files.noFilesFound")}
      </div>
    );
  }

  const fileCount = countFiles(tree);

  return (
    <div
      className="h-full flex flex-col"
      style={{ backgroundColor: "var(--bg-0)" }}
    >
      {/* Header */}
      <div
        className="px-3 py-4 border-b flex items-center justify-between"
        style={{
          backgroundColor: "var(--bg-1)",
          borderColor: "var(--surface-border)",
        }}
      >
        <div className="flex items-center gap-2">
          <Folder size={16} style={{ color: "var(--amber)" }} />
          <h2
            className="text-sm font-semibold"
            style={{ color: "var(--text-0)" }}
          >
            {t("files.title")}
          </h2>
        </div>
        <span
          className="text-xs px-2 py-1 rounded"
          style={{
            backgroundColor: "var(--bg-2)",
            color: "var(--text-2)",
          }}
        >
          {fileCount}
        </span>
      </div>

      {/* Tree container */}
      <div
        className="flex-1 overflow-y-auto px-3 py-4"
        style={{ backgroundColor: "var(--bg-0)" }}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: "0px" }}>
          {tree.map((node) => (
            <TreeNode key={node.path} node={node} depth={0} parentPath="" />
          ))}
        </div>
      </div>
    </div>
  );
}

interface TreeNodeProps {
  node: FileTreeNode;
  depth: number;
  parentPath: string;
}

function TreeNode({ node, depth, parentPath }: TreeNodeProps) {
  const [expanded, setExpanded] = useState(depth < 1);
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const setDetailTab = useAppStore((s) => s.setDetailTab);

  const fullPath = parentPath ? `${parentPath}/${node.name}` : node.name;
  const isSelected = selectedNodeId === `File:${fullPath}`;

  const handleClick = () => {
    if (node.isDir) {
      setExpanded(!expanded);
    } else {
      setSelectedNodeId(`File:${fullPath}`, node.name);
      setDetailTab("code");
    }
  };

  return (
    <div>
      <button
        onClick={handleClick}
        className="flex items-center gap-2 w-full rounded text-left text-[13px] transition-colors relative group"
        style={{
          paddingLeft: `${depth * 16 + 12}px`,
          paddingRight: "8px",
          paddingTop: "6px",
          paddingBottom: "6px",
          backgroundColor: isSelected
            ? "var(--accent-subtle)"
            : "transparent",
          color: isSelected ? "var(--accent)" : "var(--text-1)",
        }}
      >
        {/* Indentation guide lines */}
        {depth > 0 && (
          <div
            style={{
              position: "absolute",
              left: `${(depth - 1) * 16 + 20}px`,
              top: "0",
              bottom: "0",
              width: "1px",
              borderLeft: "1px dotted",
              borderColor: "var(--surface-border)",
              opacity: "0.5",
            }}
          />
        )}

        {/* Chevron or spacer */}
        {node.isDir ? (
          expanded ? (
            <ChevronDown
              size={14}
              className="shrink-0"
              style={{ color: "var(--text-2)" }}
            />
          ) : (
            <ChevronRight
              size={14}
              className="shrink-0"
              style={{ color: "var(--text-2)" }}
            />
          )
        ) : (
          <span style={{ width: "14px", height: "14px", flexShrink: 0 }} />
        )}

        {/* Icon */}
        {node.isDir ? (
          <Folder
            size={14}
            className="shrink-0"
            style={{ color: "var(--amber)" }}
          />
        ) : (
          <File
            size={14}
            className="shrink-0"
            style={{ color: getFileColor(node.name) }}
          />
        )}

        {/* Name */}
        <span className="truncate flex-1">{node.name}</span>

        {/* Hover background indicator */}
        <div
          className="absolute inset-0 rounded pointer-events-none group-hover:opacity-100"
          style={{
            backgroundColor: "var(--surface-hover)",
            opacity: "0",
            transition: "opacity 0.15s ease-in-out",
            zIndex: -1,
          }}
        />
      </button>

      {/* Children */}
      {node.isDir && expanded && (
        <div>
          {node.children.map((child) => (
            <TreeNode
              key={child.path}
              node={child}
              depth={depth + 1}
              parentPath={fullPath}
            />
          ))}
        </div>
      )}
    </div>
  );
}
