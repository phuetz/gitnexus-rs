/**
 * Temporal Coupling view — file pairs that change together.
 */

import type { GitCoupling } from "../../lib/tauri-commands";
import { LoadingOrbs } from "../shared/LoadingOrbs";

interface Props {
  data: GitCoupling[];
  loading: boolean;
}

function strengthColor(s: number): string {
  if (s >= 0.7) return "#f7768e";
  if (s >= 0.4) return "#e0af68";
  return "#9ece6a";
}

export function CouplingView({ data, loading }: Props) {
  if (loading) return <LoadingOrbs label="Analyzing coupling..." />;

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
        No temporal coupling detected. Files change independently.
      </div>
    );
  }

  const strong = data.filter((c) => c.couplingStrength > 0.7).length;

  return (
    <div style={{ padding: 16 }}>
      <div
        style={{
          fontSize: 11,
          color: "var(--text-3)",
          marginBottom: 12,
          display: "flex",
          gap: 16,
        }}
      >
        <span>{data.length} coupled pairs detected</span>
        {strong > 0 && (
          <span style={{ color: "#f7768e" }}>
            {strong} strongly coupled (&gt;70%)
          </span>
        )}
      </div>

      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
        <thead>
          <tr
            style={{
              borderBottom: "1px solid var(--border)",
              color: "var(--text-2)",
              textAlign: "left",
            }}
          >
            <th style={{ padding: "6px 8px", fontWeight: 600 }}>#</th>
            <th style={{ padding: "6px 8px", fontWeight: 600 }}>File A</th>
            <th style={{ padding: "6px 8px", fontWeight: 600 }}>File B</th>
            <th style={{ padding: "6px 8px", fontWeight: 600, textAlign: "right" }}>Shared</th>
            <th style={{ padding: "6px 8px", fontWeight: 600, width: 120 }}>Strength</th>
          </tr>
        </thead>
        <tbody>
          {data.slice(0, 30).map((c, i) => (
            <tr
              key={`${c.fileA}-${c.fileB}`}
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
              <td style={{ padding: "6px 8px", color: "var(--text-3)" }}>
                {i + 1}
              </td>
              <td
                style={{
                  padding: "6px 8px",
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  maxWidth: 250,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {c.fileA.replace(/\\/g, "/")}
              </td>
              <td
                style={{
                  padding: "6px 8px",
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  maxWidth: 250,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {c.fileB.replace(/\\/g, "/")}
              </td>
              <td style={{ padding: "6px 8px", textAlign: "right" }}>
                {c.sharedCommits}
              </td>
              <td style={{ padding: "6px 8px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  <div
                    style={{
                      flex: 1,
                      height: 6,
                      borderRadius: 3,
                      background: "var(--bg-3)",
                      overflow: "hidden",
                    }}
                  >
                    <div
                      style={{
                        width: `${c.couplingStrength * 100}%`,
                        height: "100%",
                        borderRadius: 3,
                        background: strengthColor(c.couplingStrength),
                        transition: "width 0.3s ease",
                      }}
                    />
                  </div>
                  <span
                    style={{
                      fontSize: 10,
                      fontWeight: 600,
                      color: strengthColor(c.couplingStrength),
                      minWidth: 32,
                      textAlign: "right",
                    }}
                  >
                    {Math.round(c.couplingStrength * 100)}%
                  </span>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
