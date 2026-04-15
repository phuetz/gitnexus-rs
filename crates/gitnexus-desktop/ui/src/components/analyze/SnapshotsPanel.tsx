/**
 * SnapshotsPanel — list, create, delete, and diff snapshots.
 *
 * Two columns: snapshot list on the left, diff view on the right. The user
 * picks a "from" and "to" snapshot (or "live") to compute a diff that
 * highlights what was added / removed / modified between the two states.
 *
 * Lives in the Analyze mode under a "Snapshots" sub-view; Tauri stores
 * snapshot copies in <.gitnexus>/snapshots/.
 */

import { useState } from "react";
import {
  useQuery,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
import {
  Camera,
  Trash2,
  ArrowRightLeft,
  PlusCircle,
  Loader2,
  AlertCircle,
} from "lucide-react";
import { toast } from "sonner";
import { commands } from "../../lib/tauri-commands";
import { useAppStore } from "../../stores/app-store";
import type {
  SnapshotMeta,
  SnapshotDiff,
} from "../../lib/tauri-commands";

export function SnapshotsPanel() {
  const queryClient = useQueryClient();
  const activeRepo = useAppStore((s) => s.activeRepo);
  const [from, setFrom] = useState<string>("");
  const [to, setTo] = useState<string>("live");

  const { data: snapshots = [] } = useQuery({
    queryKey: ["snapshots", activeRepo],
    queryFn: () => commands.snapshotList(),
    enabled: !!activeRepo,
    staleTime: 30_000,
  });

  // Auto-select the most-recent snapshot as "from" when the list loads.
  if (snapshots.length > 0 && from === "") {
    setFrom(snapshots[0].id);
  }

  const createMut = useMutation({
    mutationFn: (label?: string) => commands.snapshotCreate(label),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["snapshots", activeRepo] });
      toast.success("Snapshot created");
    },
    onError: (e) => toast.error(`Snapshot failed: ${(e as Error).message}`),
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => commands.snapshotDelete(id),
    onSuccess: (next) =>
      queryClient.setQueryData(["snapshots", activeRepo], next),
  });

  const diffQ = useQuery({
    queryKey: ["snapshot-diff", activeRepo, from, to],
    queryFn: () => commands.snapshotDiff({ from, to }),
    enabled: !!from && !!to && from !== to,
  });

  if (!activeRepo) {
    return (
      <div style={{ padding: 24, color: "var(--text-3)", fontSize: 12 }}>
        Open a repository to manage snapshots.
      </div>
    );
  }

  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "320px 1fr",
        gap: 16,
        padding: 16,
        height: "100%",
        overflow: "hidden",
      }}
    >
      {/* Left: list */}
      <div
        style={{
          border: "1px solid var(--surface-border)",
          borderRadius: 8,
          background: "var(--surface)",
          padding: 12,
          overflow: "auto",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            marginBottom: 10,
          }}
        >
          <Camera size={14} style={{ color: "var(--accent)" }} />
          <span style={{ fontSize: 12, fontWeight: 600 }}>Snapshots</span>
          <span style={{ fontSize: 10, color: "var(--text-3)" }}>({snapshots.length}/10)</span>
          <button
            onClick={() => {
              const label = window.prompt("Snapshot label:", "Manual snapshot");
              if (label != null) createMut.mutate(label || "Manual snapshot");
            }}
            disabled={createMut.isPending}
            title="Create a snapshot of the current graph"
            style={{
              marginLeft: "auto",
              padding: "3px 8px",
              background: "var(--accent)",
              border: "none",
              borderRadius: 4,
              color: "#fff",
              fontSize: 10,
              fontWeight: 600,
              cursor: "pointer",
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
            }}
          >
            <PlusCircle size={11} />
            New
          </button>
        </div>

        {/* "live" pseudo-row */}
        <SnapshotRow
          id="live"
          label="Current (live)"
          createdAt={Date.now()}
          nodeCount={null}
          edgeCount={null}
          isFrom={from === "live"}
          isTo={to === "live"}
          onPickFrom={() => setFrom("live")}
          onPickTo={() => setTo("live")}
        />

        {snapshots.length === 0 ? (
          <div style={{ marginTop: 8, fontSize: 11, color: "var(--text-3)" }}>
            No snapshots yet. Click "New" to record one.
          </div>
        ) : (
          snapshots.map((s) => (
            <SnapshotRow
              key={s.id}
              id={s.id}
              label={s.label}
              createdAt={s.createdAt}
              nodeCount={s.nodeCount}
              edgeCount={s.edgeCount}
              sizeBytes={s.sizeBytes}
              isFrom={from === s.id}
              isTo={to === s.id}
              onPickFrom={() => setFrom(s.id)}
              onPickTo={() => setTo(s.id)}
              onDelete={() => {
                if (window.confirm(`Delete snapshot "${s.label}"?`))
                  deleteMut.mutate(s.id);
              }}
            />
          ))
        )}
      </div>

      {/* Right: diff view */}
      <div
        style={{
          border: "1px solid var(--surface-border)",
          borderRadius: 8,
          background: "var(--surface)",
          padding: 16,
          overflow: "auto",
        }}
      >
        <DiffHeader from={from} to={to} swap={() => { const t = from; setFrom(to); setTo(t); }} />
        {!from || !to ? (
          <div style={{ marginTop: 16, color: "var(--text-3)", fontSize: 12 }}>
            Pick a "from" and "to" snapshot in the left column.
          </div>
        ) : from === to ? (
          <div style={{ marginTop: 16, color: "#e0af68", fontSize: 12, display: "flex", alignItems: "center", gap: 6 }}>
            <AlertCircle size={12} /> Pick two different snapshots.
          </div>
        ) : diffQ.isLoading ? (
          <div style={{ marginTop: 24, color: "var(--text-3)", fontSize: 12, display: "flex", alignItems: "center", gap: 6 }}>
            <Loader2 size={12} className="animate-spin" /> Computing diff…
          </div>
        ) : diffQ.error ? (
          <div style={{ marginTop: 16, color: "#f7768e", fontSize: 12 }}>
            {(diffQ.error as Error).message}
          </div>
        ) : diffQ.data ? (
          <DiffView diff={diffQ.data} />
        ) : null}
      </div>
    </div>
  );
}

// ─── Sub-components ──────────────────────────────────────────────────

function SnapshotRow({
  id,
  label,
  createdAt,
  nodeCount,
  edgeCount,
  sizeBytes,
  isFrom,
  isTo,
  onPickFrom,
  onPickTo,
  onDelete,
}: {
  id: string;
  label: string;
  createdAt: number;
  nodeCount: number | null;
  edgeCount: number | null;
  sizeBytes?: number;
  isFrom: boolean;
  isTo: boolean;
  onPickFrom: () => void;
  onPickTo: () => void;
  onDelete?: () => void;
}) {
  const isLive = id === "live";
  return (
    <div
      style={{
        padding: 8,
        marginTop: 6,
        border: `1px solid ${isFrom || isTo ? "var(--accent)" : "var(--surface-border)"}`,
        borderRadius: 6,
        background: isLive ? "rgba(122,162,247,0.06)" : "var(--bg-2)",
      }}
    >
      <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
        <span style={{ fontSize: 12, fontWeight: 600 }}>{label}</span>
        {isLive && (
          <span
            style={{
              fontSize: 8,
              fontWeight: 700,
              textTransform: "uppercase",
              padding: "1px 4px",
              borderRadius: 3,
              background: "rgba(158,206,106,0.15)",
              color: "#9ece6a",
            }}
          >
            live
          </span>
        )}
        {onDelete && (
          <button
            onClick={onDelete}
            style={{
              marginLeft: "auto",
              padding: 2,
              background: "transparent",
              border: "none",
              color: "var(--text-3)",
              cursor: "pointer",
            }}
            aria-label="Delete snapshot"
            title="Delete snapshot"
          >
            <Trash2 size={10} />
          </button>
        )}
      </div>
      <div style={{ fontSize: 10, color: "var(--text-3)", marginTop: 2 }}>
        {isLive
          ? "in-memory current state"
          : `${formatDate(createdAt)} · ${nodeCount ?? "?"} nodes · ${edgeCount ?? "?"} edges${
              sizeBytes ? ` · ${formatSize(sizeBytes)}` : ""
            }`}
      </div>
      <div style={{ marginTop: 6, display: "flex", gap: 4 }}>
        <button
          onClick={onPickFrom}
          style={pickBtnStyle(isFrom, "#7aa2f7")}
        >
          From
        </button>
        <button
          onClick={onPickTo}
          style={pickBtnStyle(isTo, "#9ece6a")}
        >
          To
        </button>
      </div>
    </div>
  );
}

function pickBtnStyle(active: boolean, color: string): React.CSSProperties {
  return {
    flex: 1,
    padding: "3px 6px",
    background: active ? color : "transparent",
    border: `1px solid ${active ? color : "var(--surface-border)"}`,
    borderRadius: 4,
    color: active ? "#fff" : "var(--text-3)",
    cursor: "pointer",
    fontSize: 10,
    fontWeight: 600,
    fontFamily: "inherit",
  };
}

function DiffHeader({
  from,
  to,
  swap,
}: {
  from: string;
  to: string;
  swap: () => void;
}) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
      <ArrowRightLeft size={14} style={{ color: "var(--accent)" }} />
      <span style={{ fontSize: 12, fontWeight: 600 }}>Diff</span>
      <code style={{ fontSize: 11, color: "var(--text-2)" }}>{from || "?"}</code>
      <span style={{ color: "var(--text-3)" }}>→</span>
      <code style={{ fontSize: 11, color: "var(--text-2)" }}>{to || "?"}</code>
      <button
        onClick={swap}
        title="Swap from/to"
        aria-label="Swap from and to"
        style={{
          padding: "2px 6px",
          background: "transparent",
          border: "1px solid var(--surface-border)",
          borderRadius: 4,
          color: "var(--text-3)",
          cursor: "pointer",
          fontSize: 10,
        }}
      >
        ⇄
      </button>
    </div>
  );
}

function DiffView({ diff }: { diff: SnapshotDiff }) {
  return (
    <div style={{ marginTop: 12 }}>
      {/* Headline counts */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(140px, 1fr))",
          gap: 8,
          marginBottom: 16,
        }}
      >
        <CountCard label="Added" value={diff.totalAdded} color="#9ece6a" />
        <CountCard label="Removed" value={diff.totalRemoved} color="#f7768e" />
        <CountCard label="Modified" value={diff.totalModified} color="#e0af68" />
        <CountCard
          label="Net nodes"
          value={diff.toNodeCount - diff.fromNodeCount}
          color="var(--accent)"
        />
        <CountCard
          label="Net edges"
          value={diff.toEdgeCount - diff.fromEdgeCount}
          color="var(--accent)"
        />
      </div>

      {/* Per-label breakdown */}
      {diff.byLabel.length > 0 && (
        <SectionTitle>By node label</SectionTitle>
      )}
      {diff.byLabel.length > 0 && (
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11, marginBottom: 14 }}>
          <thead>
            <tr style={{ color: "var(--text-3)", fontSize: 10, textAlign: "left" }}>
              <th style={th()}>Label</th>
              <th style={th("right")}>From</th>
              <th style={th("right")}>To</th>
              <th style={th("right")}>+ Added</th>
              <th style={th("right")}>− Removed</th>
            </tr>
          </thead>
          <tbody>
            {diff.byLabel.slice(0, 20).map((d) => (
              <tr key={d.label}>
                <td style={td()}>{d.label}</td>
                <td style={td("right")}>{d.fromCount}</td>
                <td style={td("right")}>{d.toCount}</td>
                <td style={{ ...td("right"), color: d.added > 0 ? "#9ece6a" : "var(--text-3)" }}>
                  {d.added > 0 ? `+${d.added}` : "—"}
                </td>
                <td style={{ ...td("right"), color: d.removed > 0 ? "#f7768e" : "var(--text-3)" }}>
                  {d.removed > 0 ? `−${d.removed}` : "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {/* Sample lists */}
      {diff.addedSample.length > 0 && (
        <NodeSampleList title={`Added (${diff.totalAdded})`} color="#9ece6a" nodes={diff.addedSample} />
      )}
      {diff.removedSample.length > 0 && (
        <NodeSampleList title={`Removed (${diff.totalRemoved})`} color="#f7768e" nodes={diff.removedSample} />
      )}
      {diff.modifiedSample.length > 0 && <ModifiedList nodes={diff.modifiedSample} total={diff.totalModified} />}

      {diff.totalAdded === 0 && diff.totalRemoved === 0 && diff.totalModified === 0 && (
        <div style={{ color: "var(--text-3)", fontSize: 11 }}>
          The two snapshots are structurally identical.
        </div>
      )}
    </div>
  );
}

function CountCard({
  label,
  value,
  color,
}: {
  label: string;
  value: number;
  color: string;
}) {
  const sign = value > 0 ? "+" : "";
  return (
    <div
      style={{
        padding: "8px 12px",
        background: "var(--bg-2)",
        border: `1px solid var(--surface-border)`,
        borderLeft: `3px solid ${color}`,
        borderRadius: 6,
      }}
    >
      <div style={{ fontSize: 9, fontWeight: 700, textTransform: "uppercase", color: "var(--text-3)" }}>
        {label}
      </div>
      <div
        style={{
          fontSize: 18,
          fontWeight: 700,
          fontFamily: "var(--font-display)",
          color,
        }}
      >
        {label.startsWith("Net") && value !== 0 ? `${sign}${value}` : value.toLocaleString()}
      </div>
    </div>
  );
}

function NodeSampleList({
  title,
  color,
  nodes,
}: {
  title: string;
  color: string;
  nodes: { id: string; name: string; label: string; filePath: string }[];
}) {
  return (
    <div style={{ marginBottom: 14 }}>
      <SectionTitle accent={color}>{title}</SectionTitle>
      <ul style={{ margin: 0, paddingLeft: 16, fontSize: 11 }}>
        {nodes.map((n) => (
          <li key={n.id} style={{ color: "var(--text-2)" }}>
            <code>{n.name}</code>{" "}
            <span style={{ color: "var(--text-3)" }}>
              ({n.label}{n.filePath ? ` · ${n.filePath}` : ""})
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}

function ModifiedList({
  nodes,
  total,
}: {
  nodes: { id: string; name: string; label: string; filePath: string; changes: string[] }[];
  total: number;
}) {
  return (
    <div style={{ marginBottom: 14 }}>
      <SectionTitle accent="#e0af68">Modified ({total})</SectionTitle>
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {nodes.map((n) => (
          <div
            key={n.id}
            style={{
              padding: "5px 8px",
              border: "1px solid var(--surface-border)",
              borderLeft: "3px solid #e0af68",
              borderRadius: 4,
              fontSize: 11,
              background: "var(--bg-2)",
            }}
          >
            <div>
              <code style={{ fontWeight: 600 }}>{n.name}</code>{" "}
              <span style={{ color: "var(--text-3)" }}>
                ({n.label}{n.filePath ? ` · ${n.filePath}` : ""})
              </span>
            </div>
            <ul style={{ margin: "2px 0 0 16px", color: "var(--text-3)", fontSize: 10 }}>
              {n.changes.map((c, i) => (
                <li key={i}>{c}</li>
              ))}
            </ul>
          </div>
        ))}
      </div>
    </div>
  );
}

function SectionTitle({
  children,
  accent,
}: {
  children: React.ReactNode;
  accent?: string;
}) {
  return (
    <div
      style={{
        fontSize: 10,
        fontWeight: 700,
        textTransform: "uppercase",
        color: accent ?? "var(--text-3)",
        marginBottom: 6,
        letterSpacing: 0.5,
      }}
    >
      {children}
    </div>
  );
}

function th(align: "left" | "right" = "left"): React.CSSProperties {
  return { padding: "4px 8px", textAlign: align, fontWeight: 600, borderBottom: "1px solid var(--surface-border)" };
}
function td(align: "left" | "right" = "left"): React.CSSProperties {
  return {
    padding: "4px 8px",
    textAlign: align,
    color: "var(--text-1)",
    fontFamily: align === "right" ? "var(--font-mono)" : "inherit",
    borderBottom: "1px solid var(--surface-border)",
  };
}

function formatDate(ms: number) {
  const d = new Date(ms);
  return d.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

