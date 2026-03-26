import { useState } from "react";
import { ChevronRight, ChevronDown, File, Folder } from "lucide-react";
import { useFileTree } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";
import type { FileTreeNode } from "../../lib/tauri-commands";

export function FileTreeView() {
  const { data: tree, isLoading, error } = useFileTree(true);

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center text-[var(--text-muted)]">
        Loading file tree...
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex items-center justify-center text-[var(--danger)]">
        Error: {String(error)}
      </div>
    );
  }

  if (!tree || tree.length === 0) {
    return (
      <div className="h-full flex items-center justify-center text-[var(--text-muted)]">
        No files found
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto p-2">
      <h2 className="text-sm font-semibold text-[var(--text-secondary)] px-2 py-1 mb-1">
        Files
      </h2>
      {tree.map((node) => (
        <TreeNode key={node.path} node={node} depth={0} parentPath="" />
      ))}
    </div>
  );
}

function TreeNode({
  node,
  depth,
  parentPath,
}: {
  node: FileTreeNode;
  depth: number;
  parentPath: string;
}) {
  const [expanded, setExpanded] = useState(depth < 1);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const setDetailTab = useAppStore((s) => s.setDetailTab);

  const fullPath = parentPath ? `${parentPath}/${node.name}` : node.name;

  const handleClick = () => {
    if (node.isDir) {
      setExpanded(!expanded);
    } else {
      // Select the file node and switch to code tab
      setSelectedNodeId(`File:${fullPath}`);
      setDetailTab("code");
    }
  };

  return (
    <div>
      <button
        onClick={handleClick}
        className="flex items-center gap-1 w-full px-2 py-0.5 rounded text-left text-[13px] hover:bg-[var(--bg-tertiary)] transition-colors"
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
      >
        {node.isDir ? (
          expanded ? (
            <ChevronDown size={14} className="text-[var(--text-muted)] shrink-0" />
          ) : (
            <ChevronRight size={14} className="text-[var(--text-muted)] shrink-0" />
          )
        ) : (
          <span className="w-[14px] shrink-0" />
        )}
        {node.isDir ? (
          <Folder size={14} className="text-[var(--warning)] shrink-0" />
        ) : (
          <File size={14} className="text-[var(--text-muted)] shrink-0" />
        )}
        <span className="truncate">{node.name}</span>
      </button>
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
