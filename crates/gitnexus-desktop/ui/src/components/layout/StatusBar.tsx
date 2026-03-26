import { useAppStore } from "../../stores/app-store";

export function StatusBar() {
  const activeRepo = useAppStore((s) => s.activeRepo);
  const zoomLevel = useAppStore((s) => s.zoomLevel);

  return (
    <div
      className="h-[22px] flex items-center px-3 gap-5 text-[11px] shrink-0 select-none"
      style={{
        background: "var(--bg-1)",
        borderTop: "1px solid var(--surface-border)",
        color: "var(--text-3)",
        fontFamily: "var(--font-mono)",
        fontSize: 10,
      }}
    >
      {activeRepo ? (
        <>
          <span className="flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full" style={{ background: "var(--green)" }} />
            {activeRepo}
          </span>
          <span>Zoom: {zoomLevel}</span>
        </>
      ) : (
        <span>No repository</span>
      )}
      <span className="ml-auto" style={{ color: "var(--text-3)" }}>
        GitNexus v0.1.0
      </span>
    </div>
  );
}
