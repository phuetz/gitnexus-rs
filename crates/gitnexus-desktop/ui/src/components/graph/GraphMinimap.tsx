import { useRef, useCallback, useEffect } from "react";
import { EyeOff } from "lucide-react";
import type Sigma from "sigma";
import type Graph from "graphology";
import type { SigmaNodeAttributes, SigmaEdgeAttributes } from "../../lib/graph-adapter";

interface GraphMinimapProps {
  visible: boolean;
  opacity: number;
  onOpacityChange: (v: number) => void;
  onClose: () => void;
  sigmaRef: React.RefObject<Sigma | null>;
  graphRef: React.RefObject<Graph<SigmaNodeAttributes, SigmaEdgeAttributes>>;
}

export function GraphMinimap({
  visible,
  opacity,
  onOpacityChange,
  onClose,
  sigmaRef,
  graphRef,
}: GraphMinimapProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const sigma = sigmaRef.current;
    const graph = graphRef.current;
    if (!canvas || !sigma || !graph || graph.order === 0) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const canvasW = 160;
    const canvasH = 120;
    ctx.fillStyle = "#1a1b26";
    ctx.fillRect(0, 0, canvasW, canvasH);

    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
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

    // Viewport rectangle
    const cam = sigma.getCamera().getState();
    const dim = sigma.getDimensions();
    const vpHW = (cam.ratio * dim.width) / (2 * dim.width);
    const vpHH = (cam.ratio * dim.height) / (2 * dim.height);

    const normToMiniX = (nx: number) =>
      padding + (nx * graphW + (graphW * 0.5 - graphW * 0.5)) * scale;
    const normToMiniY = (ny: number) =>
      padding + (ny * graphH + (graphH * 0.5 - graphH * 0.5)) * scale;

    const vpX = normToMiniX(cam.x - vpHW);
    const vpY = normToMiniY(cam.y - vpHH);
    const vpW2 = vpHW * 2 * graphW * scale;
    const vpH2 = vpHH * 2 * graphH * scale;

    ctx.strokeStyle = "#7aa2f7";
    ctx.lineWidth = 1.5;
    ctx.fillStyle = "rgba(122, 162, 247, 0.12)";
    ctx.fillRect(vpX, vpY, vpW2, vpH2);
    ctx.strokeRect(vpX, vpY, vpW2, vpH2);
  }, [sigmaRef, graphRef]);

  // Throttled draw via RAF
  const rafRef = useRef<number | null>(null);
  const drawThrottled = useCallback(() => {
    if (rafRef.current) return;
    rafRef.current = requestAnimationFrame(() => {
      draw();
      rafRef.current = null;
    });
  }, [draw]);

  // Attach to sigma afterRender
  useEffect(() => {
    const sigma = sigmaRef.current;
    if (!sigma) return;
    sigma.on("afterRender", drawThrottled);
    return () => {
      sigma.removeListener("afterRender", drawThrottled);
    };
  }, [sigmaRef, drawThrottled]);

  if (!visible) return null;

  return (
    <div
      className="absolute z-15 pointer-events-auto"
      style={{
        bottom: "16px",
        left: "16px",
        borderRadius: "var(--radius-md)",
        backgroundColor: "var(--bg-2)",
        border: "1px solid var(--surface-border)",
        opacity,
        transition: "opacity 0.2s ease",
      }}
      onMouseEnter={() => onOpacityChange(1.0)}
      onMouseLeave={() => onOpacityChange(0.8)}
    >
      <canvas
        ref={canvasRef}
        width={160}
        height={120}
        style={{
          display: "block",
          cursor: "pointer",
          borderRadius: "var(--radius-md)",
        }}
      />
      <button
        onClick={onClose}
        aria-label="Close minimap"
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
  );
}
