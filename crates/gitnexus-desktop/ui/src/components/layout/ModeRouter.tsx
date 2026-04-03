import { useAppStore } from "../../stores/app-store";
import { ErrorBoundary } from "../shared/ErrorBoundary";
import { ExplorerMode } from "../explorer/ExplorerMode";
import { AnalyzeMode } from "../analyze/AnalyzeMode";
import { ManageMode } from "../manage/ManageMode";
import { ChatMode } from "../chat/ChatMode";

export function ModeRouter() {
  const mode = useAppStore((s) => s.mode);

  return (
    <>
      {/* Explorer is ALWAYS mounted to preserve Sigma.js WebGL context.
          Hidden via CSS when not active — no unmount/remount. */}
      <div
        style={{
          visibility: mode === "explorer" ? "visible" : "hidden",
          pointerEvents: mode === "explorer" ? "auto" : "none",
          position: mode === "explorer" ? "relative" : "absolute",
          inset: mode === "explorer" ? undefined : 0,
          width: "100%",
          height: "100%",
        }}
      >
        <ErrorBoundary>
          <ExplorerMode />
        </ErrorBoundary>
      </div>

      {mode === "analyze" && (
        <ErrorBoundary>
          <AnalyzeMode />
        </ErrorBoundary>
      )}
      {mode === "chat" && (
        <ErrorBoundary>
          <ChatMode />
        </ErrorBoundary>
      )}
      {mode === "manage" && (
        <ErrorBoundary>
          <ManageMode />
        </ErrorBoundary>
      )}
    </>
  );
}
