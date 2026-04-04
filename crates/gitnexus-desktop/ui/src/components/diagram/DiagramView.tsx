import { useState, useRef, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { Workflow, Copy, Search } from "lucide-react";
import { commands } from "../../lib/tauri-commands";
import { useI18n } from "../../hooks/use-i18n";

export function DiagramView() {
  const { t } = useI18n();
  const [target, setTarget] = useState("");
  const [searchTarget, setSearchTarget] = useState("");
  const [renderError, setRenderError] = useState(false);
  const mermaidRef = useRef<HTMLDivElement>(null);

  const { data: diagram, isLoading } = useQuery({
    queryKey: ["diagram", searchTarget],
    queryFn: () => commands.getDiagram(searchTarget),
    enabled: searchTarget.length > 0,
    staleTime: 60_000,
  });

  // Render Mermaid diagram as SVG
  useEffect(() => {
    if (!diagram?.mermaid || !mermaidRef.current) return;

    let cancelled = false;
    setRenderError(false);

    (async () => {
      try {
        const mermaid = await import("mermaid");
        mermaid.default.initialize({
          startOnLoad: false,
          theme: "dark",
          themeVariables: {
            primaryColor: "#7aa2f7",
            primaryTextColor: "#c0caf5",
            lineColor: "#565f89",
            secondaryColor: "#bb9af7",
          },
        });

        const id = `mermaid-diagram-${Date.now()}`;
        const { svg } = await mermaid.default.render(id, diagram.mermaid);
        if (!cancelled && mermaidRef.current) {
          mermaidRef.current.innerHTML = svg;
        }
      } catch {
        if (!cancelled) {
          setRenderError(true);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [diagram]);

  const handleSearch = () => {
    if (target.trim()) {
      setRenderError(false);
      setSearchTarget(target.trim());
    }
  };

  const handleCopy = async () => {
    if (diagram?.mermaid) {
      try {
        await navigator.clipboard.writeText("```mermaid\n" + diagram.mermaid + "\n```");
        toast.success(t("diagram.copied"));
      } catch {
        toast.error("Copy failed");
      }
    }
  };

  return (
    <div className="h-full overflow-auto" style={{ padding: 24 }}>
      <h2 className="text-lg font-semibold" style={{ color: "var(--text-0)", marginBottom: 20 }}>
        <Workflow size={20} style={{ display: "inline", marginRight: 8, verticalAlign: "text-bottom" }} />
        {t("sidebar.diagram")}
      </h2>

      {/* Search bar */}
      <div style={{ display: "flex", gap: 8, marginBottom: 24 }}>
        <div style={{ position: "relative", flex: 1 }}>
          <Search
            size={14}
            style={{ position: "absolute", left: 12, top: "50%", transform: "translateY(-50%)", color: "var(--text-3)" }}
          />
          <input
            value={target}
            onChange={(e) => setTarget(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            placeholder={t("diagram.placeholder")}
            style={{
              width: "100%",
              padding: "8px 12px 8px 32px",
              borderRadius: "var(--radius-md)",
              border: "1px solid var(--surface-border)",
              background: "var(--bg-2)",
              color: "var(--text-0)",
              fontSize: 13,
            }}
          />
        </div>
        <button
          onClick={handleSearch}
          style={{
            padding: "8px 16px",
            borderRadius: "var(--radius-md)",
            background: "var(--accent)",
            color: "white",
            fontSize: 12,
            fontWeight: 600,
            border: "none",
            cursor: "pointer",
          }}
        >
          {t("diagram.generate")}
        </button>
      </div>

      {isLoading && (
        <div style={{ color: "var(--text-3)", padding: 20, textAlign: "center" }}>
          {t("diagram.generating")}
        </div>
      )}

      {diagram && (
        <div>
          {/* Header with copy button */}
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              marginBottom: 12,
            }}
          >
            <span style={{ fontSize: 12, color: "var(--text-2)" }}>
              {diagram.targetLabel}: <strong style={{ color: "var(--text-0)" }}>{diagram.targetName}</strong>
            </span>
            <button
              onClick={handleCopy}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 4,
                padding: "4px 10px",
                borderRadius: "var(--radius-sm)",
                border: "1px solid var(--surface-border)",
                background: "var(--bg-2)",
                color: "var(--text-2)",
                fontSize: 11,
                cursor: "pointer",
              }}
            >
              <Copy size={12} />
              {t("diagram.copyMermaid")}
            </button>
          </div>

          {/* Rendered Mermaid diagram */}
          <div
            ref={mermaidRef}
            style={{
              padding: 16,
              borderRadius: "var(--radius-lg)",
              border: "1px solid var(--surface-border)",
              background: "var(--bg-2)",
              overflow: "auto",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              minHeight: 200,
            }}
          />

          {renderError && (
            <pre
              style={{
                marginTop: 12,
                padding: 16,
                borderRadius: "var(--radius-lg)",
                border: "1px solid var(--surface-border)",
                background: "var(--bg-2)",
                color: "var(--text-2)",
                fontSize: 12,
                fontFamily: "var(--font-mono)",
                overflow: "auto",
                whiteSpace: "pre-wrap",
                lineHeight: 1.6,
              }}
            >
              {diagram.mermaid}
            </pre>
          )}
        </div>
      )}

      {!diagram && !isLoading && searchTarget && (
        <div style={{ color: "var(--text-3)", padding: 20, textAlign: "center" }}>
          {t("diagram.noDiagram")}
        </div>
      )}
    </div>
  );
}
