import { useCallback, useRef, useEffect, useState } from "react";
import CytoscapeComponent from "react-cytoscapejs";
import type cytoscape from "cytoscape";
import { useGraphData } from "../../hooks/use-tauri-query";
import { useAppStore } from "../../stores/app-store";
import { GraphToolbar } from "./GraphToolbar";
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
      },
    });
  }

  return elements;
}

const stylesheet: cytoscape.StylesheetCSS[] = [
  {
    selector: "node",
    css: {
      label: "data(label)",
      "background-color": "data(color)" as any,
      "font-size": 12,
      color: "#c0caf5",
      "text-valign": "bottom",
      "text-margin-y": 6,
      width: 40,
      height: 40,
      "border-width": 0,
      "overlay-padding": 6,
    },
  },
  {
    selector: "node:selected",
    css: {
      "border-width": 3,
      "border-color": "#7aa2f7",
      width: 50,
      height: 50,
    },
  },
  {
    selector: "edge",
    css: {
      width: 1,
      "line-color": "#292e42",
      "target-arrow-color": "#292e42",
      "target-arrow-shape": "triangle",
      "curve-style": "bezier",
      "arrow-scale": 0.6,
    },
  },
  {
    selector: "edge:selected",
    css: {
      "line-color": "#7aa2f7",
      "target-arrow-color": "#7aa2f7",
      width: 2,
    },
  },
];

export function GraphExplorer() {
  const zoomLevel = useAppStore((s) => s.zoomLevel);
  const setZoomLevel = useAppStore((s) => s.setZoomLevel);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const cyRef = useRef<cytoscape.Core | null>(null);
  const [layout, setLayout] = useState("cose");
  const [tooltip, setTooltip] = useState<{
    x: number;
    y: number;
    name: string;
    label: string;
    filePath: string;
  } | null>(null);

  const filter: GraphFilter = {
    zoomLevel,
    maxNodes: 500,
  };

  const { data, isLoading, error } = useGraphData(filter, true);
  const elements = data ? buildElements(data.nodes, data.edges) : [];

  const handleFit = useCallback(() => {
    cyRef.current?.fit(undefined, 30);
  }, []);

  const handleLayoutChange = useCallback(
    (newLayout: string) => {
      setLayout(newLayout);
      if (!cyRef.current) return;
      const layoutOpts: any =
        newLayout === "grid"
          ? { name: "grid", rows: Math.ceil(Math.sqrt(elements.length)) }
          : newLayout === "circle"
            ? { name: "circle" }
            : newLayout === "breadthfirst"
              ? { name: "breadthfirst" }
              : { name: "cose", animate: false, nodeOverlap: 20 };
      cyRef.current.layout(layoutOpts).run();
    },
    [elements.length]
  );

  const handleCyInit = useCallback(
    (cy: cytoscape.Core) => {
      cyRef.current = cy;

      // Single click → select
      cy.on("tap", "node", (evt) => {
        setSelectedNodeId(evt.target.id());
      });

      // Click background → deselect
      cy.on("tap", (evt) => {
        if (evt.target === cy) {
          setSelectedNodeId(null);
        }
      });

      // Double click → zoom to next level
      cy.on("dbltap", "node", () => {
        const next = NEXT_ZOOM[useAppStore.getState().zoomLevel];
        if (next) {
          setZoomLevel(next);
        }
      });

      // Hover → tooltip
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
      });

      cy.on("mouseout", "node", () => {
        setTooltip(null);
      });
    },
    [setSelectedNodeId, setZoomLevel]
  );

  // Run layout when elements change, then fit to screen
  useEffect(() => {
    if (cyRef.current && elements.length > 0) {
      const cy = cyRef.current;
      const layoutOpts: any =
        zoomLevel === "package"
          ? { name: "grid", rows: Math.ceil(Math.sqrt(elements.length)) }
          : { name: layout, animate: false, nodeOverlap: 20 };
      const l = cy.layout(layoutOpts);
      l.on("layoutstop", () => {
        cy.fit(undefined, 40);
      });
      l.run();
    }
  }, [elements.length, zoomLevel, layout]);

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center text-[var(--text-muted)]">
        Loading graph...
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex items-center justify-center text-[var(--danger)]">
        Error: {String(error)}
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <GraphToolbar
        stats={data?.stats}
        layout={layout}
        onLayoutChange={handleLayoutChange}
        onFit={handleFit}
      />
      <div className="flex-1 bg-[var(--bg-primary)] relative">
        <CytoscapeComponent
          elements={elements}
          stylesheet={stylesheet}
          cy={handleCyInit}
          style={{ width: "100%", height: "100%" }}
        />
        {/* Tooltip overlay */}
        {tooltip && (
          <div
            className="absolute pointer-events-none z-50 px-2 py-1 rounded bg-[var(--bg-secondary)] border border-[var(--border)] shadow-lg text-xs max-w-[250px]"
            style={{
              left: tooltip.x,
              top: tooltip.y,
              transform: "translate(-50%, -100%)",
            }}
          >
            <div className="flex items-center gap-1.5">
              <span
                className="w-2 h-2 rounded-full shrink-0"
                style={{ backgroundColor: LABEL_COLORS[tooltip.label] || "#565f89" }}
              />
              <span className="font-medium truncate">{tooltip.name}</span>
            </div>
            <p className="text-[10px] text-[var(--text-muted)] truncate">{tooltip.filePath}</p>
          </div>
        )}
      </div>
    </div>
  );
}
