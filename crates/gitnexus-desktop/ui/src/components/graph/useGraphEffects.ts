import { useEffect } from "react";
import {
  filterGraphByDepth,
  filterGraphByCommunities,
} from "../../lib/graph-adapter";
import type { GraphState } from "./useGraphState";
import type Graph from "graphology";
import type { SigmaNodeAttributes, SigmaEdgeAttributes } from "../../lib/graph-adapter";

interface GraphEffectsOptions {
  gs: GraphState;
  selectedNodeId: string | null | undefined;
  searchMatchIds: string[];
  selectedFeatures: string[];
  egoDepth: number;
  graphRef: React.RefObject<Graph<SigmaNodeAttributes, SigmaEdgeAttributes>>;
  focusNode: (id: string) => void;
  refresh: () => void;
  fitView: () => void;
  zoomIn: () => void;
  zoomOut: () => void;
  exportPNG: () => void;
  setSearchOpen: (v: boolean) => void;
  setSelectedNodeId: (id: string | null, name: string | null) => void;
}

export function useGraphEffects({
  gs,
  selectedNodeId,
  searchMatchIds,
  selectedFeatures,
  egoDepth,
  graphRef,
  focusNode,
  refresh,
  fitView,
  zoomIn,
  zoomOut,
  exportPNG,
  setSearchOpen,
  setSelectedNodeId,
}: GraphEffectsOptions) {
  // Ego-network BFS
  useEffect(() => {
    const graph = graphRef.current;
    if (!selectedNodeId || !graph || !graph.hasNode(selectedNodeId)) {
      gs.setEgoNodeIds(undefined);
      gs.setEgoDepthMap(undefined);
      return;
    }
    const nodeIds = new Set<string>();
    const depthMap = new Map<string, number>();
    const queue: [string, number][] = [[selectedNodeId, 0]];
    nodeIds.add(selectedNodeId);
    depthMap.set(selectedNodeId, 0);
    while (queue.length > 0) {
      const [cur, d] = queue.shift()!;
      if (d >= egoDepth) continue;
      graph.neighbors(cur).forEach((nb) => {
        if (!nodeIds.has(nb)) {
          nodeIds.add(nb);
          depthMap.set(nb, d + 1);
          queue.push([nb, d + 1]);
        }
      });
    }
    gs.setEgoNodeIds(nodeIds);
    gs.setEgoDepthMap(depthMap);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedNodeId, egoDepth]);

  // Depth filter
  useEffect(() => {
    const g = graphRef.current;
    if (!g || g.order === 0) return;
    filterGraphByDepth(g, selectedNodeId ?? null, gs.depthFilter);
    refresh();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [gs.depthFilter, selectedNodeId, refresh]);

  // Camera focus on selection
  useEffect(() => {
    if (selectedNodeId) focusNode(selectedNodeId);
    refresh();
  }, [selectedNodeId, focusNode, refresh]);

  // Refresh on search highlights
  useEffect(() => {
    refresh();
  }, [searchMatchIds, refresh]);

  // Community filter
  useEffect(() => {
    const g = graphRef.current;
    if (!g) return;
    filterGraphByCommunities(g, selectedFeatures instanceof Set ? selectedFeatures : new Set(selectedFeatures));
    refresh();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedFeatures, refresh]);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "g") { e.preventDefault(); setSearchOpen(true); }
      if ((e.ctrlKey || e.metaKey) && e.key === "e") { e.preventDefault(); exportPNG(); }
      if ((e.ctrlKey || e.metaKey) && (e.key === "=" || e.key === "+")) { e.preventDefault(); zoomIn(); }
      if ((e.ctrlKey || e.metaKey) && e.key === "-") { e.preventDefault(); zoomOut(); }
      if ((e.ctrlKey || e.metaKey) && e.key === "0") { e.preventDefault(); fitView(); }
      if (e.key === "Escape" && !e.ctrlKey && !e.metaKey) {
        setSelectedNodeId(null, null);
        gs.setContextMenu(null);
        gs.setShortcutsOpen(false);
      }
      if (e.key === "?" && !e.ctrlKey && !e.metaKey) gs.setShortcutsOpen((p) => !p);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [exportPNG, zoomIn, zoomOut, fitView, setSearchOpen, setSelectedNodeId]);

  // Custom window events
  useEffect(() => {
    const onFit = () => fitView();
    const onCycle = () => {
      const ls = ["forceatlas2", "grid", "circle", "random"];
      gs.setLayout(ls[(ls.indexOf(gs.layout) + 1) % ls.length]);
    };
    window.addEventListener("gitnexus:fit-graph", onFit);
    window.addEventListener("gitnexus:cycle-layout", onCycle);
    return () => {
      window.removeEventListener("gitnexus:fit-graph", onFit);
      window.removeEventListener("gitnexus:cycle-layout", onCycle);
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [gs.layout, fitView]);
}
