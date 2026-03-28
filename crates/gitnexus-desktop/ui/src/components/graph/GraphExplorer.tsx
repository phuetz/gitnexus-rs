import { useCallback, useRef, useEffect, useState } from "react";
import CytoscapeComponent from "react-cytoscapejs";
import type cytoscape from "cytoscape";
import { AlertCircle, Copy, EyeOff, Network } from "lucide-react";
import { useGraphData } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";
import { GraphToolbar } from "./GraphToolbar";
import { NodeHoverCard } from "./NodeHoverCard";
import { ViewModeToggle, type ViewMode } from "./ViewModeToggle";
import { TreemapView } from "./TreemapView";
import { useI18n } from "../../hooks/use-i18n";
import { Tooltip } from "../shared/Tooltip";
import type { GraphFilter, CytoNode, CytoEdge, ZoomLevel } from "../../lib/tauri-commands";

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

const NEXT_ZOOM: Record<ZoomLevel, ZoomLevel | null> = {
  package: "module",
  module: "symbol",
  symbol: null,
};

function buildElements(nodes: CytoNode[], edges: CytoEdge[]) {
  const elements: cytoscape.ElementDefinition[] = [];

  for (const node of nodes) {
    elements.push({
      data: {
        id: node.id,
        label: node.name,
        nodeLabel: node.label,
        filePath: node.filePath,
        startLine: node.startLine,
        endLine: node.endLine,
        color: LABEL_COLORS[node.label] || "#565f89",
      },
    });
  }

  for (const edge of edges) {
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
      width: 38,
      height: 38,
      "border-width": 1.5,
      "border-color": "rgba(255,255,255,0.1)" as any,
      "border-opacity": 1,
      "overlay-padding": 6,
      "text-max-width": "90px" as any,
      "text-wrap": "ellipsis" as any,
      "min-zoomed-font-size": 8,
    },
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
      "font-weight": "bold" as any,
      "z-index": 999,
      "overlay-color": "#7aa2f7",
      "overlay-opacity": 0.12,
      "overlay-padding": 10,
    },
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
      opacity: 0.6,
    },
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
];
/* eslint-enable @typescript-eslint/no-explicit-any */

export function GraphExplorer() {
  const { t, tt } = useI18n();
  const zoomLevel = useAppStore((s) => s.zoomLevel);
  const setZoomLevel = useAppStore((s) => s.setZoomLevel);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const setSidebarTab = useAppStore((s) => s.setSidebarTab);
  const setSearchOpen = useAppStore((s) => s.setSearchOpen);
  const setSearchQuery = useAppStore((s) => s.setSearchQuery);
  const cyRef = useRef<cytoscape.Core | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const minimapCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const [layout, setLayout] = useState("grid");
  const [tooltip, setTooltip] = useState<{
    x: number;
    y: number;
    name: string;
    label: string;
    filePath: string;
  } | null>(null);
  const [legendExpanded, setLegendExpanded] = useState(false);
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
  } | null>(null);
  const [hoverPos, setHoverPos] = useState<{ x: number; y: number } | null>(null);
  const [hoverDegrees, setHoverDegrees] = useState<{ inDeg: number; outDeg: number }>({ inDeg: 0, outDeg: 0 });
  // Unique key per mount to force layout re-run when navigating back
  const [mountId] = useState(() => Date.now());
  const [viewMode, setViewMode] = useState<ViewMode>("graph");

  const filter: GraphFilter = {
    zoomLevel,
    maxNodes: 500,
  };

  const { data, isLoading, error } = useGraphData(filter, true);
  const elements = data ? buildElements(data.nodes, data.edges) : [];

  const handleFit = useCallback(() => {
    cyRef.current?.fit(undefined, 30);
  }, []);

  const drawMinimap = useCallback(() => {
    const canvas = minimapCanvasRef.current;
    const cy = cyRef.current;
    if (!canvas || !cy) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const canvasW = 160;
    const canvasH = 120;

    // Clear canvas
    ctx.fillStyle = "var(--bg-2)";
    ctx.fillRect(0, 0, canvasW, canvasH);

    // Get graph bounds
    const elements = cy.elements();
    if (elements.length === 0) return;

    const bbox = elements.boundingBox();
    const graphW = bbox.w;
    const graphH = bbox.h;
    if (graphW === 0 || graphH === 0) return;
    const scale = Math.min(
      canvasW / graphW,
      canvasH / graphH
    );

    // Draw nodes as dots
    cy.nodes().forEach((node) => {
      const x = (node.position("x") - bbox.x1) * scale;
      const y = (node.position("y") - bbox.y1) * scale;
      const color = node.data("color") || "#565f89";

      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(x, y, 1.5, 0, Math.PI * 2);
      ctx.fill();
    });

    // Draw viewport rectangle
    const extent = cy.extent();
    const vpX = (extent.x1 - bbox.x1) * scale;
    const vpY = (extent.y1 - bbox.y1) * scale;
    const vpW = (extent.x2 - extent.x1) * scale;
    const vpH = (extent.y2 - extent.y1) * scale;

    ctx.strokeStyle = "var(--accent-border)";
    ctx.lineWidth = 1;
    ctx.fillStyle = "rgba(122, 162, 247, 0.15)";
    ctx.fillRect(vpX, vpY, vpW, vpH);
    ctx.strokeRect(vpX, vpY, vpW, vpH);
  }, []);

  const handleMinimapPan = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = minimapCanvasRef.current;
    const cy = cyRef.current;
    if (!canvas || !cy) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const elements = cy.elements();
    const bbox = elements.boundingBox();
    const canvasW = 160;
    const canvasH = 120;
    const scale = Math.min(
      canvasW / bbox.w,
      canvasH / bbox.h
    );

    const graphX = bbox.x1 + x / scale;
    const graphY = bbox.y1 + y / scale;

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
                  nodeRepulsion: () => 8000,
                  idealEdgeLength: () => 120,
                  edgeElasticity: () => 100,
                  gravity: 0.25,
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

      // Double click → zoom to next level
      cy.on("dbltap", "node", () => {
        const next = NEXT_ZOOM[useAppStore.getState().zoomLevel];
        if (next) {
          setZoomLevel(next);
        }
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

      // Minimap events
      cy.on("render", drawMinimap);
      cy.on("pan", drawMinimap);
      cy.on("zoom", drawMinimap);
      cy.on("layoutstop", drawMinimap);
    },
    [setSelectedNodeId, setZoomLevel, drawMinimap]
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

      const layoutOpts: cytoscape.LayoutOptions =
        zoomLevel === "package"
          ? { name: "grid", rows: Math.ceil(Math.sqrt(elements.length)), padding: 40 }
          : {
              name: layout === "cose" ? "cose" : layout,
              animate: false,
              nodeOverlap: 40,
              nodeRepulsion: () => 8000,
              idealEdgeLength: () => 120,
              edgeElasticity: () => 100,
              gravity: 0.25,
              padding: 50,
              randomize: false,
              componentSpacing: 80,
              nestingFactor: 1.2,
            } as cytoscape.LayoutOptions;
      const l = cy.layout(layoutOpts);
      l.on("layoutstop", () => {
        cy.fit(undefined, 40);
      });
      l.run();
    }
  }, [elements.length, zoomLevel, layout, mountId]);

  // Resize observer: auto-fit when container resizes
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => {
      if (cyRef.current && !cyRef.current.destroyed()) {
        cyRef.current.resize();
        cyRef.current.fit(undefined, 30);
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
        />
        <div
          className="flex-1 relative flex items-center justify-center overflow-hidden"
          style={{ backgroundColor: "var(--bg-1)" }}
        >
          {/* Shimmer animation background */}
          <div className="absolute inset-0 opacity-30">
            <div
              className="absolute inset-0"
              style={{
                backgroundImage:
                  "linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent)",
                backgroundSize: "200% 100%",
                animation: "shimmer 2s infinite",
              }}
            />
          </div>

          {/* Loading content */}
          <div className="relative flex flex-col items-center gap-3">
            <div
              className="w-12 h-12 rounded-lg"
              style={{
                backgroundColor: "var(--surface)",
                border: "2px solid",
                borderColor: "var(--accent)",
                borderTopColor: "var(--accent-subtle)",
                animation: "spin 1s linear infinite",
              }}
            />
            <p
              style={{
                color: "var(--text-3)",
                fontSize: "14px",
                fontWeight: "500",
              }}
            >
              {t("graph.loadingGraph")}
            </p>
          </div>

          <style>{`
            @keyframes shimmer {
              0% {
                background-position: -200% 0;
              }
              100% {
                background-position: 200% 0;
              }
            }
            @keyframes spin {
              to {
                transform: rotate(360deg);
              }
            }
          `}</style>
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
      <div ref={containerRef} className="flex-1 relative cytoscape-container">
        <CytoscapeComponent
          key={mountId}
          elements={elements}
          stylesheet={stylesheet}
          layout={{
            name: zoomLevel === "package" ? "grid" : (layout === "cose" ? "cose" : layout),
            ...(zoomLevel === "package"
              ? { rows: Math.ceil(Math.sqrt(elements.length)), padding: 40 }
              : {
                  animate: false,
                  nodeOverlap: 40,
                  nodeRepulsion: () => 8000,
                  idealEdgeLength: () => 120,
                  edgeElasticity: () => 100,
                  gravity: 0.25,
                  padding: 50,
                  randomize: true,
                  componentSpacing: 80,
                }),
          } as cytoscape.LayoutOptions}
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

        {/* Legend overlay — bottom-right to avoid graph overlap */}
        <div
          className="absolute z-15 pointer-events-auto"
          style={{
            bottom: "12px",
            right: "12px",
            borderRadius: "var(--radius-md)",
            backgroundColor: "rgba(14, 17, 24, 0.92)",
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
                  maxWidth: "150px",
                }}
              >
                {data &&
                  data.nodes.length > 0 &&
                  (() => {
                    const nodeTypes = new Map<string, boolean>();
                    data.nodes.forEach((node) => {
                      nodeTypes.set(node.label, true);
                    });

                    const sortedTypes = Array.from(nodeTypes.keys()).sort();

                    return sortedTypes.map((type) => (
                      <div
                        key={type}
                        className="flex items-center gap-2"
                        style={{ padding: "4px 0" }}
                      >
                        <span
                          className="w-2 h-2 rounded-full flex-shrink-0"
                          style={{
                            backgroundColor:
                              LABEL_COLORS[type] || "#565f89",
                          }}
                        />
                        <span
                          className="text-[11px] truncate"
                          style={{
                            color: "var(--text-2)",
                          }}
                        >
                          {type}
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
    </div>
  );
}
