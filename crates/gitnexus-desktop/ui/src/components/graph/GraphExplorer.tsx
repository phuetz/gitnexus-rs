import { useCallback, useMemo, useRef, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { Zap } from "lucide-react";
import { useGraphData } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";
import { useSigma } from "../../hooks/use-sigma";
import { commands } from "../../lib/tauri-commands";
import { buildGraphologyGraph } from "../../lib/graph-adapter";
import { FeatureNavigator } from "./FeatureNavigator";
import { NodeHoverCard } from "./NodeHoverCard";
import { TreemapView } from "./TreemapView";
import { useI18n } from "../../hooks/use-i18n";
import { CypherQueryFAB } from "./CypherQueryFAB";
import { ProcessFlowModal } from "./ProcessFlowModal";
import { LENS_EDGE_TYPES } from "../explorer/LensSelector";
import { useGraphState } from "./useGraphState";
import { useGraphEffects } from "./useGraphEffects";
import { GraphContextMenu } from "./GraphContextMenu";
import { GraphLegend } from "./GraphLegend";
import { CommunitiesPanel } from "./CommunitiesPanel";
import { GraphMinimap } from "./GraphMinimap";
import { GraphToolbarRow } from "./GraphToolbarRow";
import { GraphShortcutsOverlay } from "./GraphShortcutsOverlay";
import { GraphZoomControls } from "./GraphZoomControls";
import { GraphLoading, GraphEmpty, GraphError } from "./GraphEmptyStates";
import type { GraphFilter } from "../../lib/tauri-commands";

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
  const toggleFeature = useAppStore((s) => s.toggleFeature);
  const resetFeatures = useAppStore((s) => s.resetFeatures);

  // ── Local state ──────────────────────────────────────────────────
  const gs = useGraphState();

  // ── Derived ──────────────────────────────────────────────────────
  const highlightedNodeIds = useMemo(() => new Set(searchMatchIds), [searchMatchIds]);

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
    highlightedNodeIds,
    impactNodeIds: gs.impactOverlay ? gs.impactNodeIds : undefined,
    egoNodeIds: gs.egoNodeIds,
    egoDepthMap: gs.egoDepthMap,
    onNodeClick: useCallback((nodeId: string | null) => {
      if (nodeId) {
        const g = graphRef.current;
        setSelectedNodeId(nodeId, g?.hasNode(nodeId) ? g.getNodeAttribute(nodeId, "label") : null);
      } else { setSelectedNodeId(null, null); gs.setContextMenu(null); }
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [setSelectedNodeId]),
    onNodeHover: useCallback((nodeId: string | null) => {
      if (!nodeId) { gs.setHoveredNode(null); gs.setHoverPos(null); return; }
      const g = graphRef.current; const sigma = sigmaRef.current;
      if (!g || !sigma || !g.hasNode(nodeId)) return;
      const a = g.getNodeAttributes(nodeId);
      const vp = sigma.graphToViewport({ x: a.x, y: a.y });
      gs.setHoveredNode({ id: nodeId, name: a.label, label: a.nodeType, filePath: a.filePath, startLine: a.startLine, endLine: a.endLine, parameterCount: a.parameterCount, returnType: a.returnType, isTraced: a.isTraced, isDeadCandidate: a.isDeadCandidate, complexity: a.complexity });
      gs.setHoverPos({ x: vp.x, y: vp.y });
      gs.setHoverDegrees({ inDeg: g.inDegree(nodeId), outDeg: g.outDegree(nodeId) });
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []),
    onNodeRightClick: useCallback((nodeId: string, x: number, y: number) => {
      const g = graphRef.current; if (!g?.hasNode(nodeId)) return;
      const a = g.getNodeAttributes(nodeId);
      gs.setContextMenu({ x, y, nodeId, name: a.label, filePath: a.filePath });
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []),
    onNodeDoubleClick: useCallback((nodeId: string) => {
      gs.setFocusNodeId(nodeId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []),
  });

  // ── Data ─────────────────────────────────────────────────────────
  const { data, isLoading, error } = useGraphData({ zoomLevel, maxNodes: 200 } as GraphFilter, true);
  const { data: subgraphData } = useQuery({
    queryKey: ["subgraph", gs.focusNodeId],
    queryFn: () => commands.getSubgraph(gs.focusNodeId!, 2),
    enabled: !!gs.focusNodeId,
    staleTime: 30_000,
  });
  const activeData = gs.focusNodeId && subgraphData ? subgraphData : data;

  // ── Graph build effect ────────────────────────────────────────────
  const prevKeyRef = useRef("");
  useEffect(() => {
    if (!activeData || activeData.nodes.length === 0) return;
    const key = `${activeData.stats.nodeCount}-${activeData.stats.edgeCount}-${zoomLevel}-${gs.focusNodeId ?? ""}-${[...effectiveHiddenEdgeTypes].sort().join(",")}`;
    if (key === prevKeyRef.current) return;
    prevKeyRef.current = key;
    setGraph(buildGraphologyGraph(activeData.nodes, activeData.edges, effectiveHiddenEdgeTypes));
    runLayout();
  }, [activeData, zoomLevel, effectiveHiddenEdgeTypes, gs.focusNodeId, setGraph, runLayout]);

  // ── All other effects ─────────────────────────────────────────────
  useGraphEffects({ gs, selectedNodeId, searchMatchIds, selectedFeatures: [...selectedFeatures], egoDepth, graphRef, focusNode, refresh, fitView, zoomIn, zoomOut, exportPNG, setSearchOpen, setSelectedNodeId });

  // ── Impact overlay ────────────────────────────────────────────────
  const toggleImpactOverlay = useCallback(async () => {
    if (gs.impactOverlay) { gs.setImpactNodeIds(new Map()); gs.setImpactOverlay(false); refresh(); return; }
    if (!selectedNodeId) return;
    try {
      const result = await commands.getImpactAnalysis(selectedNodeId, "both", 3);
      const map = new Map<string, number>([[selectedNodeId, 0]]);
      const mark = (items: Array<{ node: { id: string }; depth: number }>) => {
        for (const item of items) if (!map.has(item.node.id)) map.set(item.node.id, item.depth);
      };
      if (result.upstream) mark(result.upstream);
      if (result.downstream) mark(result.downstream);
      gs.setImpactNodeIds(map); gs.setImpactOverlay(true); refresh();
    } catch { /* silently fail */ }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedNodeId, gs.impactOverlay, refresh]);

  // ── Toolbar ───────────────────────────────────────────────────────
  const handleFit = useCallback(() => fitView(), [fitView]);
  const handleExport = useCallback(() => exportPNG(), [exportPNG]);
  const handleLayoutChange = useCallback((l: string) => { gs.setLayout(l); runLayout(); }, [runLayout, gs]);
  const handleToggleEdgeType = useCallback((type: string) => {
    gs.setHiddenEdgeTypes((prev) => { const next = new Set(prev); if (next.has(type)) next.delete(type); else next.add(type); return next; });
  }, [gs]);

  const toolbarProps = { stats: data?.stats, layout: gs.layout, onLayoutChange: handleLayoutChange, onFit: handleFit, onExport: handleExport, hiddenEdgeTypes: gs.hiddenEdgeTypes, onToggleEdgeType: handleToggleEdgeType, depthFilter: gs.depthFilter, onDepthFilterChange: gs.setDepthFilter };

  // ── Early returns ─────────────────────────────────────────────────
  if (isLoading) return <GraphLoading {...toolbarProps} />;
  if (data && data.nodes.length === 0) return <GraphEmpty {...toolbarProps} />;
  if (error) return <GraphError {...toolbarProps} error={error} />;

  // ── Main render ──────────────────────────────────────────────────
  return (
    <div className="h-full flex flex-col">
      <GraphToolbarRow {...toolbarProps} onFlows={() => gs.setFlowsOpen(true)} viewMode={gs.viewMode} onViewModeChange={gs.setViewMode} />

      {gs.viewMode === "treemap" ? (
        <div className="flex-1 relative"><TreemapView data={data} isLoading={isLoading} /></div>
      ) : (
        <div className="flex flex-1 min-h-0">
          <FeatureNavigator selectedFeatures={selectedFeatures} onToggleFeature={toggleFeature} onReset={resetFeatures} />
          <div className="flex-1 relative" style={{ backgroundColor: "#06060a" }}>
            <div ref={containerRef} className="absolute inset-0" style={{ cursor: "grab" }} role="img" aria-label="Knowledge graph visualization" />

            {isLayoutRunning && (
              <div className="absolute inset-0 z-30 flex items-center justify-center" style={{ backgroundColor: "rgba(9,11,16,0.5)", backdropFilter: "blur(2px)" }}>
                <div style={{ color: "var(--text-2)", fontSize: 13 }}>{t("graph.computingLayout")}</div>
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
              onCopyName={(name) => navigator.clipboard.writeText(name)}
              onCopyFilePath={(fp) => navigator.clipboard.writeText(fp)}
            />

            <GraphMinimap visible={gs.minimapVisible} opacity={gs.minimapOpacity} onOpacityChange={gs.setMinimapOpacity} onClose={() => gs.setMinimapVisible(false)} sigmaRef={sigmaRef} graphRef={graphRef} />

            {selectedNodeId && (
              <button onClick={toggleImpactOverlay} className="absolute z-20 flex items-center gap-1.5 rounded-lg transition-all" style={{ bottom: 70, left: 16, padding: "8px 14px", background: gs.impactOverlay ? "#f7768e" : "var(--bg-2)", color: gs.impactOverlay ? "white" : "var(--text-2)", border: `1px solid ${gs.impactOverlay ? "#f7768e" : "var(--surface-border)"}`, backdropFilter: "blur(12px)", fontSize: 11, fontWeight: 600, cursor: "pointer", boxShadow: gs.impactOverlay ? "0 0 20px rgba(247,118,142,0.3)" : "var(--shadow-sm)" }}>
                <Zap size={13} />{gs.impactOverlay ? t("graph.clearImpact") : t("graph.impactOverlay")}
              </button>
            )}

            <CypherQueryFAB />
            <GraphShortcutsOverlay visible={gs.shortcutsOpen} />
            <GraphZoomControls onZoomIn={zoomIn} onZoomOut={zoomOut} onFitView={fitView} legendExpanded={gs.legendExpanded} />
            <GraphLegend nodes={data?.nodes ?? []} expanded={gs.legendExpanded} onExpand={() => gs.setLegendExpanded(true)} onCollapse={() => gs.setLegendExpanded(false)} />
          </div>
          <CommunitiesPanel />
        </div>
      )}

      <ProcessFlowModal open={gs.flowsOpen} onClose={() => gs.setFlowsOpen(false)} />
    </div>
  );
}
