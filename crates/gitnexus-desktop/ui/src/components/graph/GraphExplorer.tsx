import { useCallback, useMemo, useRef, useEffect, useState } from "react";
import CytoscapeComponent from "react-cytoscapejs";
import type cytoscape from "cytoscape";
import { useQuery } from "@tanstack/react-query";
import { AlertCircle, Copy, EyeOff, Network, Zap } from "lucide-react";
import { useGraphData } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";
import { commands } from "../../lib/tauri-commands";
import { GraphToolbar } from "./GraphToolbar";
import { NodeHoverCard } from "./NodeHoverCard";
import { ViewModeToggle, type ViewMode } from "./ViewModeToggle";
import { TreemapView } from "./TreemapView";
import { useI18n } from "../../hooks/use-i18n";
import { Tooltip } from "../shared/Tooltip";
import { LoadingOrbs } from "../shared/LoadingOrbs";
import { CypherQueryFAB } from "./CypherQueryFAB";
import { ProcessFlowModal } from "./ProcessFlowModal";
import type { GraphFilter, CytoNode, CytoEdge } from "../../lib/tauri-commands";

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

const LABEL_SIZES: Record<string, number> = {
  Project: 50,
  Package: 42,
  Folder: 36,
  Community: 36,
  Process: 34,
  Class: 30,
  Interface: 28,
  Struct: 28,
  Trait: 28,
  Module: 28,
  File: 24,
  Enum: 22,
  Namespace: 22,
  Route: 20,
  Function: 16,
  Method: 14,
  Constructor: 14,
  Property: 12,
  Tool: 12,
  Variable: 10,
  Type: 10,
  Import: 8,
};

const COMMUNITY_COLORS = [
  "#7aa2f7", "#bb9af7", "#9ece6a", "#e0af68", "#f7768e",
  "#73daca", "#7dcfff", "#ff9e64", "#c0caf5", "#565f89",
  "#2ac3de", "#b4f9f8",
];

function hashString(s: string): number {
  let hash = 0;
  for (let i = 0; i < s.length; i++) {
    hash = ((hash << 5) - hash + s.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

function buildElements(nodes: CytoNode[], edges: CytoEdge[], hiddenEdgeTypes?: Set<string>) {
  const elements: cytoscape.ElementDefinition[] = [];

  for (const node of nodes) {
    const size = LABEL_SIZES[node.label] || 16;
    const color = node.community
      ? COMMUNITY_COLORS[hashString(node.community) % COMMUNITY_COLORS.length]
      : LABEL_COLORS[node.label] || "#565f89";
    elements.push({
      data: {
        id: node.id,
        label: node.name,
        nodeLabel: node.label,
        filePath: node.filePath,
        startLine: node.startLine,
        endLine: node.endLine,
        parameterCount: node.parameterCount,
        returnType: node.returnType,
        isTraced: node.isTraced,
        isDeadCandidate: node.isDeadCandidate,
        color,
        size,
      },
    });
  }

  const filteredEdges = hiddenEdgeTypes
    ? edges.filter(e => !hiddenEdgeTypes.has(e.relType))
    : edges;

  for (const edge of filteredEdges) {
    elements.push({
      data: {
        id: edge.id,
        source: edge.source,
        target: edge.target,
        label: edge.relType,
        color: EDGE_COLORS[edge.relType] || "#3b4261",
      },
    });
  }

  return elements;
}

/** Edge colors by relationship type for better readability */
const EDGE_COLORS: Record<string, string> = {
  CALLS: "#7aa2f7",
  CONTAINS: "#565f89",
  IMPORTS: "#9ece6a",
  IMPLEMENTS: "#bb9af7",
  EXTENDS: "#e0af68",
  USES: "#7dcfff",
  DEPENDS_ON: "#ff9e64",
  REFERENCES: "#73daca",
};

// NOTE: `as any` casts are required — @types/cytoscape is missing many valid CSS properties.
/* eslint-disable @typescript-eslint/no-explicit-any */
const stylesheet: cytoscape.StylesheetCSS[] = [
  {
    selector: "node",
    css: {
      label: "data(label)",
      "background-color": "data(color)" as any,
      "font-size": 10,
      color: "#c0caf5",
      "text-valign": "bottom",
      "text-margin-y": 5,
      width: "data(size)" as any,
      height: "data(size)" as any,
      "border-width": 1.5,
      "border-color": "rgba(255,255,255,0.1)" as any,
      "border-opacity": 1,
      "overlay-padding": 6,
      "text-max-width": "90px" as any,
      "text-wrap": "ellipsis" as any,
      "min-zoomed-font-size": 8,
      "text-background-opacity": 0.7,
      "text-background-color": "#090b10",
      "text-background-padding": "2px",
      "text-background-shape": "roundrectangle",
      "shadow-blur": 12,
      "shadow-color": "data(color)",
      "shadow-opacity": 0.2,
      "shadow-offset-x": 0,
      "shadow-offset-y": 0,
    } as any,
  },
  {
    selector: "node:selected",
    css: {
      "border-width": 3,
      "border-color": "#7aa2f7",
      "border-opacity": 1,
      width: 50,
      height: 50,
      "font-size": 12,
      "font-weight": "bold",
      "z-index": 999,
      "overlay-color": "#7aa2f7",
      "overlay-opacity": 0.12,
      "overlay-padding": 10,
      "shadow-blur": 25,
      "shadow-color": "#7aa2f7",
      "shadow-opacity": 0.5,
      "shadow-offset-x": 0,
      "shadow-offset-y": 0,
    } as any,
  },
  {
    selector: "node:active",
    css: {
      "overlay-color": "#7aa2f7",
      "overlay-opacity": 0.15,
    },
  },
  {
    selector: "edge",
    css: {
      width: 1.5,
      "line-color": "data(color)" as any,
      "target-arrow-color": "data(color)" as any,
      "target-arrow-shape": "triangle",
      "curve-style": "bezier",
      "arrow-scale": 0.7,
      opacity: 0.75,
      "underlay-color": "data(color)",
      "underlay-padding": 2,
      "underlay-opacity": 0.06,
    } as any,
  },
  {
    selector: "edge:selected",
    css: {
      "line-color": "#7aa2f7",
      "target-arrow-color": "#7aa2f7",
      width: 2.5,
      opacity: 1,
      "z-index": 999,
    },
  },
  // Impact overlay classes
  {
    selector: "node.impact-origin",
    css: {
      "border-width": 4,
      "border-color": "#f7768e",
      "shadow-blur": 30,
      "shadow-color": "#f7768e",
      "shadow-opacity": 0.7,
    } as any,
  },
  {
    selector: "node.impact-depth-1",
    css: {
      "border-width": 3,
      "border-color": "#ff9e64",
      "shadow-blur": 20,
      "shadow-color": "#ff9e64",
      "shadow-opacity": 0.5,
    } as any,
  },
  {
    selector: "node.impact-depth-2",
    css: {
      "border-width": 2,
      "border-color": "#e0af68",
      "shadow-blur": 15,
      "shadow-color": "#e0af68",
      "shadow-opacity": 0.4,
    } as any,
  },
  {
    selector: "node.impact-depth-3",
    css: {
      "border-width": 2,
      "border-color": "#9ece6a",
      "shadow-blur": 10,
      "shadow-color": "#9ece6a",
      "shadow-opacity": 0.3,
    } as any,
  },
  {
    selector: "node.impact-dimmed",
    css: {
      opacity: 0.2,
    } as any,
  },
];
/* eslint-enable @typescript-eslint/no-explicit-any */

export function GraphExplorer() {
  const { t, tt } = useI18n();
  const zoomLevel = useAppStore((s) => s.zoomLevel);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const setSidebarTab = useAppStore((s) => s.setSidebarTab);
  const setSearchOpen = useAppStore((s) => s.setSearchOpen);
  const setSearchQuery = useAppStore((s) => s.setSearchQuery);
  const cyRef = useRef<cytoscape.Core | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const minimapCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const [layout, setLayout] = useState("cose");
  const [tooltip, setTooltip] = useState<{
    x: number;
    y: number;
    name: string;
    label: string;
    filePath: string;
  } | null>(null);
  const [legendExpanded, setLegendExpanded] = useState(false);
  const [flowsOpen, setFlowsOpen] = useState(false);
  const [minimapVisible, setMinimapVisible] = useState(true);
  const [minimapOpacity, setMinimapOpacity] = useState(0.8);
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    nodeId: string;
    name: string;
    filePath: string;
  } | null>(null);
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
  } | null>(null);
  const [hoverPos, setHoverPos] = useState<{ x: number; y: number } | null>(null);
  const [hoverDegrees, setHoverDegrees] = useState<{ inDeg: number; outDeg: number }>({ inDeg: 0, outDeg: 0 });
  // Unique key per mount to force layout re-run when navigating back
  const [mountId] = useState(() => Date.now());
  const [viewMode, setViewMode] = useState<ViewMode>("graph");
  const [impactOverlay, setImpactOverlay] = useState(false);
  const [layoutRunning, setLayoutRunning] = useState(false);
  const [focusNodeId, setFocusNodeId] = useState<string | null>(null);
  const [hiddenEdgeTypes] = useState<Set<string>>(
    new Set(["IMPORTS", "HAS_METHOD", "HAS_PROPERTY", "CONTAINS"])
  );
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);

  // Impact overlay: highlight affected nodes when toggled
  const toggleImpactOverlay = useCallback(async () => {
    const cy = cyRef.current;
    if (!cy) return;

    if (impactOverlay) {
      // Clear overlay
      cy.nodes().removeClass("impact-origin impact-depth-1 impact-depth-2 impact-depth-3 impact-dimmed");
      setImpactOverlay(false);
      return;
    }

    const nodeId = selectedNodeId;
    if (!nodeId) return;

    try {
      const result = await commands.getImpactAnalysis(nodeId, "both", 3);
      const affectedIds = new Set<string>();

      // Mark origin
      const originNode = cy.getElementById(nodeId);
      if (originNode.length) {
        originNode.addClass("impact-origin");
        affectedIds.add(nodeId);
      }

      // Mark upstream + downstream by depth
      const markNodes = (items: Array<{ id: string; depth: number }>) => {
        for (const item of items) {
          affectedIds.add(item.id);
          const n = cy.getElementById(item.id);
          if (n.length) {
            if (item.depth <= 1) n.addClass("impact-depth-1");
            else if (item.depth <= 2) n.addClass("impact-depth-2");
            else n.addClass("impact-depth-3");
          }
        }
      };

      if (result.upstream) markNodes(result.upstream.map(n => ({ id: n.node.id, depth: n.depth })));
      if (result.downstream) markNodes(result.downstream.map(n => ({ id: n.node.id, depth: n.depth })));

      // Dim non-affected nodes
      cy.nodes().forEach((n) => {
        if (!affectedIds.has(n.id())) {
          n.addClass("impact-dimmed");
        }
      });

      setImpactOverlay(true);
    } catch {
      // silently fail
    }
  }, [selectedNodeId, impactOverlay]);

  const filter: GraphFilter = {
    zoomLevel,
    maxNodes: 200,
  };

  const { data, isLoading, error } = useGraphData(filter, true);

  const { data: subgraphData } = useQuery({
    queryKey: ["subgraph", focusNodeId],
    queryFn: () => commands.getSubgraph(focusNodeId!, 2),
    enabled: !!focusNodeId,
    staleTime: 30_000,
  });

  const activeData = focusNodeId && subgraphData ? subgraphData : data;
  const elements = useMemo(
    () => (activeData ? buildElements(activeData.nodes, activeData.edges, hiddenEdgeTypes) : []),
    [activeData, hiddenEdgeTypes]
  );
  const dataVersion = useMemo(
    () => (data ? `${data.stats.nodeCount}-${data.stats.edgeCount}-${zoomLevel}` : ""),
    [data, zoomLevel]
  );

  const handleFit = useCallback(() => {
    cyRef.current?.fit(undefined, 30);
  }, []);

  const handleExportPNG = useCallback(() => {
    const cy = cyRef.current;
    if (!cy) return;
    const png = cy.png({ output: "blob", scale: 2, bg: "#090b10" });
    const url = URL.createObjectURL(png as Blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "gitnexus-graph.png";
    a.click();
    URL.revokeObjectURL(url);
  }, []);

  // Global keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Ctrl+G: Go to symbol (open search)
      if ((e.ctrlKey || e.metaKey) && e.key === "g") {
        e.preventDefault();
        setSearchOpen(true);
      }
      // Ctrl+E: Export graph PNG
      if ((e.ctrlKey || e.metaKey) && e.key === "e") {
        e.preventDefault();
        handleExportPNG();
      }
      // Escape: clear selection
      if (e.key === "Escape" && !e.ctrlKey && !e.metaKey) {
        setSelectedNodeId(null);
        setContextMenu(null);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [handleExportPNG, setSearchOpen, setSelectedNodeId]);

  const drawMinimap = useCallback(() => {
    const canvas = minimapCanvasRef.current;
    const cy = cyRef.current;
    if (!canvas || !cy) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const canvasW = 160;
    const canvasH = 120;

    // Resolve CSS variables via getComputedStyle (canvas 2D context cannot use var())
    const computedStyle = getComputedStyle(document.documentElement);
    const bgColor = computedStyle.getPropertyValue("--bg-2").trim() || "#1a1b26";
    const accentBorder = computedStyle.getPropertyValue("--accent-border").trim() || "#7aa2f7";

    // Clear canvas with resolved background color
    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, canvasW, canvasH);

    // Get graph bounds
    const elements = cy.elements();
    if (elements.length === 0) return;

    const bbox = elements.boundingBox();
    const graphW = bbox.w;
    const graphH = bbox.h;
    if (graphW === 0 || graphH === 0) return;

    // Add padding so dots don't sit on the very edge
    const padding = 8;
    const innerW = canvasW - padding * 2;
    const innerH = canvasH - padding * 2;
    const scale = Math.min(innerW / graphW, innerH / graphH);

    // Draw nodes as colored dots
    cy.nodes().forEach((node) => {
      const x = padding + (node.position("x") - bbox.x1) * scale;
      const y = padding + (node.position("y") - bbox.y1) * scale;
      const color = node.data("color") || "#565f89";

      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(x, y, 2, 0, Math.PI * 2);
      ctx.fill();
    });

    // Draw viewport rectangle showing the current visible area
    const extent = cy.extent();
    const vpX = padding + (extent.x1 - bbox.x1) * scale;
    const vpY = padding + (extent.y1 - bbox.y1) * scale;
    const vpW = (extent.x2 - extent.x1) * scale;
    const vpH = (extent.y2 - extent.y1) * scale;

    ctx.strokeStyle = accentBorder;
    ctx.lineWidth = 1.5;
    ctx.fillStyle = "rgba(122, 162, 247, 0.12)";
    ctx.fillRect(vpX, vpY, vpW, vpH);
    ctx.strokeRect(vpX, vpY, vpW, vpH);
  }, []);

  const minimapRafRef = useRef<number | null>(null);
  const drawMinimapThrottled = useCallback(() => {
    if (minimapRafRef.current) return;
    minimapRafRef.current = requestAnimationFrame(() => {
      drawMinimap();
      minimapRafRef.current = null;
    });
  }, [drawMinimap]);

  const handleMinimapPan = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = minimapCanvasRef.current;
    const cy = cyRef.current;
    if (!canvas || !cy) return;

    const rect = canvas.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const clickY = e.clientY - rect.top;

    const elements = cy.elements();
    if (elements.length === 0) return;
    const bbox = elements.boundingBox();
    const canvasW = 160;
    const canvasH = 120;
    const padding = 8;
    const innerW = canvasW - padding * 2;
    const innerH = canvasH - padding * 2;
    const scale = Math.min(innerW / bbox.w, innerH / bbox.h);

    const graphX = bbox.x1 + (clickX - padding) / scale;
    const graphY = bbox.y1 + (clickY - padding) / scale;

    cy.pan({
      x: cy.width() / 2 - graphX,
      y: cy.height() / 2 - graphY,
    });
  }, []);

  const handleLayoutChange = useCallback(
    (newLayout: string) => {
      setLayout(newLayout);
      if (!cyRef.current) return;
      const layoutOpts: cytoscape.LayoutOptions =
        newLayout === "grid"
          ? { name: "grid", rows: Math.ceil(Math.sqrt(elements.length)), padding: 40 }
          : newLayout === "circle"
            ? { name: "circle", padding: 40 }
            : newLayout === "breadthfirst"
              ? { name: "breadthfirst", padding: 40 }
              : {
                  name: "cose",
                  animate: false,
                  nodeOverlap: 40,
                  nodeRepulsion: () => 6000,
                  idealEdgeLength: () => 100,
                  edgeElasticity: () => 40,
                  gravity: 0.3,
                  numIter: 1500,
                  padding: 50,
                  randomize: true,
                  componentSpacing: 80,
                  nestingFactor: 1.2,
                } as cytoscape.LayoutOptions;
      cyRef.current.layout(layoutOpts).run();
    },
    [elements.length]
  );

  const handleCyInit = useCallback(
    (cy: cytoscape.Core) => {
      cyRef.current = cy;

      // Single click → select
      cy.on("tap", "node", (evt) => {
        const node = evt.target;
        setSelectedNodeId(node.id(), node.data("label") as string); // "label" in cytoscape data = display name (set from node.name in buildElements)
      });

      // Click background → deselect and close context menu
      cy.on("tap", (evt) => {
        if (evt.target === cy) {
          setSelectedNodeId(null, null);
          setContextMenu(null);
        }
      });

      // Double click → load subgraph centered on this node
      cy.on("dbltap", "node", (evt) => {
        const nodeId = evt.target.id();
        setFocusNodeId(nodeId);
      });

      // Hover → tooltip + hover card
      cy.on("mouseover", "node", (evt) => {
        const node = evt.target;
        const pos = node.renderedPosition();
        setTooltip({
          x: pos.x,
          y: pos.y - 30,
          name: node.data("label"),
          label: node.data("nodeLabel"),
          filePath: node.data("filePath"),
        });
        setHoveredNode({
          id: node.id(),
          name: node.data("label"),
          label: node.data("nodeLabel"),
          filePath: node.data("filePath"),
          startLine: node.data("startLine"),
          endLine: node.data("endLine"),
          parameterCount: node.data("parameterCount"),
          returnType: node.data("returnType"),
          isTraced: node.data("isTraced"),
          isDeadCandidate: node.data("isDeadCandidate"),
        });
        setHoverPos({ x: pos.x, y: pos.y });
        setHoverDegrees({
          inDeg: node.indegree(false),
          outDeg: node.outdegree(false),
        });
      });

      cy.on("mouseout", "node", () => {
        setTooltip(null);
        setHoveredNode(null);
        setHoverPos(null);
      });

      // Right-click → context menu
      cy.on("cxttap", "node", (evt) => {
        evt.originalEvent.preventDefault();
        const node = evt.target;
        const renderedPos = node.renderedPosition();
        setContextMenu({
          x: renderedPos.x,
          y: renderedPos.y,
          nodeId: node.id(),
          name: node.data("label"),
          filePath: node.data("filePath"),
        });
      });

      // Edge hover → highlight with relation-type color
      cy.on("mouseover", "edge", (evt) => {
        const edge = evt.target;
        const relType = edge.data("label") || "";
        const color = EDGE_COLORS[relType] || "#7aa2f7";
        edge.style({
          "line-color": color,
          "target-arrow-color": color,
          width: 2.5,
          opacity: 1,
        });
      });

      cy.on("mouseout", "edge", (evt) => {
        const edge = evt.target;
        edge.removeStyle();
      });

      // Minimap events (no "render" — too frequent, causes jank)
      cy.on("pan", drawMinimapThrottled);
      cy.on("zoom", drawMinimapThrottled);
      cy.on("layoutstop", drawMinimapThrottled);
    },
    [setSelectedNodeId, drawMinimapThrottled]
  );

  // Clean up Cytoscape listeners on unmount
  useEffect(() => {
    return () => {
      if (cyRef.current && !cyRef.current.destroyed()) {
        cyRef.current.removeAllListeners();
      }
    };
  }, [mountId]);

  // Run layout when elements change, then fit to screen
  useEffect(() => {
    if (cyRef.current && elements.length > 0) {
      const cy = cyRef.current;

      // Scatter nodes randomly first so layouts have good initial positions
      const w = cy.width() || 800;
      const h = cy.height() || 600;
      cy.nodes().forEach((n) => {
        n.position({
          x: Math.random() * w * 0.8 + w * 0.1,
          y: Math.random() * h * 0.8 + h * 0.1,
        });
      });

      setLayoutRunning(true);

      const layoutOpts: cytoscape.LayoutOptions =
        zoomLevel === "package"
          ? { name: "grid", rows: Math.ceil(Math.sqrt(elements.length)), padding: 40 }
          : {
              name: layout === "cose" ? "cose" : layout,
              animate: false,
              nodeOverlap: 40,
              nodeRepulsion: () => 6000,
              idealEdgeLength: () => 100,
              edgeElasticity: () => 40,
              gravity: 0.3,
              numIter: 1500,
              padding: 50,
              randomize: false,
              componentSpacing: 80,
              nestingFactor: 1.2,
            } as cytoscape.LayoutOptions;
      const l = cy.layout(layoutOpts);
      l.on("layoutstop", () => {
        cy.fit(undefined, 40);
        setLayoutRunning(false);
      });
      l.run();
    }
  }, [dataVersion, layout, mountId]);

  // Resize observer: auto-fit when container resizes
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      if (cyRef.current && !cyRef.current.destroyed()) {
        cyRef.current.resize();
      }
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // Close context menu on Escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setContextMenu(null);
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  // Close context menu on scroll
  useEffect(() => {
    const handleScroll = () => {
      setContextMenu(null);
    };
    const container = containerRef.current;
    if (container) {
      container.addEventListener("scroll", handleScroll);
      return () => container.removeEventListener("scroll", handleScroll);
    }
  }, []);

  // Listen for keyboard shortcut custom events (F=fit, L=cycle layout)
  useEffect(() => {
    const onFit = () => {
      cyRef.current?.fit(undefined, 30);
    };
    const onCycleLayout = () => {
      const layouts = ["cose", "grid", "circle", "breadthfirst"];
      const idx = layouts.indexOf(layout);
      const next = layouts[(idx + 1) % layouts.length];
      handleLayoutChange(next);
    };
    window.addEventListener("gitnexus:fit-graph", onFit);
    window.addEventListener("gitnexus:cycle-layout", onCycleLayout);
    return () => {
      window.removeEventListener("gitnexus:fit-graph", onFit);
      window.removeEventListener("gitnexus:cycle-layout", onCycleLayout);
    };
  }, [layout, handleLayoutChange]);

  if (isLoading) {
    return (
      <div className="h-full flex flex-col">
        <GraphToolbar
          stats={undefined}
          layout={layout}
          onLayoutChange={handleLayoutChange}
          onFit={handleFit}
            onExport={handleExportPNG}
        />
        <div className="flex-1">
          <LoadingOrbs label={t("graph.loadingGraph")} />
        </div>
      </div>
    );
  }

  if (data && data.nodes.length === 0) {
    return (
      <div className="h-full flex flex-col">
        <GraphToolbar
          stats={data?.stats}
          layout={layout}
          onLayoutChange={handleLayoutChange}
          onFit={handleFit}
            onExport={handleExportPNG}
        />
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
          <p className="text-lg font-medium">No graph data available</p>
          <p className="text-sm">Analyze a repository first to see the knowledge graph.</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex flex-col">
        <GraphToolbar
          stats={data?.stats}
          layout={layout}
          onLayoutChange={handleLayoutChange}
          onFit={handleFit}
            onExport={handleExportPNG}
        />
        <div
          className="flex-1 relative flex items-center justify-center"
          style={{ backgroundColor: "var(--bg-1)" }}
        >
          <div className="flex flex-col items-center gap-3">
            <div
              className="p-3 rounded-lg"
              style={{
                backgroundColor: "var(--rose)",
                opacity: 0.15,
              }}
            >
              <AlertCircle
                size={24}
                style={{
                  color: "var(--rose)",
                }}
              />
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
              <p
                style={{
                  color: "var(--text-4)",
                  fontSize: "12px",
                }}
              >
                {String(error)}
              </p>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <div style={{ flex: 1 }}>
          <GraphToolbar
            stats={data?.stats}
            layout={layout}
            onLayoutChange={handleLayoutChange}
            onFit={handleFit}
            onExport={handleExportPNG}
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
      <div ref={containerRef} className="flex-1 relative cytoscape-container" role="img" aria-label="Knowledge graph visualization">
        {layoutRunning && (
          <div className="absolute inset-0 z-30 flex items-center justify-center"
            style={{ backgroundColor: "rgba(9, 11, 16, 0.5)", backdropFilter: "blur(2px)" }}>
            <div style={{ color: "var(--text-2)", fontSize: 13 }}>Computing layout...</div>
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
            &larr; Back to full graph
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
            Showing top {data.stats.nodeCount} nodes by importance. Double-click a node to explore its neighborhood.
          </div>
        )}

        <CytoscapeComponent
          key={mountId}
          elements={elements}
          stylesheet={stylesheet}
          layout={{ name: "preset" } as cytoscape.LayoutOptions}
          cy={handleCyInit}
          style={{ width: "100%", height: "100%" }}
        />
        {/* Tooltip overlay */}
        {tooltip && (
          <div
            className="absolute pointer-events-none z-50 rounded-lg text-xs max-w-[250px]"
            style={{
              left: tooltip.x,
              top: tooltip.y,
              transform: "translate(-50%, -100%)",
              backgroundColor: "var(--surface)",
              border: "1px solid",
              borderColor: "var(--surface-border)",
              backdropFilter: "blur(8px)",
              boxShadow: "var(--shadow-lg)",
              padding: "8px 12px",
            }}
          >
            <div className="flex items-center gap-1.5">
              <span
                className="w-2 h-2 rounded-full shrink-0"
                style={{
                  backgroundColor:
                    LABEL_COLORS[tooltip.label] || "#565f89",
                }}
              />
              <span
                className="font-medium truncate"
                style={{
                  color: "var(--text-1)",
                }}
              >
                {tooltip.name}
              </span>
            </div>
            <p
              className="text-[10px] truncate mt-1"
              style={{
                color: "var(--text-4)",
              }}
            >
              {tooltip.filePath}
            </p>
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
              setSelectedNodeId("File:" + hoveredNode.filePath, hoveredNode.name);
            }
          }}
          onImpact={() => {
            if (hoveredNode) {
              setSelectedNodeId(hoveredNode.id, hoveredNode.name);
              setSidebarTab("impact");
            }
          }}
        />

        {/* Context menu overlay */}
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
              <button
                onClick={() => {
                  setSidebarTab("files");
                  setSelectedNodeId("File:" + contextMenu.filePath, contextMenu.name);
                  setContextMenu(null);
                }}
                className="w-full text-left transition-colors"
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
                {tt("graph.contextMenu.goToDefinition").label}
              </button>
            </Tooltip>
            <Tooltip content={tt("graph.contextMenu.findReferences").tip}>
              <button
                onClick={() => {
                  setSearchQuery(contextMenu.name);
                  setSearchOpen(true);
                  setContextMenu(null);
                }}
                className="w-full text-left transition-colors"
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
                {tt("graph.contextMenu.findReferences").label}
              </button>
            </Tooltip>
            <div
              style={{
                borderTop: "1px solid var(--surface-border)",
                margin: "4px 0",
              }}
            />
            {/* View Impact */}
            <button
              onClick={() => {
                setSelectedNodeId(contextMenu.nodeId, contextMenu.name);
                setSidebarTab("impact");
                setContextMenu(null);
              }}
              className="w-full text-left transition-colors"
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
              View Impact
            </button>
            <Tooltip content={tt("graph.contextMenu.expandNeighbors").tip}>
              <button
                onClick={() => {
                  setContextMenu(null);
                }}
                className="w-full text-left transition-colors"
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
                {tt("graph.contextMenu.expandNeighbors").label}
              </button>
            </Tooltip>
            <Tooltip content={tt("graph.contextMenu.hideNode").tip}>
              <button
                onClick={() => {
                  cyRef.current?.getElementById(contextMenu.nodeId).remove();
                  setContextMenu(null);
                }}
                className="w-full text-left transition-colors"
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
                {tt("graph.contextMenu.hideNode").label}
              </button>
            </Tooltip>
            <div
              style={{
                borderTop: "1px solid var(--surface-border)",
                margin: "4px 0",
              }}
            />
            <Tooltip content={tt("graph.contextMenu.copyName").tip}>
              <button
                onClick={() => {
                  navigator.clipboard.writeText(contextMenu.name);
                  setContextMenu(null);
                }}
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
                <Copy size={14} style={{ marginRight: "8px" }} />
                {tt("graph.contextMenu.copyName").label}
              </button>
            </Tooltip>
            <Tooltip content={tt("graph.contextMenu.copyFilePath").tip}>
              <button
                onClick={() => {
                  navigator.clipboard.writeText(contextMenu.filePath);
                  setContextMenu(null);
                }}
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
                <Copy size={14} style={{ marginRight: "8px" }} />
                {tt("graph.contextMenu.copyFilePath").label}
              </button>
            </Tooltip>
          </div>
        )}

        {/* Minimap overlay — bottom-left */}
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
              onClick={handleMinimapPan}
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
                e.currentTarget.style.backgroundColor = "rgba(0, 0, 0, 0.7)";
                e.currentTarget.style.color = "var(--text-0)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = "rgba(0, 0, 0, 0.5)";
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
              boxShadow: impactOverlay ? "0 0 20px rgba(247, 118, 142, 0.3)" : "var(--shadow-sm)",
            }}
          >
            <Zap size={13} />
            {impactOverlay ? "Clear Impact" : "Impact Overlay"}
          </button>
        )}

        {/* Cypher query FAB */}
        <CypherQueryFAB />

        {/* Legend overlay — bottom-right to avoid graph overlap */}
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
                  ×
                </button>
              </div>
              <div
                className="space-y-1 max-h-[calc(8*28px)] overflow-y-auto"
                style={{
                  maxWidth: "180px",
                }}
              >
                {data &&
                  data.nodes.length > 0 &&
                  (() => {
                    // Count nodes per label type
                    const labelCounts = new Map<string, number>();
                    data.nodes.forEach((node) => {
                      labelCounts.set(node.label, (labelCounts.get(node.label) || 0) + 1);
                    });

                    // Sort by count descending, then alphabetically
                    const sortedEntries = Array.from(labelCounts.entries()).sort(
                      (a, b) => b[1] - a[1] || a[0].localeCompare(b[0])
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
                          style={{
                            color: "var(--text-1)",
                          }}
                        >
                          {type}
                        </span>
                        <span
                          className="text-[10px] ml-auto flex-shrink-0"
                          style={{
                            color: "var(--text-3)",
                          }}
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
