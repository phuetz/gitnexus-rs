import { useAppStore } from "../../stores/app-store";
import { DetailPanel } from "../layout/DetailPanel";
import { CodeInspectorPanel } from "../layout/CodeInspectorPanel";

export function ExplorerRightPanel() {
  const selectedNodeId = useAppStore((s) => s.selectedNodeId);

  if (!selectedNodeId) {
    return (
      <div
        className="flex items-center justify-center h-full"
        style={{
          background: "var(--glass-bg)",
          backdropFilter: "blur(var(--glass-blur))",
          borderLeft: "1px solid var(--glass-border)",
          color: "var(--text-3)",
        }}
      >
        <p className="text-sm" style={{ fontFamily: "var(--font-body)" }}>
          Select a symbol to inspect
        </p>
      </div>
    );
  }

  return (
    <div
      className="flex flex-col h-full overflow-hidden"
      style={{
        background: "var(--glass-bg)",
        backdropFilter: "blur(var(--glass-blur))",
        borderLeft: "1px solid var(--glass-border)",
      }}
    >
      {/* Top: Code preview */}
      <div
        className="shrink-0"
        style={{
          maxHeight: "40%",
          borderBottom: "1px solid var(--surface-border)",
          overflow: "auto",
        }}
      >
        <CodeInspectorPanel />
      </div>

      {/* Bottom: Detail tabs */}
      <div className="flex-1 min-h-0 overflow-auto">
        <DetailPanel />
      </div>
    </div>
  );
}
