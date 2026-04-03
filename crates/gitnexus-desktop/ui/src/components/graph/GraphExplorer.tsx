import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { AlertCircle, Copy, EyeOff, Network, Zap } from "lucide-react";
import { useGraphData } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";
import { useSigma } from "../../hooks/use-sigma";
import { commands } from "../../lib/tauri-commands";
import {
  buildGraphologyGraph,
  filterGraphByDepth,
  NODE_COLORS,
} from "../../lib/graph-adapter";
import { GraphToolbar } from "./GraphToolbar";
import { NodeHoverCard } from "./NodeHoverCard";
import { ViewModeToggle, type ViewMode } from "./ViewModeToggle";
import { TreemapView } from "./TreemapView";
import { useI18n } from "../../hooks/use-i18n";
import { Tooltip } from "../shared/Tooltip";
import { LoadingOrbs } from "../shared/LoadingOrbs";
import { CypherQueryFAB } from "./CypherQueryFAB";
import { ProcessFlowModal } from "./ProcessFlowModal";
import type { GraphFilter } from "../../lib/tauri-commands";

// ─── Label colors (used only for the legend + context menu dot) ────
const LABEL_COLORS = NODE_COLORS;

export function GraphExplorer() {
  const { t, tt } = useI18n();
  const zoomLevel = useAppStore((s) => s.zoomLevel);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const setSidebarTab = useAppStore((s) => s.setSidebarTab);
  const setSearchOpen = useAppStore((s) => s.setSearchOpen);
  const setSearchQuery = useAppStore((s) => s.setSearchQuery);
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const searchMatchIds = useAppStore((s) => s.searchMatchIds);

  // ── Local state ──────────────────────────────────────────────────
  const [viewMode, setViewMode] = useState<ViewMode>("graph");
  const [legendExpanded, setLegendExpanded] = useState(false);
  const [flowsOpen, setFlowsOpen] = useState(false);
  const [minimapVisible, setMinimapVisible] = useState(true);
  const [minimapOpacity, setMinimapOpacity] = useState(0.8);
  const [impactOverlay, setImpactOverlay] = useState(false);
  const [focusNodeId, setFocusNodeId] = useState<string | null>(null);
  const [hiddenEdgeTypes, setHiddenEdgeTypes] = useState<Set<string>>(
    new Set(["IMPORTS", "HAS_METHOD", "HAS_PROPERTY", "CONTAINS"]),
  );
  const [depthFilter, setDepthFilter] = useState<number | null>(null);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [layout, setLayout] = useState("forceatlas2");

  // Context menu
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    nodeId: string;
    name: string;
    filePath: string;
  } | null>(null);

  // Hover card
  const [hoveredNode, setHoveredNode] = useState<{
    id: string;
    name: string;
    label: string;
    filePath: string;
    startLine?: number;
    endLine?: number;
    parameterCount?: number;
    returnType?: string;
    isTraced?: boolean;
    isDeadCandidate?: boolean;
    complexity?: number;
  } | null>(null);
  const [hoverPos, setHoverPos] = useState<{ x: number; y: number } | null>(
    null,
  );
  const [hoverDegrees, setHoverDegrees] = useState<{
    inDeg: number;
    outDeg: number;
  }>({ inDeg: 0, outDeg: 0 });

  // Impact node map (nodeId -> depth) for the sigma reducer
  const [impactNodeIds, setImpactNodeIds] = useState<Map<string, number>>(
    new Map(),
  );

  // Search highlighted set
  const highlightedNodeIds = useMemo(
    () => new Set(searchMatchIds),
    [searchMatchIds],
  );

  // ── Minimap canvas ref ───────────────────────────────────────────
  const minimapCanvasRef = useRef<HTMLCanvasElement | null>(null);

  // ── Sigma hook ───────────────────────────────────────────────────
  const {
    containerRef,
    graphRef,
    isLayoutRunning,
    setGraph,
    runLayout,
    focusNode,
    fitView,
    zoomIn,
    zoomOut,
    exportPNG,
    refresh,
    sigmaRef,
  } = useSigma({
    selectedNodeId,
    highlightedNodeIds,
    impactNodeIds: impactOverlay ? impactNodeIds : undefined,
    onNodeClick: useCallback(
      (nodeId: string | null) => {
        if (nodeId) {
          const g = graphRef.current;
          const name = g?.hasNode(nodeId)
            ? g.getNodeAttribute(nodeId, "label")
            : null;
          setSelectedNodeId(nodeId, name ?? null);
        } else {
          setSelectedNodeId(null, null);
          setContextMenu(null);
        }
      },
      // eslint-disable-next-line react-hooks/exhaustive-deps
      [setSelectedNodeId],
    ),
    onNodeHover: useCallback(
      (nodeId: string | null) => {
        if (!nodeId) {
          setHoveredNode(null);
          setHoverPos(null);
          return;
        }
        const g = graphRef.current;
        const sigma = sigmaRef.current;
        if (!g || !sigma || !g.hasNode(nodeId)) return;
        const attrs = g.getNodeAttributes(nodeId);
        const viewportPos = sigma.graphToViewport({ x: attrs.x, y: attrs.y });
        setHoveredNode({
          id: nodeId,
          name: attrs.label,
          label: attrs.nodeType,
          filePath: attrs.filePath,
          startLine: attrs.startLine,
          endLine: attrs.endLine,
          parameterCount: attrs.parameterCount,
          returnType: attrs.returnType,
          isTraced: attrs.isTraced,
          isDeadCandidate: attrs.isDeadCandidate,
          complexity: attrs.complexity,
        });
        setHoverPos({ x: viewportPos.x, y: viewportPos.y });
        setHoverDegrees({
          inDeg: g.inDegree(nodeId),
          outDeg: g.outDegree(nodeId),
        });
      },
      // eslint-disable-next-line react-hooks/exhaustive-deps
      [],
    ),
    onNodeRightClick: useCallback(
      (nodeId: string, x: number, y: number) => {
        const g = graphRef.current;
        if (!g || !g.hasNode(nodeId)) return;
        const attrs = g.getNodeAttributes(nodeId);
        setContextMenu({
          x,
          y,
          nodeId,
          name: attrs.label,
          filePath: attrs.filePath,
        });
      },
      // eslint-disable-next-line react-hooks/exhaustive-deps
      [],
    ),
    onNodeDoubleClick: useCallback(
      (nodeId: string) => {
        setFocusNodeId(nodeId);
      },
      [],
    ),
  });

  // ── Data fetching ────────────────────────────────────────────────
  const filter: GraphFilter = { zoomLevel, maxNodes: 200 };
  const { data, isLoading, error } = useGraphData(filter, true);

  const { data: subgraphData } = useQuery({
    queryKey: ["subgraph", focusNodeId],
    queryFn: () => commands.getSubgraph(focusNodeId!, 2),
    enabled: !!focusNodeId,
    staleTime: 30_000,
  });

  const activeData = focusNodeId && subgraphData ? subgraphData : data;

  // ── Build + set graph when data changes ──────────────────────────
  const prevDataKeyRef = useRef("");
  useEffect(() => {
    if (!activeData || activeData.nodes.length === 0) return;
    const key = `${activeData.stats.nodeCount}-${activeData.stats.edgeCount}-${zoomLevel}-${focusNodeId ?? ""}-${[...hiddenEdgeTypes].sort().join(",")}`;
    if (key === prevDataKeyRef.current) return;
    prevDataKeyRef.current = key;

    const graph = buildGraphologyGraph(
      activeData.nodes,
      activeData.edges,
      hiddenEdgeTypes,
    );
    setGraph(graph);
    runLayout();
  }, [activeData, zoomLevel, hiddenEdgeTypes, focusNodeId, setGraph, runLayout]);

  // ── Depth filter ─────────────────────────────────────────────────
  useEffect(() => {
    const graph = graphRef.current;
    if (!graph || graph.order === 0) return;
    filterGraphByDepth(graph, selectedNodeId ?? null, depthFilter);
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [depthFilter, selectedNodeId, refresh]);

  // ── Camera animation when selected node changes ──────────────────
  useEffect(() => {
    if (selectedNodeId) {
      focusNode(selectedNodeId);
    }
    refresh();
  }, [selectedNodeId, focusNode, refresh]);

  // ── Refresh when search highlights change ────────────────────────
  useEffect(() => {
    refresh();
  }, [searchMatchIds, refresh]);

  // ── Impact overlay toggle ────────────────────────────────────────
  const toggleImpactOverlay = useCallback(async () => {
    if (impactOverlay) {
      setImpactNodeIds(new Map());
      setImpactOverlay(false);
      refresh();
      return;
    }

    if (!selectedNodeId) return;

    try {
      const result = await commands.getImpactAnalysis(
        selectedNodeId,
        "both",
        3,
      );
      const map = new Map<string, number>();
      map.set(selectedNodeId, 0);

      const markNodes = (items: Array<{ node: { id: string }; depth: number }>) => {
        for (const item of items) {
          if (!map.has(item.node.id)) {
            map.set(item.node.id, item.depth);
          }
        }
      };

      if (result.upstream) markNodes(result.upstream);
      if (result.downstream) markNodes(result.downstream);

      setImpactNodeIds(map);
      setImpactOverlay(true);
      refresh();
    } catch {
      // silently fail
    }
  }, [selectedNodeId, impactOverlay, refresh]);

  // ── Keyboard shortcuts ───────────────────────────────────────────
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "g") {
        e.preventDefault();
        setSearchOpen(true);
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "e") {
        e.preventDefault();
        exportPNG();
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === "=" || e.key === "+")) {
        e.preventDefault();
        zoomIn();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "-") {
        e.preventDefault();
        zoomOut();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "0") {
        e.preventDefault();
        fitView();
      }
      if (e.key === "Escape" && !e.ctrlKey && !e.metaKey) {
        setSelectedNodeId(null);
        setContextMenu(null);
        setShortcutsOpen(false);
      }
      if (e.key === "?" && !e.ctrlKey && !e.metaKey) {
        setShortcutsOpen((prev) => !prev);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [exportPNG, zoomIn, zoomOut, fitView, setSearchOpen, setSelectedNodeId]);

  // Listen for custom events (F=fit, L=cycle layout)
  useEffect(() => {
    const onFit = () => fitView();
    const onCycle = () => {
      const layouts = ["forceatlas2", "grid", "circle", "random"];
      const idx = layouts.indexOf(layout);
      const next = layouts[(idx + 1) % layouts.length];
      setLayout(next);
    };
    window.addEventListener("gitnexus:fit-graph", onFit);
    window.addEventListener("gitnexus:cycle-layout", onCycle);
    return () => {
      window.removeEventListener("gitnexus:fit-graph", onFit);
      window.removeEventListener("gitnexus:cycle-layout", onCycle);
    };
  }, [layout, fitView]);

  // ── Minimap drawing ──────────────────────────────────────────────
  const drawMinimap = useCallback(() => {
    const canvas = minimapCanvasRef.current;
    const sigma = sigmaRef.current;
    const graph = graphRef.current;
    if (!canvas || !sigma || !graph || graph.order === 0) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const canvasW = 160;
    const canvasH = 120;
    ctx.fillStyle = "#1a1b26";
    ctx.fillRect(0, 0, canvasW, canvasH);

    // Compute graph bounding box
    let minX = Infinity,
      minY = Infinity,
      maxX = -Infinity,
      maxY = -Infinity;
    graph.forEachNode((_n, attrs) => {
      if (attrs.x < minX) minX = attrs.x;
      if (attrs.y < minY) minY = attrs.y;
      if (attrs.x > maxX) maxX = attrs.x;
      if (attrs.y > maxY) maxY = attrs.y;
    });

    const graphW = maxX - minX || 1;
    const graphH = maxY - minY || 1;
    const padding = 8;
    const innerW = canvasW - padding * 2;
    const innerH = canvasH - padding * 2;
    const scale = Math.min(innerW / graphW, innerH / graphH);

    graph.forEachNode((_n, attrs) => {
      const x = padding + (attrs.x - minX) * scale;
      const y = padding + (attrs.y - minY) * scale;
      ctx.fillStyle = attrs.color || "#565f89";
      ctx.beginPath();
      ctx.arc(x, y, 2, 0, Math.PI * 2);
      ctx.fill();
    });

    // Draw viewport rectangle
    const cam = sigma.getCamera().getState();
    const dim = sigma.getDimensions();
    const vpCX = cam.x;
    const vpCY = cam.y;
    const vpHW = (cam.ratio * dim.width) / (2 * dim.width);
    const vpHH = (cam.ratio * dim.height) / (2 * dim.height);

    // Map normalized viewport to minimap coords
    const normToMiniX = (nx: number) => padding + (nx * graphW + (graphW * 0.5 - graphW * 0.5)) * scale;
    const normToMiniY = (ny: number) => padding + (ny * graphH + (graphH * 0.5 - graphH * 0.5)) * scale;

    const vpX = normToMiniX(vpCX - vpHW);
    const vpY = normToMiniY(vpCY - vpHH);
    const vpW2 = vpHW * 2 * graphW * scale;
    const vpH2 = vpHH * 2 * graphH * scale;

    ctx.strokeStyle = "#7aa2f7";
    ctx.lineWidth = 1.5;
    ctx.fillStyle = "rgba(122, 162, 247, 0.12)";
    ctx.fillRect(vpX, vpY, vpW2, vpH2);
    ctx.strokeRect(vpX, vpY, vpW2, vpH2);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Redraw minimap on layout changes
  const minimapRafRef = useRef<number | null>(null);
  const drawMinimapThrottled = useCallback(() => {
    if (minimapRafRef.current) return;
    minimapRafRef.current = requestAnimationFrame(() => {
      drawMinimap();
      minimapRafRef.current = null;
    });
  }, [drawMinimap]);

  // Attach minimap redraw to sigma afterRender
  useEffect(() => {
    const sigma = sigmaRef.current;
    if (!sigma) return;
    sigma.on("afterRender", drawMinimapThrottled);
    return () => {
      sigma.removeListener("afterRender", drawMinimapThrottled);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [drawMinimapThrottled]);

  // ── Toolbar callbacks ────────────────────────────────────────────
  const handleFit = useCallback(() => fitView(), [fitView]);
  const handleExportPNG = useCallback(() => exportPNG(), [exportPNG]);

  const handleLayoutChange = useCallback(
    (_newLayout: string) => {
      setLayout(_newLayout);
      // For sigma, we always use FA2 layout; the layout name is cosmetic
      // (Sigma doesn't have built-in grid/circle/breadthfirst like Cytoscape)
      runLayout();
    },
    [runLayout],
  );

  const handleToggleEdgeType = useCallback((type: string) => {
    setHiddenEdgeTypes((prev) => {
      const next = new Set(prev);
      if (next.has(type)) next.delete(type);
      else next.add(type);
      return next;
    });
  }, []);

  // ── Shared toolbar props ─────────────────────────────────────────
  const toolbarProps = {
    stats: data?.stats,
    layout,
    onLayoutChange: handleLayoutChange,
    onFit: handleFit,
    onExport: handleExportPNG,
    hiddenEdgeTypes,
    onToggleEdgeType: handleToggleEdgeType,
    depthFilter,
    onDepthFilterChange: setDepthFilter,
  };

  // ── Loading state ────────────────────────────────────────────────
  if (isLoading) {
    return (
      <div className="h-full flex flex-col">
        <GraphToolbar {...toolbarProps} />
        <div className="flex-1">
          <LoadingOrbs label={t("graph.loadingGraph")} />
        </div>
      </div>
    );
  }

  // ── Empty state ──────────────────────────────────────────────────
  if (data && data.nodes.length === 0) {
    return (
      <div className="h-full flex flex-col">
        <GraphToolbar {...toolbarProps} />
        <div
          className="flex-1 relative flex flex-col items-center justify-center gap-4 overflow-hidden"
          style={{ backgroundColor: "var(--bg-1)", color: "var(--text-3)" }}
        >
          <div
            className="flex items-center justify-center"
            style={{
              width: "96px",
              height: "96px",
              borderRadius: "var(--radius-md)",
              backgroundColor: "var(--bg-3)",
              border: "2px dashed var(--surface-border)",
            }}
          >
            <Network size={64} style={{ color: "var(--text-4)" }} />
          </div>
          <p className="text-lg font-medium">{t("graph.noData")}</p>
          <p className="text-sm">{t("graph.analyzeFirst")}</p>
        </div>
      </div>
    );
  }

  // ── Error state ──────────────────────────────────────────────────
  if (error) {
    return (
      <div className="h-full flex flex-col">
        <GraphToolbar {...toolbarProps} />
        <div
          className="flex-1 relative flex items-center justify-center"
          style={{ backgroundColor: "var(--bg-1)" }}
        >
          <div className="flex flex-col items-center gap-3">
            <div
              className="p-3 rounded-lg"
              style={{ backgroundColor: "var(--rose)", opacity: 0.15 }}
            >
              <AlertCircle size={24} style={{ color: "var(--rose)" }} />
            </div>
            <div className="text-center">
              <p
                style={{
                  color: "var(--text-2)",
                  fontSize: "14px",
                  fontWeight: "500",
                  marginBottom: "4px",
                }}
              >
                {t("graph.failedToLoad")}
              </p>
              <p style={{ color: "var(--text-4)", fontSize: "12px" }}>
                {String(error)}
              </p>
            </div>
          </div>
        </div>
      </div>
    );
  }

  // ─── Main render ─────────────────────────────────────────────────
  return (
    <div className="h-full flex flex-col">
      {/* Toolbar + view mode toggle */}
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <div style={{ flex: 1 }}>
          <GraphToolbar
            {...toolbarProps}
            onFlows={() => setFlowsOpen(true)}
          />
        </div>
        <div style={{ paddingRight: 12 }}>
          <ViewModeToggle mode={viewMode} onChange={setViewMode} />
        </div>
      </div>

      {viewMode === "treemap" ? (
        <div className="flex-1 relative">
          <TreemapView data={data} isLoading={isLoading} />
        </div>
      ) : (
        <div className="flex-1 relative" style={{ backgroundColor: "#06060a" }}>
          {/* Sigma container */}
          <div
            ref={containerRef}
            className="absolute inset-0"
            style={{ cursor: "grab" }}
            role="img"
            aria-label="Knowledge graph visualization"
          />

          {/* Layout computing overlay */}
          {isLayoutRunning && (
            <div
              className="absolute inset-0 z-30 flex items-center justify-center"
              style={{
                backgroundColor: "rgba(9, 11, 16, 0.5)",
                backdropFilter: "blur(2px)",
              }}
            >
              <div style={{ color: "var(--text-2)", fontSize: 13 }}>
                {t("graph.computingLayout")}
              </div>
            </div>
          )}

          {/* Focus mode: back button */}
          {focusNodeId && (
            <button
              onClick={() => setFocusNodeId(null)}
              className="absolute top-16 left-4 z-20 rounded-lg text-xs font-medium"
              style={{
                padding: "6px 12px",
                background: "var(--accent)",
                color: "white",
                border: "none",
                cursor: "pointer",
              }}
            >
              &larr; {t("graph.backToFull")}
            </button>
          )}

          {/* Truncation banner */}
          {data?.stats.truncated && !focusNodeId && (
            <div
              className="absolute top-16 left-4 right-4 z-20 rounded-lg text-xs"
              style={{
                padding: "8px 12px",
                background: "var(--bg-2)",
                border: "1px solid var(--surface-border)",
                color: "var(--text-2)",
                textAlign: "center",
              }}
            >
              {t("graph.showingTopNodes").replace(
                "{0}",
                String(data.stats.nodeCount),
              )}
            </div>
          )}

          {/* Node hover card */}
          <NodeHoverCard
            node={hoveredNode}
            position={hoverPos}
            inDegree={hoverDegrees.inDeg}
            outDegree={hoverDegrees.outDeg}
            onViewSource={() => {
              if (hoveredNode) {
                setSidebarTab("files");
                setSelectedNodeId(
                  "File:" + hoveredNode.filePath,
                  hoveredNode.name,
                );
              }
            }}
            onImpact={() => {
              if (hoveredNode) {
                setSelectedNodeId(hoveredNode.id, hoveredNode.name);
                setSidebarTab("impact");
              }
            }}
          />

          {/* Context menu */}
          {contextMenu && (
            <div
              className="absolute z-50 pointer-events-auto rounded-lg text-xs"
              style={{
                left: contextMenu.x,
                top: contextMenu.y,
                backgroundColor: "var(--bg-3)",
                border: "1px solid var(--surface-border-hover)",
                boxShadow: "var(--shadow-lg)",
                minWidth: "200px",
                overflow: "hidden",
              }}
            >
              <Tooltip content={tt("graph.contextMenu.goToDefinition").tip}>
                <ContextMenuButton
                  onClick={() => {
                    setSidebarTab("files");
                    setSelectedNodeId(
                      "File:" + contextMenu.filePath,
                      contextMenu.name,
                    );
                    setContextMenu(null);
                  }}
                >
                  {tt("graph.contextMenu.goToDefinition").label}
                </ContextMenuButton>
              </Tooltip>
              <Tooltip content={tt("graph.contextMenu.findReferences").tip}>
                <ContextMenuButton
                  onClick={() => {
                    setSearchQuery(contextMenu.name);
                    setSearchOpen(true);
                    setContextMenu(null);
                  }}
                >
                  {tt("graph.contextMenu.findReferences").label}
                </ContextMenuButton>
              </Tooltip>
              <div
                style={{
                  borderTop: "1px solid var(--surface-border)",
                  margin: "4px 0",
                }}
              />
              {/* View Impact */}
              <ContextMenuButton
                onClick={() => {
                  setSelectedNodeId(contextMenu.nodeId, contextMenu.name);
                  setSidebarTab("impact");
                  setContextMenu(null);
                }}
              >
                {t("graph.viewImpact")}
              </ContextMenuButton>
              <Tooltip content={tt("graph.contextMenu.expandNeighbors").tip}>
                <ContextMenuButton onClick={() => setContextMenu(null)}>
                  {tt("graph.contextMenu.expandNeighbors").label}
                </ContextMenuButton>
              </Tooltip>
              <Tooltip content={tt("graph.contextMenu.hideNode").tip}>
                <ContextMenuButton
                  onClick={() => {
                    const g = graphRef.current;
                    if (g && g.hasNode(contextMenu.nodeId)) {
                      g.dropNode(contextMenu.nodeId);
                      refresh();
                    }
                    setContextMenu(null);
                  }}
                >
                  {tt("graph.contextMenu.hideNode").label}
                </ContextMenuButton>
              </Tooltip>
              <div
                style={{
                  borderTop: "1px solid var(--surface-border)",
                  margin: "4px 0",
                }}
              />
              <Tooltip content={tt("graph.contextMenu.copyName").tip}>
                <ContextMenuButton
                  onClick={() => {
                    navigator.clipboard.writeText(contextMenu.name);
                    setContextMenu(null);
                  }}
                >
                  <Copy size={14} style={{ marginRight: "8px" }} />
                  {tt("graph.contextMenu.copyName").label}
                </ContextMenuButton>
              </Tooltip>
              <Tooltip content={tt("graph.contextMenu.copyFilePath").tip}>
                <ContextMenuButton
                  onClick={() => {
                    navigator.clipboard.writeText(contextMenu.filePath);
                    setContextMenu(null);
                  }}
                >
                  <Copy size={14} style={{ marginRight: "8px" }} />
                  {tt("graph.contextMenu.copyFilePath").label}
                </ContextMenuButton>
              </Tooltip>
            </div>
          )}

          {/* Minimap overlay */}
          {minimapVisible && (
            <div
              className="absolute z-15 pointer-events-auto"
              style={{
                bottom: "16px",
                left: "16px",
                borderRadius: "var(--radius-md)",
                backgroundColor: "var(--bg-2)",
                border: "1px solid var(--surface-border)",
                opacity: minimapOpacity,
                transition: "opacity 0.2s ease",
              }}
              onMouseEnter={() => setMinimapOpacity(1.0)}
              onMouseLeave={() => setMinimapOpacity(0.8)}
            >
              <canvas
                ref={minimapCanvasRef}
                width={160}
                height={120}
                style={{
                  display: "block",
                  cursor: "pointer",
                  borderRadius: "var(--radius-md)",
                }}
              />
              <button
                onClick={() => setMinimapVisible(false)}
                className="absolute transition-colors"
                style={{
                  top: "4px",
                  right: "4px",
                  padding: "4px",
                  backgroundColor: "rgba(0, 0, 0, 0.5)",
                  borderRadius: "4px",
                  color: "var(--text-3)",
                  border: "none",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.backgroundColor =
                    "rgba(0, 0, 0, 0.7)";
                  e.currentTarget.style.color = "var(--text-0)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.backgroundColor =
                    "rgba(0, 0, 0, 0.5)";
                  e.currentTarget.style.color = "var(--text-3)";
                }}
              >
                <EyeOff size={12} />
              </button>
            </div>
          )}

          {/* Impact overlay toggle */}
          {selectedNodeId && (
            <button
              onClick={toggleImpactOverlay}
              className="absolute z-20 flex items-center gap-1.5 rounded-lg transition-all"
              style={{
                bottom: 70,
                left: 16,
                padding: "8px 14px",
                background: impactOverlay ? "#f7768e" : "var(--bg-2)",
                color: impactOverlay ? "white" : "var(--text-2)",
                border: `1px solid ${impactOverlay ? "#f7768e" : "var(--surface-border)"}`,
                backdropFilter: "blur(12px)",
                fontSize: 11,
                fontWeight: 600,
                cursor: "pointer",
                boxShadow: impactOverlay
                  ? "0 0 20px rgba(247, 118, 142, 0.3)"
                  : "var(--shadow-sm)",
              }}
            >
              <Zap size={13} />
              {impactOverlay ? t("graph.clearImpact") : t("graph.impactOverlay")}
            </button>
          )}

          {/* Cypher query FAB */}
          <CypherQueryFAB />

          {/* Keyboard shortcuts overlay */}
          {shortcutsOpen && (
            <div
              className="absolute z-30 rounded-xl"
              style={{
                top: 60,
                right: 16,
                padding: "16px 20px",
                background: "var(--bg-2)",
                border: "1px solid var(--surface-border)",
                backdropFilter: "blur(12px)",
                boxShadow: "var(--shadow-lg)",
                fontSize: 11,
                color: "var(--text-2)",
                minWidth: 220,
              }}
            >
              <div
                style={{
                  fontWeight: 600,
                  color: "var(--text-0)",
                  marginBottom: 8,
                  fontSize: 12,
                }}
              >
                {t("graph.keyboardShortcuts")}
              </div>
              {[
                ["Ctrl+G", t("graph.shortcut.goToSymbol")],
                ["Ctrl+E", t("graph.shortcut.exportPng")],
                ["Ctrl+Shift+S", t("graph.shortcut.screenshot")],
                [
                  "Ctrl+=/\u2212/0",
                  t("graph.shortcut.zoomInOutFit"),
                ],
                [
                  "Alt+\u2190/\u2192",
                  t("graph.shortcut.navigateBackForward"),
                ],
                ["Escape", t("graph.shortcut.clearSelection")],
                ["Double-click", t("graph.shortcut.focusSubgraph")],
                ["?", t("graph.shortcut.toggleHelp")],
              ].map(([key, desc]) => (
                <div
                  key={key}
                  className="flex justify-between py-1"
                  style={{ gap: 16 }}
                >
                  <kbd
                    className="font-mono text-[10px] rounded px-1.5 py-0.5"
                    style={{
                      background: "var(--bg-3)",
                      color: "var(--text-1)",
                    }}
                  >
                    {key}
                  </kbd>
                  <span>{desc}</span>
                </div>
              ))}
            </div>
          )}

          {/* Zoom Controls */}
          <div
            className="absolute z-20 flex flex-col gap-1"
            style={{ bottom: legendExpanded ? 200 : 80, right: 16 }}
          >
            <button
              onClick={zoomIn}
              className="w-8 h-8 rounded-lg flex items-center justify-center text-sm font-bold"
              style={{
                background: "var(--bg-2)",
                border: "1px solid var(--surface-border)",
                color: "var(--text-2)",
                cursor: "pointer",
              }}
              title="Zoom in (Ctrl+=)"
            >
              +
            </button>
            <button
              onClick={zoomOut}
              className="w-8 h-8 rounded-lg flex items-center justify-center text-sm font-bold"
              style={{
                background: "var(--bg-2)",
                border: "1px solid var(--surface-border)",
                color: "var(--text-2)",
                cursor: "pointer",
              }}
              title="Zoom out (Ctrl+-)"
            >
              {"\u2212"}
            </button>
            <button
              onClick={fitView}
              className="w-8 h-8 rounded-lg flex items-center justify-center text-[10px] font-bold"
              style={{
                background: "var(--bg-2)",
                border: "1px solid var(--surface-border)",
                color: "var(--text-2)",
                cursor: "pointer",
              }}
              title="Fit view (Ctrl+0)"
            >
              {"\u229E"}
            </button>
          </div>

          {/* Legend overlay */}
          <div
            className="absolute z-15 pointer-events-auto"
            style={{
              bottom: "12px",
              right: "12px",
              borderRadius: "var(--radius-md)",
              backgroundColor: "var(--bg-2)",
              backdropFilter: "blur(12px)",
              border: "1px solid var(--surface-border)",
              padding: "8px 12px",
            }}
          >
            {!legendExpanded ? (
              <button
                onClick={() => setLegendExpanded(true)}
                className="uppercase text-[10px] font-semibold transition-colors"
                style={{ color: "var(--text-3)" }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.color = "var(--text-2)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.color = "var(--text-3)";
                }}
              >
                {t("graph.legend")}
              </button>
            ) : (
              <div>
                <div className="flex items-center justify-between mb-2">
                  <span
                    className="uppercase text-[10px] font-semibold"
                    style={{ color: "var(--text-3)" }}
                  >
                    {t("graph.legend")}
                  </span>
                  <button
                    onClick={() => setLegendExpanded(false)}
                    className="ml-2 text-xs transition-colors"
                    style={{ color: "var(--text-3)" }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.color = "var(--text-2)";
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.color = "var(--text-3)";
                    }}
                  >
                    x
                  </button>
                </div>
                <div
                  className="space-y-1 max-h-[calc(8*28px)] overflow-y-auto"
                  style={{ maxWidth: "180px" }}
                >
                  {data &&
                    data.nodes.length > 0 &&
                    (() => {
                      const labelCounts = new Map<string, number>();
                      data.nodes.forEach((node) => {
                        labelCounts.set(
                          node.label,
                          (labelCounts.get(node.label) || 0) + 1,
                        );
                      });
                      const sortedEntries = Array.from(
                        labelCounts.entries(),
                      ).sort(
                        (a, b) => b[1] - a[1] || a[0].localeCompare(b[0]),
                      );
                      return sortedEntries.map(([type, count]) => (
                        <div
                          key={type}
                          className="flex items-center gap-2"
                          style={{ padding: "4px 0" }}
                        >
                          <span
                            className="w-2.5 h-2.5 rounded-full flex-shrink-0"
                            style={{
                              backgroundColor:
                                LABEL_COLORS[type] || "#565f89",
                            }}
                          />
                          <span
                            className="text-[11px] truncate"
                            style={{ color: "var(--text-1)" }}
                          >
                            {type}
                          </span>
                          <span
                            className="text-[10px] ml-auto flex-shrink-0"
                            style={{ color: "var(--text-3)" }}
                          >
                            {count}
                          </span>
                        </div>
                      ));
                    })()}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Process Flow Modal */}
      <ProcessFlowModal open={flowsOpen} onClose={() => setFlowsOpen(false)} />
    </div>
  );
}

// ─── Context menu button helper ────────────────────────────────────

function ContextMenuButton({
  onClick,
  children,
}: {
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className="w-full text-left transition-colors flex items-center"
      style={{
        padding: "8px 16px",
        color: "var(--text-2)",
        backgroundColor: "var(--bg-3)",
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.backgroundColor = "var(--surface-hover)";
        e.currentTarget.style.color = "var(--text-0)";
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.backgroundColor = "var(--bg-3)";
        e.currentTarget.style.color = "var(--text-2)";
      }}
    >
      {children}
    </button>
  );
}
