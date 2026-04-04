import { useQuery } from "@tanstack/react-query";
import { HeartPulse } from "lucide-react";
import { commands } from "../../lib/tauri-commands";
import { useI18n } from "../../hooks/use-i18n";
import { CodeHealthCard } from "../health/CodeHealthCard";
import { LoadingOrbs } from "../shared/LoadingOrbs";

export function ReportView() {
  const { t } = useI18n();

  const { data: hotspots, isLoading: loadingHotspots } = useQuery({
    queryKey: ["hotspots-report"],
    queryFn: () => commands.getHotspots(90),
    staleTime: 60_000,
  });

  const { data: couplings, isLoading: loadingCouplings } = useQuery({
    queryKey: ["coupling-report"],
    queryFn: () => commands.getCoupling(3),
    staleTime: 60_000,
  });

  const { data: ownership, isLoading: loadingOwnership } = useQuery({
    queryKey: ["ownership-report"],
    queryFn: () => commands.getOwnership(),
    staleTime: 60_000,
  });

  const isLoading = loadingHotspots || loadingCouplings || loadingOwnership;

  return (
    <div className="h-full overflow-auto" style={{ padding: 24 }}>
      <h2 className="text-lg font-semibold" style={{ color: "var(--text-0)", marginBottom: 20 }}>
        <HeartPulse size={20} style={{ display: "inline", marginRight: 8, verticalAlign: "text-bottom" }} />
        {t("sidebar.report")}
      </h2>

      {/* Health gauge */}
      <div style={{ marginBottom: 24 }}>
        <CodeHealthCard />
      </div>

      {isLoading && (
        <div className="flex items-center justify-center py-12">
          <LoadingOrbs />
        </div>
      )}

      {/* Top hotspots */}
      {hotspots && hotspots.length > 0 && (
        <Section title={`${t("health.hotspots")} (Top 10)`}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
            <thead>
              <tr style={{ background: "var(--bg-2)" }}>
                <th style={th}>{t("report.file")}</th>
                <th style={th}>{t("report.commits")}</th>
                <th style={th}>{t("report.churn")}</th>
                <th style={th}>{t("report.score")}</th>
              </tr>
            </thead>
            <tbody>
              {hotspots.slice(0, 10).map((h) => (
                <tr key={h.path} style={{ borderTop: "1px solid var(--surface-border)" }}>
                  <td style={{ ...td, fontFamily: "var(--font-mono)", maxWidth: 300, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{h.path}</td>
                  <td style={{ ...td, textAlign: "center" }}>{h.commitCount}</td>
                  <td style={{ ...td, textAlign: "center" }}>{h.churn}</td>
                  <td style={{ ...td, textAlign: "center" }}>
                    <ScoreBadge value={h.score} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </Section>
      )}

      {/* Top couplings */}
      {couplings && couplings.length > 0 && (
        <Section title={t("report.temporalCoupling")}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
            <thead>
              <tr style={{ background: "var(--bg-2)" }}>
                <th style={th}>{t("report.fileA")}</th>
                <th style={th}>{t("report.fileB")}</th>
                <th style={th}>{t("report.shared")}</th>
                <th style={th}>{t("report.strength")}</th>
              </tr>
            </thead>
            <tbody>
              {couplings.slice(0, 10).map((c, i) => (
                <tr key={i} style={{ borderTop: "1px solid var(--surface-border)" }}>
                  <td style={{ ...td, fontFamily: "var(--font-mono)", fontSize: 11 }}>{c.fileA}</td>
                  <td style={{ ...td, fontFamily: "var(--font-mono)", fontSize: 11 }}>{c.fileB}</td>
                  <td style={{ ...td, textAlign: "center" }}>{c.sharedCommits}</td>
                  <td style={{ ...td, textAlign: "center" }}>
                    <ScoreBadge value={c.couplingStrength} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </Section>
      )}

      {/* Ownership summary */}
      {ownership && ownership.length > 0 && (
        <Section title={`${t("health.ownership")} — ${t("report.distributedFiles")}`}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
            <thead>
              <tr style={{ background: "var(--bg-2)" }}>
                <th style={th}>{t("report.file")}</th>
                <th style={th}>{t("report.primaryAuthor")}</th>
                <th style={th}>{t("report.authors")}</th>
                <th style={th}>{t("report.ownership")}</th>
              </tr>
            </thead>
            <tbody>
              {ownership.slice(0, 10).map((o) => (
                <tr key={o.path} style={{ borderTop: "1px solid var(--surface-border)" }}>
                  <td style={{ ...td, fontFamily: "var(--font-mono)", maxWidth: 250, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{o.path}</td>
                  <td style={td}>{o.primaryAuthor}</td>
                  <td style={{ ...td, textAlign: "center" }}>{o.authorCount}</td>
                  <td style={{ ...td, textAlign: "center" }}>{Math.round(o.ownershipPct)}%</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Section>
      )}
    </div>
  );
}

const th: React.CSSProperties = { padding: "8px 12px", textAlign: "left", color: "var(--text-2)", fontWeight: 500 };
const td: React.CSSProperties = { padding: "6px 12px", color: "var(--text-0)" };

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: 24 }}>
      <h3 className="text-sm font-semibold" style={{ color: "var(--text-1)", marginBottom: 10 }}>
        {title}
      </h3>
      <div
        style={{
          borderRadius: "var(--radius-lg)",
          border: "1px solid var(--surface-border)",
          overflow: "hidden",
        }}
      >
        {children}
      </div>
    </div>
  );
}

function ScoreBadge({ value }: { value: number }) {
  const pct = Math.round(value * 100);
  const color = pct >= 70 ? "#f7768e" : pct >= 40 ? "#e0af68" : "#9ece6a";
  return (
    <span
      style={{
        display: "inline-block",
        padding: "2px 8px",
        borderRadius: 99,
        fontSize: 11,
        fontWeight: 600,
        background: `${color}20`,
        color,
      }}
    >
      {pct}%
    </span>
  );
}
