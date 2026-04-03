/**
 * RepoDashboard -- SonarQube / CodeScene-inspired overview screen.
 * Shows quality banner, metric cards, node type distribution, and top connected nodes.
 */

import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Files,
  Code2,
  Boxes,
  Network,
  ArrowRightLeft,
  Languages,
  Zap,
  GitBranch,
  Shield,
} from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import { CodeHealthCard } from "../health/CodeHealthCard";
import { commands } from "../../lib/tauri-commands";
import type { CytoNode, CytoEdge, GraphPayload } from "../../lib/tauri-commands";
import {
  AnimatedCard,
  AnimatedCounter,
  StaggerContainer,
  StaggerItem,
  SkeletonBlock,
} from "../shared/motion";

// ─── Label Colors (mirrored from GraphExplorer) ─────────────────────

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

// ─── Helpers ────────────────────────────────────────────────────────

/** Count nodes matching a given label (case-insensitive). */
function countByLabel(nodes: CytoNode[], label: string): number {
  return nodes.filter(
    (n) => n.label.toLowerCase() === label.toLowerCase(),
  ).length;
}

/** Build a map of label -> count, sorted descending. */
function buildLabelDistribution(nodes: CytoNode[]): { label: string; count: number; color: string }[] {
  const map = new Map<string, number>();
  for (const n of nodes) {
    map.set(n.label, (map.get(n.label) ?? 0) + 1);
  }
  return Array.from(map.entries())
    .map(([label, count]) => ({
      label,
      count,
      color: LABEL_COLORS[label] ?? "#565f89",
    }))
    .sort((a, b) => b.count - a.count);
}

/** Compute degree (edge count) per node, return top N. */
function topConnectedNodes(
  nodes: CytoNode[],
  edges: CytoEdge[],
  limit: number,
): { node: CytoNode; degree: number }[] {
  const degreeMap = new Map<string, number>();
  for (const e of edges) {
    degreeMap.set(e.source, (degreeMap.get(e.source) ?? 0) + 1);
    degreeMap.set(e.target, (degreeMap.get(e.target) ?? 0) + 1);
  }
  const nodeMap = new Map<string, CytoNode>();
  for (const n of nodes) nodeMap.set(n.id, n);

  return Array.from(degreeMap.entries())
    .sort((a, b) => b[1] - a[1])
    .slice(0, limit)
    .map(([id, degree]) => ({ node: nodeMap.get(id)!, degree }))
    .filter((x) => x.node != null);
}

/** Unique language count from nodes. */
function uniqueLanguages(nodes: CytoNode[]): number {
  const langs = new Set<string>();
  for (const n of nodes) {
    if (n.language) langs.add(n.language);
  }
  return langs.size;
}

// ─── Metric Card Config ─────────────────────────────────────────────

interface MetricDef {
  key: string;
  label: string;
  icon: React.ComponentType<{ size?: number; style?: React.CSSProperties }>;
  color: string;
  getValue: (nodes: CytoNode[], edges: CytoEdge[]) => number;
}

const METRICS: MetricDef[] = [
  {
    key: "files",
    label: "Files",
    icon: Files,
    color: "#565f89",
    getValue: (nodes) => countByLabel(nodes, "File"),
  },
  {
    key: "functions",
    label: "Functions",
    icon: Code2,
    color: "#7aa2f7",
    getValue: (nodes) => countByLabel(nodes, "Function"),
  },
  {
    key: "classes",
    label: "Classes",
    icon: Boxes,
    color: "#bb9af7",
    getValue: (nodes) => countByLabel(nodes, "Class"),
  },
  {
    key: "modules",
    label: "Modules",
    icon: Network,
    color: "#565f89",
    getValue: (nodes) => countByLabel(nodes, "Module"),
  },
  {
    key: "relations",
    label: "Relations",
    icon: ArrowRightLeft,
    color: "#7dcfff",
    getValue: (_nodes, edges) => edges.length,
  },
  {
    key: "languages",
    label: "Languages",
    icon: Languages,
    color: "#9ece6a",
    getValue: (nodes) => uniqueLanguages(nodes),
  },
  {
    key: "entryPoints",
    label: "Entry Points",
    icon: Zap,
    color: "#e0af68",
    getValue: (nodes) =>
      nodes.filter((n) => n.entryPointScore != null && n.entryPointScore > 0)
        .length,
  },
  {
    key: "processes",
    label: "Processes",
    icon: GitBranch,
    color: "#ff9e64",
    getValue: (nodes) => countByLabel(nodes, "Process"),
  },
  {
    key: "traced",
    label: "Traced",
    icon: Shield,
    color: "#73daca",
    getValue: (nodes) =>
      nodes.filter((n) => n.isTraced === true).length,
  },
];

// ─── Quality level ──────────────────────────────────────────────────

function qualityGradient(totalNodes: number): {
  bg: string;
  border: string;
  badge: string;
  badgeText: string;
  text: string;
} {
  if (totalNodes >= 500) {
    return {
      bg: "linear-gradient(135deg, rgba(158,206,106,0.10) 0%, rgba(158,206,106,0.04) 100%)",
      border: "rgba(158,206,106,0.25)",
      badge: "rgba(158,206,106,0.18)",
      badgeText: "#9ece6a",
      text: "Healthy",
    };
  }
  if (totalNodes >= 100) {
    return {
      bg: "linear-gradient(135deg, rgba(224,175,104,0.10) 0%, rgba(224,175,104,0.04) 100%)",
      border: "rgba(224,175,104,0.25)",
      badge: "rgba(224,175,104,0.18)",
      badgeText: "#e0af68",
      text: "Growing",
    };
  }
  return {
    bg: "linear-gradient(135deg, rgba(86,95,137,0.10) 0%, rgba(86,95,137,0.04) 100%)",
    border: "rgba(86,95,137,0.25)",
    badge: "rgba(86,95,137,0.18)",
    badgeText: "var(--text-2)",
    text: "Small",
  };
}

// ─── Component ──────────────────────────────────────────────────────

export function RepoDashboard() {
  const activeRepo = useAppStore((s) => s.activeRepo);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const setMode = useAppStore((s) => s.setMode);

  // Fetch graph data at symbol zoom level to get all nodes
  const { data, isLoading, error } = useQuery<GraphPayload>({
    queryKey: ["dashboard-graph", activeRepo],
    queryFn: () =>
      commands.getGraphData({
        zoomLevel: "symbol",
        maxNodes: 10_000,
      }),
    enabled: !!activeRepo,
    staleTime: 60_000,
  });

  const nodes = data?.nodes ?? [];
  const edges = data?.edges ?? [];
  const totalNodes = data?.stats?.nodeCount ?? nodes.length;
  const totalEdges = data?.stats?.edgeCount ?? edges.length;

  const distribution = useMemo(() => buildLabelDistribution(nodes), [nodes]);
  const topNodes = useMemo(() => topConnectedNodes(nodes, edges, 8), [nodes, edges]);
  const quality = useMemo(() => qualityGradient(totalNodes), [totalNodes]);
  const totalFiles = useMemo(() => countByLabel(nodes, "File"), [nodes]);
  const distributionTotal = useMemo(
    () => distribution.reduce((sum, d) => sum + d.count, 0),
    [distribution],
  );

  // ── Loading state ───────────────────────────────────────────────
  if (!activeRepo) {
    return (
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          height: "100%",
          color: "var(--text-4)",
          fontSize: 14,
        }}
      >
        Select a repository to view the dashboard.
      </div>
    );
  }

  if (isLoading) {
    return (
      <div style={{ padding: 24, display: "flex", flexDirection: "column", gap: 16 }}>
        <SkeletonBlock height="100px" />
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 12 }}>
          {Array.from({ length: 6 }).map((_, i) => (
            <SkeletonBlock key={i} height="96px" />
          ))}
        </div>
        <SkeletonBlock height="48px" />
        <SkeletonBlock height="200px" />
      </div>
    );
  }

  if (error) {
    return (
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          height: "100%",
          color: "var(--rose)",
          fontSize: 13,
          padding: 24,
          textAlign: "center",
        }}
      >
        Failed to load dashboard data.
      </div>
    );
  }

  // ── Navigate to graph with node selected ────────────────────────
  const navigateToNode = (nodeId: string, name: string) => {
    setSelectedNodeId(nodeId, name);
    setMode("explorer");
  };

  return (
    <div
      style={{
        padding: 24,
        overflowY: "auto",
        height: "100%",
        display: "flex",
        flexDirection: "column",
        gap: 20,
      }}
    >
      {/* ── Code Health Score ─────────────────────────────────── */}
      <CodeHealthCard />

      {/* ── Quality Banner ──────────────────────────────────────── */}
      <AnimatedCard
        style={{
          background: quality.bg,
          border: `1px solid ${quality.border}`,
          borderRadius: 14,
          padding: "20px 24px",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          flexWrap: "wrap",
          gap: 12,
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <div
            style={{
              fontSize: 18,
              fontWeight: 700,
              color: "var(--text-0)",
              letterSpacing: "-0.01em",
            }}
          >
            {activeRepo}
          </div>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 16,
              fontSize: 12,
              color: "var(--text-3)",
            }}
          >
            <span>
              <strong style={{ color: "var(--text-1)" }}>
                {totalNodes.toLocaleString()}
              </strong>{" "}
              nodes
            </span>
            <span>
              <strong style={{ color: "var(--text-1)" }}>
                {totalEdges.toLocaleString()}
              </strong>{" "}
              edges
            </span>
            <span>
              <strong style={{ color: "var(--text-1)" }}>
                {totalFiles.toLocaleString()}
              </strong>{" "}
              files
            </span>
          </div>
        </div>
        <div
          style={{
            padding: "4px 12px",
            borderRadius: 20,
            background: quality.badge,
            color: quality.badgeText,
            fontSize: 11,
            fontWeight: 600,
            letterSpacing: "0.03em",
            textTransform: "uppercase",
          }}
        >
          {quality.text}
        </div>
      </AnimatedCard>

      {/* ── Metric Cards Grid ───────────────────────────────────── */}
      <StaggerContainer
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(3, 1fr)",
          gap: 12,
        }}
      >
        {METRICS.map((m) => {
          const value = m.getValue(nodes, edges);
          const Icon = m.icon;
          return (
            <StaggerItem key={m.key}>
              <AnimatedCard
                style={{
                  background: "var(--bg-1)",
                  border: "1px solid var(--surface-border)",
                  borderRadius: 12,
                  padding: "16px 18px",
                  display: "flex",
                  alignItems: "center",
                  gap: 14,
                  cursor: "default",
                }}
              >
                <div
                  style={{
                    width: 40,
                    height: 40,
                    borderRadius: 10,
                    background: `${m.color}18`,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    flexShrink: 0,
                  }}
                >
                  <Icon size={20} style={{ color: m.color }} />
                </div>
                <div style={{ minWidth: 0 }}>
                  <div
                    style={{
                      fontSize: 22,
                      fontWeight: 700,
                      color: "var(--text-0)",
                      lineHeight: 1.1,
                    }}
                  >
                    <AnimatedCounter value={value} />
                  </div>
                  <div
                    style={{
                      fontSize: 11,
                      color: "var(--text-3)",
                      marginTop: 2,
                      fontWeight: 500,
                    }}
                  >
                    {m.label}
                  </div>
                </div>
              </AnimatedCard>
            </StaggerItem>
          );
        })}
      </StaggerContainer>

      {/* ── Node Type Distribution ──────────────────────────────── */}
      {distribution.length > 0 && (
        <div
          style={{
            background: "var(--bg-1)",
            border: "1px solid var(--surface-border)",
            borderRadius: 12,
            padding: "16px 18px",
          }}
        >
          <div
            style={{
              fontSize: 12,
              fontWeight: 600,
              color: "var(--text-2)",
              marginBottom: 12,
              textTransform: "uppercase",
              letterSpacing: "0.04em",
            }}
          >
            Node Type Distribution
          </div>

          {/* Stacked horizontal bar */}
          <div
            style={{
              display: "flex",
              height: 20,
              borderRadius: 6,
              overflow: "hidden",
              background: "var(--bg-3)",
            }}
          >
            {distribution.map((d) => {
              const pct = distributionTotal > 0 ? (d.count / distributionTotal) * 100 : 0;
              if (pct < 0.5) return null;
              return (
                <div
                  key={d.label}
                  title={`${d.label}: ${d.count}`}
                  style={{
                    width: `${pct}%`,
                    background: d.color,
                    opacity: 0.85,
                    transition: "width 0.4s ease",
                    minWidth: pct > 0 ? 2 : 0,
                  }}
                />
              );
            })}
          </div>

          {/* Legend */}
          <div
            style={{
              display: "flex",
              flexWrap: "wrap",
              gap: "6px 16px",
              marginTop: 12,
            }}
          >
            {distribution.map((d) => (
              <div
                key={d.label}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                  fontSize: 11,
                  color: "var(--text-3)",
                }}
              >
                <div
                  style={{
                    width: 8,
                    height: 8,
                    borderRadius: "50%",
                    background: d.color,
                    flexShrink: 0,
                  }}
                />
                <span style={{ color: "var(--text-2)", fontWeight: 500 }}>
                  {d.label}
                </span>
                <span>{d.count.toLocaleString()}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* ── Top Connected Nodes ─────────────────────────────────── */}
      {topNodes.length > 0 && (
        <div
          style={{
            background: "var(--bg-1)",
            border: "1px solid var(--surface-border)",
            borderRadius: 12,
            padding: "16px 18px",
          }}
        >
          <div
            style={{
              fontSize: 12,
              fontWeight: 600,
              color: "var(--text-2)",
              marginBottom: 12,
              textTransform: "uppercase",
              letterSpacing: "0.04em",
            }}
          >
            Top Connected Nodes
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
            {topNodes.map(({ node, degree }) => {
              const labelColor = LABEL_COLORS[node.label] ?? "#565f89";
              return (
                <div
                  key={node.id}
                  onClick={() => navigateToNode(node.id, node.name)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 10,
                    padding: "8px 10px",
                    borderRadius: 8,
                    cursor: "pointer",
                    transition: "background 0.15s",
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLDivElement).style.background =
                      "var(--bg-2)";
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLDivElement).style.background =
                      "transparent";
                  }}
                >
                  {/* Icon dot */}
                  <div
                    style={{
                      width: 8,
                      height: 8,
                      borderRadius: "50%",
                      background: labelColor,
                      flexShrink: 0,
                    }}
                  />

                  {/* Name */}
                  <span
                    style={{
                      fontSize: 13,
                      fontWeight: 600,
                      color: "var(--text-1)",
                      flex: 1,
                      minWidth: 0,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {node.name}
                  </span>

                  {/* Type badge */}
                  <span
                    style={{
                      fontSize: 10,
                      fontWeight: 600,
                      padding: "2px 8px",
                      borderRadius: 10,
                      background: `${labelColor}20`,
                      color: labelColor,
                      textTransform: "uppercase",
                      letterSpacing: "0.04em",
                      flexShrink: 0,
                    }}
                  >
                    {node.label}
                  </span>

                  {/* File path */}
                  <span
                    style={{
                      fontSize: 11,
                      color: "var(--text-4)",
                      maxWidth: 200,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                      flexShrink: 1,
                    }}
                  >
                    {node.filePath}
                  </span>

                  {/* Degree count */}
                  <span
                    style={{
                      fontSize: 11,
                      fontWeight: 700,
                      color: "var(--text-2)",
                      fontVariantNumeric: "tabular-nums",
                      flexShrink: 0,
                      minWidth: 32,
                      textAlign: "right",
                    }}
                  >
                    {degree}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* ── Top Complex Functions ──────────────────────────────── */}
      {data && (() => {
        const complexNodes = data.nodes
          .filter(n => n.complexity && n.complexity > 5)
          .sort((a, b) => (b.complexity || 0) - (a.complexity || 0))
          .slice(0, 8);

        if (complexNodes.length === 0) return null;

        return (
          <AnimatedCard>
            <div style={{ padding: "16px 20px", borderRadius: "var(--radius-lg)", border: "1px solid var(--border)", background: "var(--bg-1)" }}>
              <h3 className="text-sm font-semibold" style={{ color: "var(--text-0)", marginBottom: 12 }}>
                Most Complex Functions
              </h3>
              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                {complexNodes.map(n => (
                  <button
                    key={n.id}
                    onClick={() => { setSelectedNodeId(n.id, n.name); setMode("explorer"); }}
                    className="flex items-center gap-2 text-left rounded-md transition-colors"
                    style={{ padding: "6px 8px", fontSize: 11 }}
                    onMouseEnter={e => (e.currentTarget.style.background = "var(--bg-2)")}
                    onMouseLeave={e => (e.currentTarget.style.background = "transparent")}
                  >
                    <span className="font-mono font-medium" style={{ color: "var(--text-0)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {n.name}
                    </span>
                    <span className="text-[10px] font-medium px-1.5 py-0.5 rounded-full" style={{
                      background: (n.complexity || 0) > 20 ? "#ef444420" : (n.complexity || 0) > 10 ? "#f59e0b20" : "#22c55e20",
                      color: (n.complexity || 0) > 20 ? "#ef4444" : (n.complexity || 0) > 10 ? "#f59e0b" : "#22c55e",
                    }}>
                      CC:{n.complexity}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          </AnimatedCard>
        );
      })()}
    </div>
  );
}
