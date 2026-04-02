import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Workflow, Copy, Check, Search } from "lucide-react";
import { commands } from "../../lib/tauri-commands";
import { useI18n } from "../../hooks/use-i18n";

export function DiagramView() {
  const { t } = useI18n();
  const [target, setTarget] = useState("");
  const [searchTarget, setSearchTarget] = useState("");
  const [copied, setCopied] = useState(false);

  const { data: diagram, isLoading } = useQuery({
    queryKey: ["diagram", searchTarget],
    queryFn: () => commands.getDiagram(searchTarget),
    enabled: searchTarget.length > 0,
    staleTime: 60_000,
  });

  const handleSearch = () => {
    if (target.trim()) {
      setSearchTarget(target.trim());
    }
  };

  const handleCopy = async () => {
    if (diagram?.mermaid) {
      await navigator.clipboard.writeText("```mermaid\n" + diagram.mermaid + "\n```");
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
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
            placeholder="Enter class, controller, or service name..."
            style={{
              width: "100%",
              padding: "8px 12px 8px 32px",
              borderRadius: "var(--radius-md)",
              border: "1px solid var(--surface-border)",
              background: "var(--bg-2)",
              color: "var(--text-0)",
              fontSize: 13,
              outline: "none",
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
          Generate
        </button>
      </div>

      {isLoading && (
        <div style={{ color: "var(--text-3)", padding: 20, textAlign: "center" }}>
          Generating diagram...
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
              {copied ? <Check size={12} /> : <Copy size={12} />}
              {copied ? "Copied!" : "Copy Mermaid"}
            </button>
          </div>

          {/* Mermaid source */}
          <pre
            style={{
              padding: 16,
              borderRadius: "var(--radius-lg)",
              border: "1px solid var(--surface-border)",
              background: "var(--bg-2)",
              color: "var(--text-0)",
              fontSize: 12,
              fontFamily: "var(--font-mono)",
              overflow: "auto",
              whiteSpace: "pre-wrap",
              lineHeight: 1.6,
            }}
          >
            {diagram.mermaid}
          </pre>
        </div>
      )}

      {!diagram && !isLoading && searchTarget && (
        <div style={{ color: "var(--text-3)", padding: 20, textAlign: "center" }}>
          No diagram generated. Symbol may not exist.
        </div>
      )}
    </div>
  );
}
