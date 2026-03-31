/**
 * Code Ownership view — author distribution per file.
 */

import { useMemo } from "react";
import type { GitOwnership } from "../../lib/tauri-commands";
import { LoadingOrbs } from "../shared/LoadingOrbs";

interface Props {
  data: GitOwnership[];
  loading: boolean;
}

const AUTHOR_COLORS = [
  "#7aa2f7",
  "#9ece6a",
  "#bb9af7",
  "#e0af68",
  "#f7768e",
  "#7dcfff",
  "#73daca",
  "#ff9e64",
];

export function OwnershipView({ data, loading }: Props) {
  if (loading) return <LoadingOrbs label="Analyzing ownership..." />;

  if (data.length === 0) {
    return (
      <div
        style={{
          padding: 40,
          textAlign: "center",
          color: "var(--text-3)",
          fontSize: 13,
        }}
      >
        No ownership data available.
      </div>
    );
  }

  // Author summary
  const authorSummary = useMemo(() => {
    const map = new Map<string, number>();
    for (const o of data) {
      map.set(o.primaryAuthor, (map.get(o.primaryAuthor) || 0) + 1);
    }
    return Array.from(map.entries())
      .sort((a, b) => b[1] - a[1]);
  }, [data]);

  const orphans = data.filter((o) => o.ownershipPct < 50);

  return (
    <div style={{ padding: 16 }}>
      {/* Author summary */}
      <div style={{ marginBottom: 20 }}>
        <div
          style={{
            fontSize: 12,
            fontWeight: 600,
            color: "var(--text-1)",
            marginBottom: 8,
          }}
        >
          Authors ({authorSummary.length})
        </div>
        <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
          {authorSummary.slice(0, 10).map(([author, count], i) => (
            <div
              key={author}
              style={{
                padding: "4px 10px",
                borderRadius: 6,
                border: "1px solid var(--border)",
                background: "var(--bg-1)",
                fontSize: 11,
                display: "flex",
                alignItems: "center",
                gap: 6,
              }}
            >
              <span
                style={{
                  width: 8,
                  height: 8,
                  borderRadius: "50%",
                  background: AUTHOR_COLORS[i % AUTHOR_COLORS.length],
                }}
              />
              <span style={{ color: "var(--text-1)" }}>{author}</span>
              <span style={{ color: "var(--text-3)", fontSize: 10 }}>
                {count} files
              </span>
            </div>
          ))}
        </div>
      </div>

      {/* Orphan warning */}
      {orphans.length > 0 && (
        <div
          style={{
            padding: "8px 12px",
            borderRadius: 8,
            border: "1px solid rgba(247,118,142,0.3)",
            background: "rgba(247,118,142,0.05)",
            marginBottom: 16,
            fontSize: 12,
            color: "#f7768e",
          }}
        >
          {orphans.length} files with no clear owner (&lt;50% ownership)
        </div>
      )}

      {/* Files table — sorted by author_count desc */}
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
        <thead>
          <tr
            style={{
              borderBottom: "1px solid var(--border)",
              color: "var(--text-2)",
              textAlign: "left",
            }}
          >
            <th style={{ padding: "6px 8px", fontWeight: 600 }}>File</th>
            <th style={{ padding: "6px 8px", fontWeight: 600 }}>Primary Author</th>
            <th style={{ padding: "6px 8px", fontWeight: 600, width: 120 }}>Ownership</th>
            <th style={{ padding: "6px 8px", fontWeight: 600, textAlign: "right" }}>Authors</th>
          </tr>
        </thead>
        <tbody>
          {data.slice(0, 30).map((o) => (
            <tr
              key={o.path}
              style={{
                borderBottom: "1px solid var(--border)",
                color: "var(--text-1)",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = "var(--bg-2)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "transparent";
              }}
            >
              <td
                style={{
                  padding: "6px 8px",
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  maxWidth: 300,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {o.path.replace(/\\/g, "/")}
              </td>
              <td style={{ padding: "6px 8px" }}>{o.primaryAuthor}</td>
              <td style={{ padding: "6px 8px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  {/* Stacked bar of authors */}
                  <div
                    style={{
                      flex: 1,
                      height: 8,
                      borderRadius: 4,
                      background: "var(--bg-3)",
                      overflow: "hidden",
                      display: "flex",
                    }}
                  >
                    {o.authors.slice(0, 4).map((a, j) => (
                      <div
                        key={a.name}
                        title={`${a.name}: ${Math.round(a.pct)}%`}
                        style={{
                          width: `${a.pct}%`,
                          height: "100%",
                          background: AUTHOR_COLORS[j % AUTHOR_COLORS.length],
                        }}
                      />
                    ))}
                  </div>
                  <span
                    style={{
                      fontSize: 10,
                      color: o.ownershipPct < 50 ? "#f7768e" : "var(--text-2)",
                      minWidth: 32,
                      textAlign: "right",
                    }}
                  >
                    {Math.round(o.ownershipPct)}%
                  </span>
                </div>
              </td>
              <td style={{ padding: "6px 8px", textAlign: "right" }}>
                {o.authorCount}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
