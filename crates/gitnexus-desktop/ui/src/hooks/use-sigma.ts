/**
 * useSigma — wraps all Sigma.js/graphology logic for the graph explorer.
 *
 * Handles: init, graph data, ForceAtlas2 layout, camera ops, node/edge
 * reducers (selection, impact, search highlighting), and PNG export.
 */
import { useRef, useEffect, useCallback, useState } from "react";
import type Sigma from "sigma";
import type Graph from "graphology";
import type {
  SigmaNodeAttributes,
  SigmaEdgeAttributes,
} from "../lib/graph-adapter";
import { dimColor } from "../lib/graph-adapter";

type SigmaRuntime = {
  Sigma: typeof import("sigma").default;
  Graph: typeof import("graphology").default;
  EdgeCurveProgram: typeof import("@sigma/edge-curve").default;
};

type LayoutRuntime = {
  FA2Layout: typeof import("graphology-layout-forceatlas2/worker").default;
  forceAtlas2: typeof import("graphology-layout-forceatlas2").default;
  noverlap: typeof import("graphology-layout-noverlap").default;
};

let sigmaRuntimePromise: Promise<SigmaRuntime> | null = null;
let layoutRuntimePromise: Promise<LayoutRuntime> | null = null;

function loadSigmaRuntime(): Promise<SigmaRuntime> {
  if (!sigmaRuntimePromise) {
    sigmaRuntimePromise = Promise.all([
      import("sigma"),
      import("graphology"),
      import("@sigma/edge-curve"),
    ]).then(([sigma, graphology, edgeCurve]) => ({
      Sigma: sigma.default,
      Graph: graphology.default,
      EdgeCurveProgram: edgeCurve.default,
    }));
  }
  return sigmaRuntimePromise;
}

function loadLayoutRuntime(): Promise<LayoutRuntime> {
  if (!layoutRuntimePromise) {
    layoutRuntimePromise = Promise.all([
      import("graphology-layout-forceatlas2/worker"),
      import("graphology-layout-forceatlas2"),
      import("graphology-layout-noverlap"),
    ]).then(([fa2Worker, forceAtlas2, noverlap]) => ({
      FA2Layout: fa2Worker.default,
      forceAtlas2: forceAtlas2.default,
      noverlap: noverlap.default,
    }));
  }
  return layoutRuntimePromise;
}

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
  const graphRef = useRef<Graph<SigmaNodeAttributes, SigmaEdgeAttributes> | null>(
    null,
  );
  const layoutRef = useRef<{ kill(): void; start(): void } | null>(null);
  const layoutTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const sigmaRuntimeRef = useRef<SigmaRuntime | null>(null);
  const layoutRuntimeRef = useRef<LayoutRuntime | null>(null);
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
    sigmaRef.current?.refresh();
  }, [options.selectedNodeId]);
  useEffect(() => {
    highlightedRef.current = options.highlightedNodeIds;
    sigmaRef.current?.refresh();
  }, [options.highlightedNodeIds]);
  useEffect(() => {
    impactRef.current = options.impactNodeIds;
    sigmaRef.current?.refresh();
  }, [options.impactNodeIds]);
  useEffect(() => {
    egoNodeIdsRef.current = options.egoNodeIds;
    sigmaRef.current?.refresh();
  }, [options.egoNodeIds]);
  useEffect(() => {
    egoDepthMapRef.current = options.egoDepthMap;
    sigmaRef.current?.refresh();
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
    let cancelled = false;
    let themeObserver: MutationObserver | null = null;
    let clickDebounceTimer: ReturnType<typeof setTimeout> | null = null;

    void (async () => {
      if (!containerRef.current) return;

      const runtime = await loadSigmaRuntime();
      if (cancelled || !containerRef.current) return;
      sigmaRuntimeRef.current = runtime;

      const graph =
        graphRef.current ??
        new runtime.Graph<SigmaNodeAttributes, SigmaEdgeAttributes>();
      graphRef.current = graph;

      // Read theme-aware colors from CSS custom properties
      const cssVars = getComputedStyle(document.documentElement);
      const labelColor = cssVars.getPropertyValue("--text-1").trim() || "#e4e4ed";
      const defaultNodeColor = cssVars.getPropertyValue("--text-4").trim() || "#64748b";
      const defaultEdgeColor = cssVars.getPropertyValue("--surface-border").trim() || "#2a2a3a";

      const sigma = new runtime.Sigma(graph, containerRef.current, {
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
          curved: runtime.EdgeCurveProgram,
        },

        // ── Custom hover: dark pill with colored border ──────────
        defaultDrawNodeHover: (context, data) => {
          const label = data.label;
          if (!label) return;
          const fontSize = 11;
          const font = `500 ${fontSize}px JetBrains Mono, monospace`;

          context.save();
          try {
            context.font = font;
            const textWidth = context.measureText(label).width;
            const nodeSize = data.size;
            const x = data.x;
            const y = data.y - nodeSize - 12;
            const padX = 8;
            const padY = 5;
            const w = textWidth + padX * 2;
            const h = fontSize + padY * 2;

            context.fillStyle = "#0d1520";
            context.beginPath();
            context.roundRect(x - w / 2, y - h / 2, w, h, 4);
            context.fill();

            context.strokeStyle = data.color || "#06b6d4";
            context.lineWidth = 1.5;
            context.stroke();

            context.fillStyle = "#e4e4ed";
            context.textAlign = "center";
            context.textBaseline = "middle";
            context.fillText(label, x, y);

            context.beginPath();
            context.arc(data.x, data.y, nodeSize + 3, 0, Math.PI * 2);
            context.strokeStyle = data.color || "#06b6d4";
            context.lineWidth = 1.5;
            context.globalAlpha = 0.4;
            context.stroke();
          } finally {
            context.restore();
          }
        },

        nodeReducer: (node, data) => {
          const res = { ...data };
          const selected = selectedRef.current;
          const highlighted = highlightedRef.current;
          const impact = impactRef.current;

          const egoNodes = egoNodeIdsRef.current;
          const egoDepths = egoDepthMapRef.current;
          if (egoNodes && egoNodes.size > 0) {
            if (egoNodes.has(node)) {
              const d = egoDepths?.get(node) ?? 0;
              if (d <= 1) {
                res.zIndex = 10 - d;
              } else if (d === 2) {
                res.color = dimColor(data.color || "#64748b", 0.8);
                res.zIndex = 3;
              } else {
                res.color = dimColor(data.color || "#64748b", 0.6);
                res.zIndex = 2;
              }
            } else {
              res.color = dimColor(data.color || "#64748b", 0.10);
              res.size = (data.size || 6) * 0.4;
              res.label = "";
              res.zIndex = 0;
            }
            return res;
          }

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

        edgeReducer: (edge, data) => {
          const res = { ...data };
          const selected = selectedRef.current;
          const g = graphRef.current;

          const egoNodes2 = egoNodeIdsRef.current;
          if (egoNodes2 && egoNodes2.size > 0 && g) {
            const [source, target] = g.extremities(edge);
            const srcIn = egoNodes2.has(source);
            const tgtIn = egoNodes2.has(target);
            if (srcIn && tgtIn) {
              res.size = Math.max(1, (data.size || 0.5) * 2);
              res.zIndex = 5;
            } else if (srcIn || tgtIn) {
              res.color = dimColor(data.color || "#2a2a3a", 0.3);
              res.size = (data.size || 0.5) * 0.8;
            } else {
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

      themeObserver = new MutationObserver(() => {
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
        if (clickDebounceTimer) {
          clearTimeout(clickDebounceTimer);
          clickDebounceTimer = null;
        }
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

      sigma.refresh();
    })();

    return () => {
      cancelled = true;
      themeObserver?.disconnect();
      if (clickDebounceTimer) {
        clearTimeout(clickDebounceTimer);
      }
      if (layoutRef.current) {
        layoutRef.current.kill();
        layoutRef.current = null;
      }
      if (layoutTimerRef.current) {
        clearTimeout(layoutTimerRef.current);
        layoutTimerRef.current = null;
      }
      sigmaRef.current?.kill();
      sigmaRef.current = null;
    };
  }, []); // ONE TIME init

  // ── Set graph data ───────────────────────────────────────────────

  const setGraph = useCallback(
    (newGraph: Graph<SigmaNodeAttributes, SigmaEdgeAttributes>) => {
      graphRef.current = newGraph;
      const sigma = sigmaRef.current;
      if (!sigma) return;

      sigma.setGraph(newGraph);
      sigma.refresh();
    },
    [],
  );

  // ── ForceAtlas2 layout ───────────────────────────────────────────

  const runLayout = useCallback(() => {
    const graph = graphRef.current;
    if (!graph || graph.order === 0) return;

    void (async () => {
      const runtime = layoutRuntimeRef.current ?? await loadLayoutRuntime();
      layoutRuntimeRef.current = runtime;

      if (layoutRef.current) {
        layoutRef.current.kill();
        layoutRef.current = null;
      }
      if (layoutTimerRef.current) {
        clearTimeout(layoutTimerRef.current);
        layoutTimerRef.current = null;
      }

      const nodeCount = graph.order;
      const settings = {
        ...runtime.forceAtlas2.inferSettings(graph),
        gravity: nodeCount < 500 ? 0.8 : nodeCount < 2000 ? 0.5 : 0.3,
        scalingRatio: nodeCount < 500 ? 15 : nodeCount < 2000 ? 30 : 60,
        slowDown: nodeCount < 500 ? 1 : 3,
        barnesHutOptimize: nodeCount > 200,
      };

      const layout = new runtime.FA2Layout(graph, { settings });
      layoutRef.current = layout;
      layout.start();
      setIsLayoutRunning(true);

      const duration =
        nodeCount > 2000 ? 15000 : nodeCount > 500 ? 8000 : nodeCount > 50 ? 4000 : 2000;
      layoutTimerRef.current = setTimeout(() => {
        if (layoutRef.current) {
          layoutRef.current.kill();
          layoutRef.current = null;
          runtime.noverlap.assign(graph, {
            maxIterations: 20,
            settings: { ratio: 1.1, margin: 5 },
          });
          sigmaRef.current?.refresh();
          requestAnimationFrame(() => {
            const camera = sigmaRef.current?.getCamera();
            if (camera) camera.animatedReset({ duration: 300 });
          });
          setIsLayoutRunning(false);
        }
      }, duration);
    })();
  }, []);

  // ── Stop layout ──────────────────────────────────────────────────

  const stopLayout = useCallback(() => {
    if (layoutRef.current) {
      layoutRef.current.kill();
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
      // Defer revoke so the browser/Tauri WebView has a chance to start the
      // download before the blob URL is invalidated. Revoking synchronously
      // races the click and produces empty downloads in some browsers.
      setTimeout(() => URL.revokeObjectURL(url), 1000);
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
