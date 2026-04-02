/**
 * Code Health Score card — circular gauge with sub-scores.
 */

import { useQuery } from "@tanstack/react-query";
import { commands } from "../../lib/tauri-commands";
import { AnimatedCard } from "../shared/motion";
import { useI18n } from "../../hooks/use-i18n";

function gradeColor(grade: string): string {
  switch (grade) {
    case "A":
      return "#9ece6a";
    case "B":
      return "#7aa2f7";
    case "C":
      return "#e0af68";
    case "D":
      return "#ff9e64";
    default:
      return "#f7768e";
  }
}

function ScoreBar({
  label,
  value,
  color,
}: {
  label: string;
  value: number;
  color: string;
}) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 11 }}>
      <span
        style={{
          width: 80,
          color: "var(--text-2)",
          flexShrink: 0,
        }}
      >
        {label}
      </span>
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
            width: `${value * 100}%`,
            height: "100%",
            borderRadius: 3,
            background: color,
            transition: "width 0.5s ease",
          }}
        />
      </div>
      <span
        style={{
          width: 32,
          textAlign: "right",
          color: "var(--text-2)",
          fontVariantNumeric: "tabular-nums",
        }}
      >
        {Math.round(value * 100)}%
      </span>
    </div>
  );
}

export function CodeHealthCard() {
  const { t } = useI18n();
  const { data: health } = useQuery({
    queryKey: ["code-health"],
    queryFn: () => commands.getCodeHealth(),
    staleTime: 60_000,
  });

  if (!health) return null;

  const color = gradeColor(health.grade);

  // SVG arc for circular gauge
  const radius = 36;
  const circumference = 2 * Math.PI * radius;
  const progress = (health.overallScore / 100) * circumference;

  return (
    <AnimatedCard>
      <div
        style={{
          padding: "16px 20px",
          borderRadius: "var(--radius-lg)",
          border: "1px solid var(--border)",
          background: "var(--bg-1)",
          display: "flex",
          gap: 20,
          alignItems: "center",
        }}
      >
        {/* Circular gauge */}
        <div style={{ position: "relative", width: 84, height: 84, flexShrink: 0 }}>
          <svg width={84} height={84} style={{ transform: "rotate(-90deg)" }}>
            <circle
              cx={42}
              cy={42}
              r={radius}
              fill="none"
              stroke="var(--bg-3)"
              strokeWidth={6}
            />
            <circle
              cx={42}
              cy={42}
              r={radius}
              fill="none"
              stroke={color}
              strokeWidth={6}
              strokeDasharray={circumference}
              strokeDashoffset={circumference - progress}
              strokeLinecap="round"
              style={{ transition: "stroke-dashoffset 0.8s ease" }}
            />
          </svg>
          <div
            style={{
              position: "absolute",
              inset: 0,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <span
              style={{
                fontSize: 22,
                fontWeight: 700,
                color,
                lineHeight: 1,
              }}
            >
              {health.grade}
            </span>
            <span
              style={{
                fontSize: 10,
                color: "var(--text-3)",
              }}
            >
              {Math.round(health.overallScore)}/100
            </span>
          </div>
        </div>

        {/* Sub-scores */}
        <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 6 }}>
          <div
            style={{
              fontSize: 13,
              fontWeight: 600,
              color: "var(--text-0)",
              marginBottom: 4,
            }}
          >
            {t("health.title")}
          </div>
          <ScoreBar label={t("health.hotspots")} value={health.hotspotScore} color="#9ece6a" />
          <ScoreBar label={t("health.cohesion")} value={health.cohesionScore} color="#7aa2f7" />
          <ScoreBar label={t("health.tracing")} value={health.tracingCoverage} color="#73daca" />
          <ScoreBar label={t("health.ownership")} value={health.ownershipScore} color="#bb9af7" />
          <ScoreBar label={t("health.complexity")} value={Math.min(1 - (health.avgComplexity / 30), 1)} color="#f59e0b" />
        </div>

        {/* Complexity detail */}
        {health.maxComplexity > 0 && (
          <div style={{ position: "relative", width: 84, flexShrink: 0, display: "flex", alignItems: "center", justifyContent: "center" }}>
            <div style={{ fontSize: 10, color: "var(--text-3)", textAlign: "center" }}>
              Max CC: {health.maxComplexity} · Avg: {health.avgComplexity.toFixed(1)}
            </div>
          </div>
        )}
      </div>
    </AnimatedCard>
  );
}
