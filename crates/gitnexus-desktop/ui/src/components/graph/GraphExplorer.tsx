import { lazy, Suspense, useCallback, useMemo, useRef, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { Zap } from "lucide-react";
import { useGraphData } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";
import { useChatStore } from "../../stores/chat-store";
import { useSigma } from "../../hooks/use-sigma";
import { commands } from "../../lib/tauri-commands";
import { buildGraphologyGraph } from "../../lib/graph-adapter";
import { NodeHoverCard } from "./NodeHoverCard";
import { useI18n } from "../../hooks/use-i18n";
import { LENS_EDGE_TYPES } from "../explorer/lens-constants";
import { useGraphState } from "./useGraphState";
import { useGraphEffects } from "./useGraphEffects";
import { GraphContextMenu } from "./GraphContextMenu";
import { GraphToolbarRow } from "./GraphToolbarRow";
import { GraphZoomControls } from "./GraphZoomControls";
import { GraphLoading, GraphEmpty, GraphError } from "./GraphEmptyStates";
import type { GraphFilter } from "../../lib/tauri-commands";

const TreemapView = lazy(() =>
  import("./TreemapView").then((m) => ({ default: m.TreemapView })),
);
const CypherQueryFAB = lazy(() =>
  import("./CypherQueryFAB").then((m) => ({ default: m.CypherQueryFAB })),
);
const ProcessFlowModal = lazy(() =>
  import("./ProcessFlowModal").then((m) => ({ default: m.ProcessFlowModal })),
);
const GraphLegend = lazy(() =>
  import("./GraphLegend").then((m) => ({ default: m.GraphLegend })),
);
const CommunitiesPanel = lazy(() =>
  import("./CommunitiesPanel").then((m) => ({ default: m.CommunitiesPanel })),
);
const GraphMinimap = lazy(() =>
  import("./GraphMinimap").then((m) => ({ default: m.GraphMinimap })),
);
const GraphShortcutsOverlay = lazy(() =>
  import("./GraphShortcutsOverlay").then((m) => ({ default: m.GraphShortcutsOverlay })),
);

export function GraphExplorer() {
  const { t } = useI18n();

  // ── Store ────────────────────────────────────────────────────────
  const zoomLevel = useAppStore((s) => s.zoomLevel);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const setMode = useAppStore((s) => s.setMode);
  const setSearchOpen = useAppStore((s) => s.setSearchOpen);
  const setSearchQuery = useAppStore((s) => s.setSearchQuery);
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);
  const searchMatchIds = useAppStore((s) => s.searchMatchIds);
  const activeLens = useAppStore((s) => s.activeLens);
  const egoDepth = useAppStore((s) => s.egoDepth);
  const selectedFeatures = useAppStore((s) => s.selectedFeatures);
  const activeRepo = useAppStore((s) => s.activeRepo);

  // ── Local state ──────────────────────────────────────────────────
  const gs = useGraphState();

  // Destructure stable setters for exhaustive-deps compliance (React useState setters are stable)
  const {
    setContextMenu, setHoveredNode, setHoverPos, setHoverDegrees,
    setFocusNodeId, setImpactNodeIds, setImpactOverlay,
    setLayout, setHiddenEdgeTypes, highlightedNodeType, setHighlightedNodeType
  } = gs;

  // ── Derived ──────────────────────────────────────────────────────
  const highlightedNodeIdsFromType = useMemo(() => {
    if (!highlightedNodeType || !activeData) return new Set<string>();
    return new Set(activeData.nodes.filter(n => n.label === highlightedNodeType).map(n => n.id));
  }, [highlightedNodeType, activeData]);

  const combinedHighlightedNodeIds = useMemo(() => {
    const set = new Set(searchMatchIds);
    highlightedNodeIdsFromType.forEach(id => set.add(id));
    return set;
  }, [searchMatchIds, highlightedNodeIdsFromType]);

  const effectiveHiddenEdgeTypes = useMemo(() => {
    const lensEdgeTypes = LENS_EDGE_TYPES[activeLens];
    if (!lensEdgeTypes) return gs.hiddenEdgeTypes;
    const allKnown = [
      "CALLS", "IMPORTS", "DEPENDS_ON", "HAS_METHOD", "HAS_PROPERTY",
      "CONTAINED_IN", "DEFINED_IN", "EXTENDS", "IMPLEMENTS", "INHERITS",
      "CONTAINS", "REFERENCES",
    ];
    const next = new Set(gs.hiddenEdgeTypes);
    for (const type of allKnown) {
      if (!lensEdgeTypes.includes(type)) next.add(type); else next.delete(type);
    }
    return next;
  }, [gs.hiddenEdgeTypes, activeLens]);

  // ── Sigma ────────────────────────────────────────────────────────
  const {
    containerRef, graphRef, isLayoutRunning,
    setGraph, runLayout, focusNode, fitView,
    zoomIn, zoomOut, exportPNG, refresh, sigmaRef,
  } = useSigma({
    selectedNodeId,
    hoveredNodeId: gs.hoveredNode?.id,
    highlightedNodeIds: combinedHighlightedNodeIds,
    impactNodeIds: gs.impactOverlay ? gs.impactNodeIds : undefined,
    egoNodeIds: gs.egoNodeIds,
    egoDepthMap: gs.egoDepthMap,
    // graphRef/sigmaRef are refs returned by useSigma — circular dep with these callbacks, suppress safely
    onNodeClick: useCallback((nodeId: string | null) => {
      if (nodeId) {
        const g = graphRef.current;
        setSelectedNodeId(nodeId, g?.hasNode(nodeId) ? g.getNodeAttribute(nodeId, "label") : null);
      } else { setSelectedNodeId(null, null); setContextMenu(null); }
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [setSelectedNodeId, setContextMenu]),
    onNodeHover: useCallback((nodeId: string | null) => {
      if (!nodeId) { setHoveredNode(null); setHoverPos(null); return; }
      const g = graphRef.current; const sigma = sigmaRef.current;
      if (!g || !sigma || !g.hasNode(nodeId)) return;
      const a = g.getNodeAttributes(nodeId);
      const vp = sigma.graphToViewport({ x: a.x, y: a.y });
      setHoveredNode({ id: nodeId, name: a.label, label: a.nodeType, filePath: a.filePath, startLine: a.startLine, endLine: a.endLine, parameterCount: a.parameterCount, returnType: a.returnType, isTraced: a.isTraced, isDeadCandidate: a.isDeadCandidate, complexity: a.complexity });
      setHoverPos({ x: vp.x, y: vp.y });
      setHoverDegrees({ inDeg: g.inDegree(nodeId), outDeg: g.outDegree(nodeId) });
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [setHoveredNode, setHoverPos, setHoverDegrees]),
    onNodeRightClick: useCallback((nodeId: string, x: number, y: number) => {
      const g = graphRef.current; if (!g?.hasNode(nodeId)) return;
      const a = g.getNodeAttributes(nodeId);
      setContextMenu({ x, y, nodeId, name: a.label, filePath: a.filePath });
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [setContextMenu]),
    onNodeDoubleClick: useCallback((nodeId: string) => {
      setFocusNodeId(nodeId);
    }, [setFocusNodeId]),
  });

  // ── Data ─────────────────────────────────────────────────────────
  const { data, isLoading, error } = useGraphData({ zoomLevel, maxNodes: 200 } as GraphFilter, true);
  // Scope by `activeRepo` so focusing on a node in repo A and then switching
  // to repo B doesn't resurrect the cached subgraph when a same-named node
  // happens to exist (e.g. the workspace root node is always "Folder:").
  const { data: subgraphData } = useQuery({
    queryKey: ["subgraph", activeRepo, gs.focusNodeId],
    queryFn: () => commands.getSubgraph(gs.focusNodeId!, 2),
    enabled: !!gs.focusNodeId,
    staleTime: 30_000,
  });
  const activeData = gs.focusNodeId && subgraphData ? subgraphData : data;

  const { data: hotspotsData } = useQuery({
    queryKey: ["git-hotspots", activeRepo, gs.hotspotDays],
    queryFn: () => commands.getHotspots(gs.hotspotDays),
    // Risk lens reuses hotspot scores as one of its inputs.
    enabled: (activeLens === "hotspots" || activeLens === "risk") && !!activeRepo,
    staleTime: 60_000,
  });

  // ── Graph build effect ────────────────────────────────────────────
  const prevKeyRef = useRef("");
  useEffect(() => {
    if (!activeData || activeData.nodes.length === 0) return;
    const key = `${activeData.stats.nodeCount}-${activeData.stats.edgeCount}-${zoomLevel}-${gs.focusNodeId ?? ""}-${complexityThreshold}-${[...effectiveHiddenEdgeTypes].sort().join(",")}`;
    if (key === prevKeyRef.current) return;
    prevKeyRef.current = key;
    setGraph(buildGraphologyGraph(activeData.nodes, activeData.edges, effectiveHiddenEdgeTypes, complexityThreshold));
    runLayout();
  }, [activeData, zoomLevel, effectiveHiddenEdgeTypes, gs.focusNodeId, complexityThreshold, setGraph, runLayout]);

  // ── Hotspots Overlay Effect ───────────────────────────────────────
  useEffect(() => {
    const g = graphRef.current;
    if (!g || g.order === 0) return;

    if (activeLens === "hotspots" && hotspotsData && hotspotsData.length > 0) {
      const scoreMap = new Map<string, number>();
      let maxScore = 0;
      for (const h of hotspotsData) {
        scoreMap.set(h.path.replace(/\\/g, '/'), h.score);
        if (h.score > maxScore) maxScore = h.score;
      }

      g.forEachNode((node, attrs) => {
        if (!attrs.filePath) return;
        
        let nodeScore = 0;
        const normalizedFilePath = attrs.filePath.replace(/\\/g, '/');
        for (const [path, score] of scoreMap.entries()) {
          if (normalizedFilePath.endsWith(path) || path.endsWith(normalizedFilePath)) {
            nodeScore = score;
            break;
          }
        }

        if (nodeScore > 0) {
          const intensity = maxScore > 0 ? Math.min(1, nodeScore / maxScore) : 0;
          const r = Math.round(234 + intensity * (239 - 234));
          const gCol = Math.round(179 + intensity * (68 - 179));
          const b = Math.round(8 + intensity * (68 - 8));
          
          g.setNodeAttribute(node, "color", `rgb(${r}, ${gCol}, ${b})`);
          g.setNodeAttribute(node, "size", (attrs.originalSize || attrs.size) * (1 + intensity * 0.5));
        } else {
          g.setNodeAttribute(node, "color", "var(--bg-3)");
          g.setNodeAttribute(node, "size", attrs.originalSize || attrs.size);
        }
      });
    } else if (activeLens !== "risk") {
      // Risk lens has its own renderer below — don't reset its colors here.
      g.forEachNode((node, attrs) => {
        if (attrs.originalColor) g.setNodeAttribute(node, "color", attrs.originalColor);
        if (attrs.originalSize) g.setNodeAttribute(node, "size", attrs.originalSize);
      });
    }

    refresh();
  }, [activeLens, hotspotsData, graphRef, refresh, activeData]);

  // ── Risk Composite Lens ───────────────────────────────────────────
  // Combines four signals into a single risk score per node:
  //   - churn from git hotspots (file-level)
  //   - is_dead_candidate flag (node-level)
  //   - !is_traced flag (node-level, inverted)
  //   - llm_risk_score if present (node-level)
  // Renders nodes from green (safe) to red (dangerous).
  useEffect(() => {
    const g = graphRef.current;
    if (!g || g.order === 0) return;
    if (activeLens !== "risk") return;

    // Build file → hotspot score lookup (normalized 0..1).
    const hotspotScores = new Map<string, number>();
    let maxHotspot = 0;
    if (hotspotsData) {
      for (const h of hotspotsData) {
        const norm = h.path.replace(/\\/g, "/");
        hotspotScores.set(norm, h.score);
        if (h.score > maxHotspot) maxHotspot = h.score;
      }
    }

    g.forEachNode((node, attrs) => {
      // Aggregate signal weights — tuned so any single red flag shows up.
      let churn = 0;
      if (attrs.filePath && maxHotspot > 0) {
        const fp = attrs.filePath.replace(/\\/g, "/");
        for (const [path, score] of hotspotScores.entries()) {
          if (fp.endsWith(path) || path.endsWith(fp)) {
            churn = Math.min(1, score / maxHotspot);
            break;
          }
        }
      }
      const dead = attrs.isDeadCandidate ? 1 : 0;
      const untraced = attrs.isTraced === false ? 0.4 : 0; // softer signal
      const llmRisk = typeof attrs.llmRiskScore === "number" ? attrs.llmRiskScore : 0;

      // Composite ∈ [0, 1] — weighted sum capped at 1.
      const risk = Math.min(
        1,
        churn * 0.4 + dead * 0.5 + untraced * 0.2 + llmRisk * 0.4,
      );

      if (risk > 0) {
        // Green (158,206,106) → Yellow (224,175,104) → Red (247,118,142)
        const r = Math.round(158 + risk * (247 - 158));
        const gCol = Math.round(206 + risk * (118 - 206));
        const b = Math.round(106 + risk * (142 - 106));
        g.setNodeAttribute(node, "color", `rgb(${r}, ${gCol}, ${b})`);
        const baseSize = attrs.originalSize || attrs.size;
        g.setNodeAttribute(node, "size", baseSize * (1 + risk * 0.6));
      } else {
        g.setNodeAttribute(node, "color", "rgba(120, 130, 145, 0.4)");
        if (attrs.originalSize) g.setNodeAttribute(node, "size", attrs.originalSize);
      }
    });
    refresh();
  }, [activeLens, hotspotsData, graphRef, refresh, activeData]);

  // ── All other effects ─────────────────────────────────────────────
  // Pass the Set directly (NOT a fresh array spread) so the community-filter
  // effect inside useGraphEffects only fires when the selection actually
  // changes — `[...set]` would create a new array reference every render.
  useGraphEffects({ gs, selectedNodeId, searchMatchIds, selectedFeatures, egoDepth, graphRef, focusNode, refresh, fitView, zoomIn, zoomOut, exportPNG, setSearchOpen, setSelectedNodeId });

  // ── Impact overlay ────────────────────────────────────────────────
  const toggleImpactOverlay = useCallback(async () => {
    if (gs.impactOverlay) { setImpactNodeIds(new Map()); setImpactOverlay(false); refresh(); return; }
    if (!selectedNodeId) return;
    try {
      const result = await commands.getImpactAnalysis(selectedNodeId, "both", 3);
      const map = new Map<string, number>([[selectedNodeId, 0]]);
      const mark = (items: Array<{ node: { id: string }; depth: number }>) => {
        for (const item of items) if (!map.has(item.node.id)) map.set(item.node.id, item.depth);
      };
      if (result.upstream) mark(result.upstream);
      if (result.downstream) mark(result.downstream);
      setImpactNodeIds(map); setImpactOverlay(true); refresh();
    } catch (e) { console.error("Impact analysis failed:", e); toast.error(t("graph.impactFailed")); }
  }, [selectedNodeId, gs.impactOverlay, refresh, setImpactNodeIds, setImpactOverlay, t]);

  // ── Toolbar ───────────────────────────────────────────────────────
  const handleFit = useCallback(() => fitView(), [fitView]);
  const handleExport = useCallback(() => exportPNG(), [exportPNG]);
  const handleLayoutChange = useCallback((l: string) => { setLayout(l); runLayout(); }, [setLayout, runLayout]);
  const handleToggleEdgeType = useCallback((type: string) => {
    setHiddenEdgeTypes((prev) => { const next = new Set(prev); if (next.has(type)) next.delete(type); else next.add(type); return next; });
  }, [setHiddenEdgeTypes]);

  const toolbarProps = { 
    stats: data?.stats, 
    layout: gs.layout, 
    onLayoutChange: handleLayoutChange, 
    onFit: handleFit, 
    onExport: handleExport, 
    hiddenEdgeTypes: gs.hiddenEdgeTypes, 
    onToggleEdgeType: handleToggleEdgeType, 
    depthFilter: gs.depthFilter, 
    onDepthFilterChange: gs.setDepthFilter,
    complexityThreshold,
    onComplexityChange: setComplexityThreshold,
    hotspotDays: gs.hotspotDays,
    onHotspotDaysChange: gs.setHotspotDays,
    showDeadCode: gs.showDeadCode,
    onToggleDeadCode: () => gs.setShowDeadCode(!gs.showDeadCode)
  };

  // ── Dead Code Effect ──────────────────────────────────────────────
  useEffect(() => {
    const g = graphRef.current;
    if (!g || g.order === 0) return;

    if (gs.showDeadCode) {
      g.forEachNode((node, attrs) => {
        if (attrs.isDeadCandidate) {
          g.setNodeAttribute(node, "color", "#f7768e"); // Rose vif pour le code mort
          g.setNodeAttribute(node, "size", (attrs.originalSize || attrs.size) * 1.5);
        } else {
          g.setNodeAttribute(node, "color", "rgba(100, 116, 139, 0.2)"); // Très estompé pour le reste
        }
      });
    } else {
      // Revert if not showing dead code (and not in hotspots lens)
      if (activeLens !== "hotspots") {
        g.forEachNode((node, attrs) => {
          if (attrs.originalColor) g.setNodeAttribute(node, "color", attrs.originalColor);
          if (attrs.originalSize) g.setNodeAttribute(node, "size", attrs.originalSize);
        });
      }
    }
    refresh();
  }, [gs.showDeadCode, activeLens, graphRef, refresh]);

  // ── Early returns ─────────────────────────────────────────────────
  if (isLoading) return <GraphLoading {...toolbarProps} />;
  if (error) return <GraphError {...toolbarProps} error={error} />;
  if (data && data.nodes.length === 0) return <GraphEmpty {...toolbarProps} />;

  // ── Main render ──────────────────────────────────────────────────
  return (
    <div className="h-full flex flex-col overflow-hidden">
      <GraphToolbarRow {...toolbarProps} onFlows={() => gs.setFlowsOpen(true)} viewMode={gs.viewMode} onViewModeChange={gs.setViewMode} />

      {gs.viewMode === "treemap" ? (
        <div className="flex-1 relative">
          <Suspense fallback={<GraphLoading {...toolbarProps} />}>
            <TreemapView data={data} isLoading={isLoading} />
          </Suspense>
        </div>
      ) : (
        <div className="flex flex-1 min-h-0 relative overflow-hidden">
          <div className="flex-1 relative" style={{ backgroundColor: "var(--bg-0)" }}>
            <div ref={containerRef} className="absolute inset-0" style={{ cursor: "grab" }} role="application" aria-label="Interactive code dependency graph" tabIndex={0} />

            {isLayoutRunning && (
              <div className="absolute bottom-4 left-4 z-30 flex items-center gap-2 px-3 py-1.5 rounded-full border border-[var(--surface-border)] bg-[var(--bg-2)] shadow-lg animate-pulse">
                <div className="w-2 h-2 rounded-full bg-[var(--accent)]" />
                <div style={{ color: "var(--text-2)", fontSize: 11, fontWeight: 500 }}>{t("graph.computingLayout")}</div>
              </div>
            )}

            {gs.focusNodeId && (
              <button onClick={() => gs.setFocusNodeId(null)} className="absolute top-16 left-4 z-20 rounded-lg text-xs font-medium" style={{ padding: "6px 12px", background: "var(--accent)", color: "white", border: "none", cursor: "pointer" }}>
                &larr; {t("graph.backToFull")}
              </button>
            )}

            {data?.stats.truncated && !gs.focusNodeId && (
              <div className="absolute top-16 left-4 right-4 z-20 rounded-lg text-xs" style={{ padding: "8px 12px", background: "var(--bg-2)", border: "1px solid var(--surface-border)", color: "var(--text-2)", textAlign: "center" }}>
                {t("graph.showingTopNodes").replace("{0}", String(data.stats.nodeCount))}
              </div>
            )}

            <NodeHoverCard node={gs.hoveredNode} position={gs.hoverPos} inDegree={gs.hoverDegrees.inDeg} outDegree={gs.hoverDegrees.outDeg}
              onViewSource={() => { if (gs.hoveredNode) { setMode("explorer"); setSelectedNodeId("File:" + gs.hoveredNode.filePath, gs.hoveredNode.name); } }}
              onImpact={() => { if (gs.hoveredNode) { setSelectedNodeId(gs.hoveredNode.id, gs.hoveredNode.name); setMode("explorer"); } }}
            />

            <GraphContextMenu contextMenu={gs.contextMenu} onClose={() => gs.setContextMenu(null)}
              onGoToDefinition={(fp, name) => { setMode("explorer"); setSelectedNodeId("File:" + fp, name); }}
              onFindReferences={(name) => { setSearchQuery(name); setSearchOpen(true); }}
              onViewImpact={(nodeId, name) => { setSelectedNodeId(nodeId, name); setMode("explorer"); }}
              onExpandNeighbors={() => {}}
              onHideNode={(nodeId) => { const g = graphRef.current; if (g?.hasNode(nodeId)) { g.dropNode(nodeId); refresh(); } }}
              onCopyName={(name) => { navigator.clipboard.writeText(name).then(() => toast.success(t("graph.copiedToClipboard")), () => toast.error(t("graph.copyFailed"))); }}
              onCopyFilePath={(fp) => { navigator.clipboard.writeText(fp).then(() => toast.success(t("graph.copiedToClipboard")), () => toast.error(t("graph.copyFailed"))); }}
              onAiAction={(action, ctx) => {
                // Pre-canned per-action prompt + chat mode. The chat panel
                // picks the dispatched question up via pendingQuestion.
                const dispatch = useChatStore.getState().dispatchQuestion;
                switch (action) {
                  case "explain":
                    dispatch("qa", `Explain the symbol \`${ctx.name}\` in \`${ctx.filePath}\`. What is its role, who calls it, and what does it depend on?`, true);
                    break;
                  case "feature_dev":
                    dispatch("feature_dev", `I want to extend or refactor \`${ctx.name}\` (in \`${ctx.filePath}\`). Design the changes.`, true);
                    break;
                  case "code_review":
                    dispatch("code_review", `review: ${ctx.name}`, true);
                    break;
                  case "dead_check":
                    dispatch("qa", `Is \`${ctx.name}\` (in \`${ctx.filePath}\`) actually used? Find all callers and assess whether this is dead code.`, true);
                    break;
                }
                setMode("chat");
              }}
            />

            {(gs.minimapVisible || gs.minimapOpacity !== 0.3) && (
              <Suspense fallback={null}>
                <GraphMinimap visible={gs.minimapVisible} opacity={gs.minimapOpacity} onOpacityChange={gs.setMinimapOpacity} onClose={() => gs.setMinimapVisible(false)} sigmaRef={sigmaRef} graphRef={graphRef} />
              </Suspense>
            )}

            {selectedNodeId && (
              <button onClick={toggleImpactOverlay} className="absolute z-20 flex items-center gap-1.5 rounded-lg transition-all" style={{ bottom: 70, left: 16, padding: "8px 14px", background: gs.impactOverlay ? "var(--rose)" : "var(--bg-2)", color: gs.impactOverlay ? "white" : "var(--text-2)", border: `1px solid ${gs.impactOverlay ? "var(--rose)" : "var(--surface-border)"}`, backdropFilter: "blur(12px)", fontSize: 11, fontWeight: 600, cursor: "pointer", boxShadow: gs.impactOverlay ? "0 0 20px color-mix(in srgb, var(--rose) 30%, transparent)" : "var(--shadow-sm)" }}>
                <Zap size={13} />{gs.impactOverlay ? t("graph.clearImpact") : t("graph.impactOverlay")}
              </button>
            )}

            <Suspense fallback={null}>
              <CypherQueryFAB />
            </Suspense>
            {gs.shortcutsOpen && (
              <Suspense fallback={null}>
                <GraphShortcutsOverlay visible={gs.shortcutsOpen} />
              </Suspense>
            )}
            <GraphZoomControls onZoomIn={zoomIn} onZoomOut={zoomOut} onFitView={fitView} legendExpanded={gs.legendExpanded} />
            <Suspense fallback={null}>
              <GraphLegend 
                nodes={data?.nodes ?? []} 
                expanded={gs.legendExpanded} 
                onExpand={() => gs.setLegendExpanded(true)} 
                onCollapse={() => gs.setLegendExpanded(false)} 
                highlightedNodeType={highlightedNodeType}
                onTypeClick={setHighlightedNodeType}
              />
            </Suspense>
            <Suspense fallback={null}>
              <CommunitiesPanel />
            </Suspense>
          </div>
        </div>
      )}

      {gs.flowsOpen && (
        <Suspense fallback={null}>
          <ProcessFlowModal open={gs.flowsOpen} onClose={() => gs.setFlowsOpen(false)} />
        </Suspense>
      )}
    </div>
  );
}
