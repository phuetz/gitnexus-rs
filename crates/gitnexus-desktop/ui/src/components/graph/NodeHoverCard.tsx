import { motion, AnimatePresence } from "framer-motion";
import { ArrowDownLeft, ArrowUpRight, Code2, Zap } from "lucide-react";

const LABEL_COLORS: Record<string, string> = {
  Function: "#7aa2f7",
  Class: "#bb9af7",
  Method: "#7dcfff",
  Interface: "#e0af68",
  Struct: "#ff9e64",
  Trait: "#9ece6a",
  Enum: "#f7768e",
  File: "#565f89",
  Folder: "#414868",
  Module: "#565f89",
  Package: "#414868",
  Variable: "#73daca",
  Type: "#c0caf5",
  Import: "#414868",
  Community: "#9ece6a",
  Process: "#e0af68",
  Constructor: "#7dcfff",
  Property: "#73daca",
  Route: "#ff9e64",
  Tool: "#e0af68",
  Namespace: "#414868",
};

export interface NodeHoverCardProps {
  node: {
    id: string;
    name: string;
    label: string;
    filePath: string;
    startLine?: number;
    endLine?: number;
  } | null;
  position: { x: number; y: number } | null;
  inDegree: number;
  outDegree: number;
  onViewSource?: () => void;
  onImpact?: () => void;
}

export function NodeHoverCard({
  node,
  position,
  inDegree,
  outDegree,
  onViewSource,
  onImpact,
}: NodeHoverCardProps) {
  return (
    <AnimatePresence>
      {node && position && (
        <motion.div
          key={node.id}
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          exit={{ opacity: 0, scale: 0.95 }}
          transition={{ duration: 0.15, ease: "easeOut" }}
          className="absolute z-50 pointer-events-none rounded-xl"
          style={{
            left: position.x + 12,
            top: position.y + 12,
            maxWidth: 280,
            background: "var(--bg-2)",
            border: "1px solid var(--surface-border)",
            backdropFilter: "blur(12px)",
            WebkitBackdropFilter: "blur(12px)",
            boxShadow: "var(--shadow-lg)",
            padding: "12px 14px",
          }}
        >
          {/* Name + type badge */}
          <div className="flex items-center gap-2 mb-1.5">
            <span
              className="font-semibold text-sm truncate"
              style={{ color: "var(--text-0)", maxWidth: 180 }}
            >
              {node.name}
            </span>
            <span
              className="text-[10px] font-medium px-1.5 py-0.5 rounded-full shrink-0"
              style={{
                backgroundColor: `${LABEL_COLORS[node.label] || "#565f89"}20`,
                color: LABEL_COLORS[node.label] || "#565f89",
                border: `1px solid ${LABEL_COLORS[node.label] || "#565f89"}30`,
              }}
            >
              {node.label}
            </span>
          </div>

          {/* File path */}
          <p
            className="text-[11px] truncate mb-2"
            style={{ color: "var(--text-3)" }}
          >
            {node.filePath}
          </p>

          {/* Line range */}
          {node.startLine != null && (
            <p
              className="text-[10px] mb-2 font-mono"
              style={{ color: "var(--text-4)" }}
            >
              {node.endLine != null
                ? `L${node.startLine}\u2013${node.endLine}`
                : `L${node.startLine}`}
            </p>
          )}

          {/* Degree info */}
          <div
            className="flex items-center gap-4 mb-2 pt-2"
            style={{ borderTop: "1px solid var(--surface-border)" }}
          >
            <div className="flex items-center gap-1">
              <ArrowDownLeft
                size={12}
                style={{ color: "var(--green)" }}
              />
              <span
                className="text-[11px] font-medium"
                style={{ color: "var(--text-2)" }}
              >
                {inDegree} in
              </span>
            </div>
            <div className="flex items-center gap-1">
              <ArrowUpRight
                size={12}
                style={{ color: "var(--accent)" }}
              />
              <span
                className="text-[11px] font-medium"
                style={{ color: "var(--text-2)" }}
              >
                {outDegree} out
              </span>
            </div>
          </div>

          {/* Action buttons */}
          <div
            className="flex items-center gap-2 pt-2"
            style={{ borderTop: "1px solid var(--surface-border)" }}
          >
            <button
              className="flex items-center gap-1 text-[10px] font-medium px-2 py-1 rounded-md pointer-events-auto"
              style={{
                background: "var(--bg-3)",
                color: "var(--text-2)",
                border: "1px solid var(--surface-border)",
              }}
              onClick={onViewSource}
            >
              <Code2 size={10} />
              Source
            </button>
            <button
              className="flex items-center gap-1 text-[10px] font-medium px-2 py-1 rounded-md pointer-events-auto"
              style={{
                background: "var(--bg-3)",
                color: "var(--text-2)",
                border: "1px solid var(--surface-border)",
              }}
              onClick={onImpact}
            >
              <Zap size={10} />
              Impact
            </button>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
