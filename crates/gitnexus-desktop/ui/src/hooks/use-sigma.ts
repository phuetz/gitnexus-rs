/**
 * useSigma — wraps all Sigma.js/graphology logic for the graph explorer.
 *
 * Handles: init, graph data, ForceAtlas2 layout, camera ops, node/edge
 * reducers (selection, impact, search highlighting), and PNG export.
 */
import { useRef, useEffect, useCallback, useState } from "react";
import Sigma from "sigma";
import Graph from "graphology";
import EdgeCurveProgram from "@sigma/edge-curve";
import FA2Layout from "graphology-layout-forceatlas2/worker";
import forceAtlas2 from "graphology-layout-forceatlas2";
import noverlap from "graphology-layout-noverlap";
import type {
  SigmaNodeAttributes,
  SigmaEdgeAttributes,
} from "../lib/graph-adapter";
import { dimColor } from "../lib/graph-adapter";

// ─── Public types ──────────────────────────────────────────────────

export interface UseSigmaOptions {
  onNodeClick?: (nodeId: string | null) => void;
  onNodeHover?: (nodeId: string | null) => void;
  onNodeRightClick?: (nodeId: string, x: number, y: number) => void;
  onNodeDoubleClick?: (nodeId: string) => void;
  selectedNodeId?: string | null;
  highlightedNodeIds?: Set<string>;
  impactNodeIds?: Map<string, number>; // nodeId -> depth
  egoNodeIds?: Set<string>;
  egoDepthMap?: Map<string, number>;
}

// ─── Hook ──────────────────────────────────────────────────────────

export function useSigma(options: UseSigmaOptions) {
  const containerRef = useRef<HTMLDivElement>(null);
  const sigmaRef = useRef<Sigma | null>(null);
  const graphRef = useRef<Graph<SigmaNodeAttributes, SigmaEdgeAttributes>>(
    new Graph<SigmaNodeAttributes, SigmaEdgeAttributes>(),
  );
  const layoutRef = useRef<FA2Layout | null>(null);
  const layoutTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [isLayoutRunning, setIsLayoutRunning] = useState(false);

  // Keep options in refs so the reducers (which are set once) can read
  // the latest values without requiring Sigma re-init.
  const selectedRef = useRef(options.selectedNodeId);
  const highlightedRef = useRef(options.highlightedNodeIds);
  const impactRef = useRef(options.impactNodeIds);
  const egoNodeIdsRef = useRef<Set<string> | undefined>(undefined);
  const egoDepthMapRef = useRef<Map<string, number> | undefined>(undefined);

  useEffect(() => {
    selectedRef.current = options.selectedNodeId;
  }, [options.selectedNodeId]);
  useEffect(() => {
    highlightedRef.current = options.highlightedNodeIds;
  }, [options.highlightedNodeIds]);
  useEffect(() => {
    impactRef.current = options.impactNodeIds;
  }, [options.impactNodeIds]);
  useEffect(() => {
    egoNodeIdsRef.current = options.egoNodeIds;
  }, [options.egoNodeIds]);
  useEffect(() => {
    egoDepthMapRef.current = options.egoDepthMap;
  }, [options.egoDepthMap]);

  // Store callbacks in refs so we can read latest without re-init
  const onNodeClickRef = useRef(options.onNodeClick);
  const onNodeHoverRef = useRef(options.onNodeHover);
  const onNodeRightClickRef = useRef(options.onNodeRightClick);
  const onNodeDoubleClickRef = useRef(options.onNodeDoubleClick);
  useEffect(() => { onNodeClickRef.current = options.onNodeClick; }, [options.onNodeClick]);
  useEffect(() => { onNodeHoverRef.current = options.onNodeHover; }, [options.onNodeHover]);
  useEffect(() => { onNodeRightClickRef.current = options.onNodeRightClick; }, [options.onNodeRightClick]);
  useEffect(() => { onNodeDoubleClickRef.current = options.onNodeDoubleClick; }, [options.onNodeDoubleClick]);

  // ── Sigma init (once) ────────────────────────────────────────────

  useEffect(() => {
    if (!containerRef.current) return;

    const graph = graphRef.current;

    // Read theme-aware colors from CSS custom properties
    const cssVars = getComputedStyle(document.documentElement);
    const labelColor = cssVars.getPropertyValue("--text-1").trim() || "#e4e4ed";
    const defaultNodeColor = cssVars.getPropertyValue("--text-4").trim() || "#64748b";
    const defaultEdgeColor = cssVars.getPropertyValue("--surface-border").trim() || "#2a2a3a";

    const sigma = new Sigma(graph, containerRef.current, {
      renderLabels: true,
      labelFont: "JetBrains Mono, Fira Code, monospace",
      labelSize: 11,
      labelWeight: "500",
      labelColor: { color: labelColor },
      labelRenderedSizeThreshold: 6,
      labelDensity: 0.15,
      labelGridCellSize: 60,
      defaultNodeColor,
      defaultEdgeColor,
      defaultEdgeType: "curved",
      hideEdgesOnMove: true,
      zIndex: true,
      minCameraRatio: 0.01,
      maxCameraRatio: 20,

      edgeProgramClasses: {
        curved: EdgeCurveProgram,
      },

      // ── Custom hover: dark pill with colored border ──────────
      defaultDrawNodeHover: (context, data) => {
        const label = data.label;
        if (!label) return;
        const fontSize = 11;
        const font = `500 ${fontSize}px JetBrains Mono, monospace`;
        context.font = font;
        const textWidth = context.measureText(label).width;
        const nodeSize = data.size;
        const x = data.x;
        const y = data.y - nodeSize - 12;
        const padX = 8;
        const padY = 5;
        const w = textWidth + padX * 2;
        const h = fontSize + padY * 2;

        // Dark pill background
        context.fillStyle = "#0d1520";
        context.beginPath();
        context.roundRect(x - w / 2, y - h / 2, w, h, 4);
        context.fill();

        // Colored border
        context.strokeStyle = data.color || "#06b6d4";
        context.lineWidth = 1.5;
        context.stroke();

        // Light text
        context.fillStyle = "#e4e4ed";
        context.textAlign = "center";
        context.textBaseline = "middle";
        context.fillText(label, x, y);

        // Glow ring around the node
        context.beginPath();
        context.arc(data.x, data.y, nodeSize + 3, 0, Math.PI * 2);
        context.strokeStyle = data.color || "#06b6d4";
        context.lineWidth = 1.5;
        context.globalAlpha = 0.4;
        context.stroke();
        context.globalAlpha = 1;
      },

      // ── Node reducer (dynamic styling) ───────────────────────
      nodeReducer: (node, data) => {
        const res = { ...data };
        const selected = selectedRef.current;
        const highlighted = highlightedRef.current;
        const impact = impactRef.current;

        // Ego-network overlay
        const egoNodes = egoNodeIdsRef.current;
        const egoDepths = egoDepthMapRef.current;
        if (egoNodes && egoNodes.size > 0) {
          if (egoNodes.has(node)) {
            const d = egoDepths?.get(node) ?? 0;
            if (d <= 1) {
              // Full opacity, normal size
              res.zIndex = 10 - d;
            } else if (d === 2) {
              res.color = dimColor(data.color || "#64748b", 0.8);
              res.zIndex = 3;
            } else {
              res.color = dimColor(data.color || "#64748b", 0.6);
              res.zIndex = 2;
            }
          } else {
            // Context node — heavily dimmed
            res.color = dimColor(data.color || "#64748b", 0.10);
            res.size = (data.size || 6) * 0.4;
            res.label = "";
            res.zIndex = 0;
          }
          return res;
        }

        // Impact overlay
        if (impact && impact.size > 0) {
          const depth = impact.get(node);
          if (depth !== undefined) {
            if (depth === 0) {
              res.color = "#ef4444";
              res.size = (data.size || 6) * 1.8;
              res.zIndex = 10;
            } else if (depth === 1) {
              res.color = "#ff9e64";
              res.size = (data.size || 6) * 1.4;
              res.zIndex = 5;
            } else if (depth === 2) {
              res.color = "#e0af68";
              res.size = (data.size || 6) * 1.2;
              res.zIndex = 3;
            } else {
              res.color = "#9ece6a";
              res.size = (data.size || 6) * 1.1;
              res.zIndex = 2;
            }
          } else {
            res.color = dimColor(data.color || "#64748b", 0.15);
            res.size = (data.size || 6) * 0.5;
          }
          return res;
        }

        // Search highlighting
        if (highlighted && highlighted.size > 0 && !selected) {
          if (highlighted.has(node)) {
            res.color = "#06b6d4";
            res.size = (data.size || 6) * 1.5;
            res.zIndex = 5;
          } else {
            res.color = dimColor(data.color || "#64748b", 0.2);
            res.size = (data.size || 6) * 0.6;
          }
          return res;
        }

        // Selection highlighting
        if (selected) {
          const g = graphRef.current;
          if (node === selected) {
            res.size = (data.size || 6) * 1.8;
            res.zIndex = 10;
          } else if (g && g.hasNode(selected) && g.areNeighbors(node, selected)) {
            res.size = (data.size || 6) * 1.3;
            res.zIndex = 5;
          } else {
            res.color = dimColor(data.color || "#64748b", 0.15);
            res.size = (data.size || 6) * 0.5;
          }
        }

        return res;
      },

      // ── Edge reducer ─────────────────────────────────────────
      edgeReducer: (edge, data) => {
        const res = { ...data };
        const selected = selectedRef.current;
        const g = graphRef.current;

        // Ego-network edge overlay
        const egoNodes2 = egoNodeIdsRef.current;
        if (egoNodes2 && egoNodes2.size > 0 && g) {
          const [source, target] = g.extremities(edge);
          const srcIn = egoNodes2.has(source);
          const tgtIn = egoNodes2.has(target);
          if (srcIn && tgtIn) {
            // Both in ego — full
            res.size = Math.max(1, (data.size || 0.5) * 2);
            res.zIndex = 5;
          } else if (srcIn || tgtIn) {
            // One in ego — partial
            res.color = dimColor(data.color || "#2a2a3a", 0.3);
            res.size = (data.size || 0.5) * 0.8;
          } else {
            // Neither in ego — near invisible
            res.color = dimColor(data.color || "#2a2a3a", 0.05);
            res.size = 0.1;
          }
          return res;
        }

        if (selected && g) {
          const [source, target] = g.extremities(edge);
          if (source === selected || target === selected) {
            res.size = Math.max(1.5, (data.size || 0.5) * 3);
            res.zIndex = 5;
          } else if (
            g.hasNode(selected) &&
            (g.areNeighbors(source, selected) || g.areNeighbors(target, selected))
          ) {
            res.size = (data.size || 0.5) * 1.5;
            res.color = dimColor(data.color || "#2a2a3a", 0.4);
          } else {
            res.color = dimColor(data.color || "#2a2a3a", 0.08);
            res.size = 0.2;
          }
        }

        return res;
      },
    });

    sigmaRef.current = sigma;

    // ── Theme change observer ────────────────────────────────────
    const themeObserver = new MutationObserver(() => {
      const s = getComputedStyle(document.documentElement);
      sigma.setSetting("labelColor", { color: s.getPropertyValue("--text-1").trim() || "#e4e4ed" });
      sigma.setSetting("defaultNodeColor", s.getPropertyValue("--text-4").trim() || "#64748b");
      sigma.setSetting("defaultEdgeColor", s.getPropertyValue("--surface-border").trim() || "#2a2a3a");
      sigma.refresh();
    });
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });

    // ── Events ─────────────────────────────────────────────────
    let clickDebounceTimer: ReturnType<typeof setTimeout> | null = null;
    sigma.on("clickNode", ({ node }) => {
      if (clickDebounceTimer) clearTimeout(clickDebounceTimer);
      clickDebounceTimer = setTimeout(() => {
        onNodeClickRef.current?.(node);
      }, 150);
    });
    sigma.on("clickStage", () => {
      onNodeClickRef.current?.(null);
    });
    sigma.on("doubleClickNode", ({ node }) => {
      onNodeDoubleClickRef.current?.(node);
    });
    sigma.on("enterNode", ({ node }) => {
      onNodeHoverRef.current?.(node);
      if (containerRef.current) containerRef.current.style.cursor = "pointer";
    });
    sigma.on("leaveNode", () => {
      onNodeHoverRef.current?.(null);
      if (containerRef.current) containerRef.current.style.cursor = "grab";
    });
    sigma.on("rightClickNode", ({ node, event }) => {
      event.original.preventDefault();
      onNodeRightClickRef.current?.(node, event.x, event.y);
    });

    return () => {
      themeObserver.disconnect();
      if (clickDebounceTimer) {
        clearTimeout(clickDebounceTimer);
      }
      if (layoutRef.current) {
        layoutRef.current.stop();
        layoutRef.current = null;
      }
      if (layoutTimerRef.current) {
        clearTimeout(layoutTimerRef.current);
        layoutTimerRef.current = null;
      }
      sigma.kill();
      sigmaRef.current = null;
    };
  }, []); // ONE TIME init

  // ── Set graph data ───────────────────────────────────────────────

  const setGraph = useCallback(
    (newGraph: Graph<SigmaNodeAttributes, SigmaEdgeAttributes>) => {
      const sigma = sigmaRef.current;
      if (!sigma) return;

      graphRef.current = newGraph;
      sigma.setGraph(newGraph);
      sigma.refresh();
    },
    [],
  );

  // ── ForceAtlas2 layout ───────────────────────────────────────────

  const runLayout = useCallback(() => {
    const graph = graphRef.current;
    if (!graph || graph.order === 0) return;

    // Stop previous layout & timer
    if (layoutRef.current) {
      layoutRef.current.stop();
      layoutRef.current = null;
    }
    if (layoutTimerRef.current) {
      clearTimeout(layoutTimerRef.current);
      layoutTimerRef.current = null;
    }

    const nodeCount = graph.order;
    const settings = {
      ...forceAtlas2.inferSettings(graph),
      gravity: nodeCount < 500 ? 0.8 : nodeCount < 2000 ? 0.5 : 0.3,
      scalingRatio: nodeCount < 500 ? 15 : nodeCount < 2000 ? 30 : 60,
      slowDown: nodeCount < 500 ? 1 : 3,
      barnesHutOptimize: nodeCount > 200,
    };

    const layout = new FA2Layout(graph, { settings });
    layoutRef.current = layout;
    layout.start();
    setIsLayoutRunning(true);

    // Auto-stop — scale with graph size (small graphs settle fast)
    const duration =
      nodeCount > 2000 ? 15000 : nodeCount > 500 ? 8000 : nodeCount > 50 ? 4000 : 2000;
    layoutTimerRef.current = setTimeout(() => {
      if (layoutRef.current) {
        layoutRef.current.stop();
        layoutRef.current = null;
        noverlap.assign(graph, {
          maxIterations: 20,
          settings: { ratio: 1.1, margin: 5 },
        });
        sigmaRef.current?.refresh();
        // Auto-fit view after layout — wait one frame so Sigma can
        // recompute the graph extent before the camera resets
        requestAnimationFrame(() => {
          const camera = sigmaRef.current?.getCamera();
          if (camera) camera.animatedReset({ duration: 300 });
        });
        setIsLayoutRunning(false);
      }
    }, duration);
  }, []);

  // ── Stop layout ──────────────────────────────────────────────────

  const stopLayout = useCallback(() => {
    if (layoutRef.current) {
      layoutRef.current.stop();
      layoutRef.current = null;
    }
    if (layoutTimerRef.current) {
      clearTimeout(layoutTimerRef.current);
      layoutTimerRef.current = null;
    }
    setIsLayoutRunning(false);
  }, []);

  // ── Camera animation: focus on a node ────────────────────────────

  const focusNode = useCallback((nodeId: string) => {
    const sigma = sigmaRef.current;
    const graph = graphRef.current;
    if (!sigma || !graph || !graph.hasNode(nodeId)) return;
    const attrs = graph.getNodeAttributes(nodeId);
    sigma
      .getCamera()
      .animate({ x: attrs.x, y: attrs.y, ratio: 0.15 }, { duration: 200 });
  }, []);

  // ── Fit view ─────────────────────────────────────────────────────

  const fitView = useCallback(() => {
    const camera = sigmaRef.current?.getCamera();
    if (camera) camera.animatedReset({ duration: 300 });
  }, []);

  // ── Zoom ─────────────────────────────────────────────────────────

  const zoomIn = useCallback(() => {
    const cam = sigmaRef.current?.getCamera();
    if (cam)
      cam.animate({ ratio: cam.getState().ratio / 1.5 }, { duration: 200 });
  }, []);

  const zoomOut = useCallback(() => {
    const cam = sigmaRef.current?.getCamera();
    if (cam)
      cam.animate({ ratio: cam.getState().ratio * 1.5 }, { duration: 200 });
  }, []);

  // ── Export PNG ───────────────────────────────────────────────────

  const exportPNG = useCallback(() => {
    const sigma = sigmaRef.current;
    if (!sigma) return;
    const canvases = sigma.getCanvases();
    // Merge all canvas layers into one
    const layers = Object.values(canvases);
    if (layers.length === 0) return;
    const w = layers[0].width;
    const h = layers[0].height;
    const merged = document.createElement("canvas");
    merged.width = w;
    merged.height = h;
    const ctx = merged.getContext("2d");
    if (!ctx) return;
    // Dark background
    ctx.fillStyle = "#06060a";
    ctx.fillRect(0, 0, w, h);
    for (const layer of layers) {
      ctx.drawImage(layer, 0, 0);
    }
    merged.toBlob((blob) => {
      if (!blob) return;
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "gitnexus-graph.png";
      a.click();
      URL.revokeObjectURL(url);
    });
  }, []);

  // ── Refresh (trigger reducer re-evaluation) ─────────────────────

  const refresh = useCallback(() => {
    sigmaRef.current?.refresh();
  }, []);

  return {
    containerRef,
    sigmaRef,
    graphRef,
    isLayoutRunning,
    setGraph,
    runLayout,
    stopLayout,
    focusNode,
    fitView,
    zoomIn,
    zoomOut,
    exportPNG,
    refresh,
  };
}
