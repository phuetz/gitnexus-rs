import { useState, useEffect, useCallback } from "react";
import {
  Download,
  FileText,
  Server,
  Route,
  Globe,
  Layout,
  Table2,
  Database,
  Layers,
  RefreshCw,
  CheckCircle,
  AlertCircle,
  FolderOpen,
} from "lucide-react";
import { commands, type AspNetStats } from "../../lib/tauri-commands";
import { useAppStore } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";

interface StatCardProps {
  icon: React.ElementType;
  label: string;
  value: number;
  color: string;
}

function StatCard({ icon: Icon, label, value, color }: StatCardProps) {
  return (
    <div
      className="flex items-center gap-3 rounded-lg"
      style={{
        padding: "12px 14px",
        background: "var(--surface-0)",
        border: "1px solid var(--surface-border)",
      }}
    >
      <div
        className="flex items-center justify-center rounded-md shrink-0"
        style={{
          width: 36,
          height: 36,
          background: `${color}15`,
          color,
        }}
      >
        <Icon size={18} />
      </div>
      <div className="flex flex-col min-w-0">
        <span
          className="text-[22px] font-bold leading-tight"
          style={{ color: "var(--text-0)", fontFamily: "var(--font-display)" }}
        >
          {value}
        </span>
        <span
          className="text-[11px] truncate"
          style={{ color: "var(--text-3)" }}
        >
          {label}
        </span>
      </div>
    </div>
  );
}

type ExportStatus = "idle" | "exporting" | "success" | "error";

export function ExportPanel() {
  const { t } = useI18n();
  const activeRepo = useAppStore((s) => s.activeRepo);

  const [stats, setStats] = useState<AspNetStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [exportStatus, setExportStatus] = useState<ExportStatus>("idle");
  const [exportPath, setExportPath] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const loadStats = useCallback(async () => {
    if (!activeRepo) {
      setLoading(false);
      setStats(null);
      return;
    }
    setLoading(true);
    try {
      const result = await commands.getAspnetStats();
      setStats(result);
    } catch {
      setStats(null);
    } finally {
      setLoading(false);
    }
  }, [activeRepo]);

  useEffect(() => {
    loadStats();
  }, [loadStats]);

  /* ── No-repo guard ── */
  if (!activeRepo) {
    return (
      <div
        className="h-full flex flex-col items-center justify-center text-center"
        style={{ background: "var(--bg-0)", padding: "48px 24px" }}
      >
        <FolderOpen
          size={48}
          style={{ color: "var(--text-3)", opacity: 0.35, marginBottom: 16 }}
        />
        <h3
          className="text-base font-semibold"
          style={{
            color: "var(--text-1)",
            margin: "0 0 8px 0",
            fontFamily: "var(--font-display)",
          }}
        >
          {t("status.noRepo")}
        </h3>
        <p
          className="text-xs"
          style={{ color: "var(--text-3)", margin: 0, maxWidth: 340, lineHeight: 1.6 }}
        >
          {t("export.noRepoDesc")}
        </p>
      </div>
    );
  }

  const handleExport = async () => {
    setExportStatus("exporting");
    setErrorMsg(null);
    try {
      const path = await commands.exportDocsDocx();
      setExportPath(path);
      setExportStatus("success");
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setExportStatus("error");
    }
  };

  const hasAspNet =
    stats != null &&
    (stats.controllers > 0 ||
      stats.entities > 0 ||
      stats.views > 0 ||
      stats.dbContexts > 0);

  const totalItems =
    stats != null
      ? stats.controllers +
        stats.actions +
        stats.apiEndpoints +
        stats.views +
        stats.entities +
        stats.dbContexts +
        stats.areas
      : 0;

  return (
    <div
      className="h-full overflow-y-auto"
      style={{ background: "var(--bg-0)" }}
    >
      <div style={{ maxWidth: 640, margin: "0 auto", padding: "32px 24px" }}>
        {/* Header */}
        <div className="flex items-center justify-between" style={{ marginBottom: 24 }}>
          <div>
            <h2
              className="text-lg font-bold"
              style={{
                color: "var(--text-0)",
                fontFamily: "var(--font-display)",
                margin: 0,
              }}
            >
              {t("export.title")}
            </h2>
            <p
              className="text-xs"
              style={{ color: "var(--text-3)", marginTop: 4 }}
            >
              {t("export.subtitle")}
            </p>
          </div>
          <button
            onClick={loadStats}
            title={t("export.refreshStats")}
            aria-label={t("export.refreshStats")}
            className="rounded-md transition-colors"
            style={{
              padding: 8,
              color: "var(--text-3)",
              background: "transparent",
              border: "none",
              cursor: "pointer",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "var(--surface-hover)";
              e.currentTarget.style.color = "var(--text-1)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "transparent";
              e.currentTarget.style.color = "var(--text-3)";
            }}
          >
            <RefreshCw size={16} className={loading ? "animate-spin" : ""} />
          </button>
        </div>

        {/* DOCX Export Card */}
        <div
          className="rounded-xl"
          style={{
            padding: "20px",
            marginBottom: 28,
            background: "var(--surface-0)",
            border: "1px solid var(--surface-border)",
          }}
        >
          <div className="flex items-start gap-4">
            <div
              className="flex items-center justify-center rounded-lg shrink-0"
              style={{
                width: 48,
                height: 48,
                background: "linear-gradient(135deg, var(--accent), #8b5cf6)",
              }}
            >
              <FileText size={24} color="white" />
            </div>
            <div className="flex-1 min-w-0">
              <h3
                className="text-sm font-semibold"
                style={{
                  color: "var(--text-0)",
                  margin: "0 0 4px 0",
                  fontFamily: "var(--font-display)",
                }}
              >
                {t("export.docxTitle")}
              </h3>
              <p
                className="text-xs"
                style={{ color: "var(--text-3)", margin: 0, lineHeight: 1.5 }}
              >
                {t("export.docxDesc")}
              </p>
            </div>
          </div>

          <button
            onClick={handleExport}
            disabled={exportStatus === "exporting"}
            aria-label={t("export.generateDocx")}
            className="w-full flex items-center justify-center gap-2 rounded-lg text-sm font-medium transition-all"
            style={{
              marginTop: 16,
              padding: "10px 16px",
              background:
                exportStatus === "exporting"
                  ? "var(--surface-1)"
                  : "linear-gradient(135deg, var(--accent), #8b5cf6)",
              color: exportStatus === "exporting" ? "var(--text-3)" : "white",
              border: "none",
              cursor: exportStatus === "exporting" ? "wait" : "pointer",
              opacity: exportStatus === "exporting" ? 0.7 : 1,
            }}
          >
            {exportStatus === "exporting" ? (
              <>
                <RefreshCw size={16} className="animate-spin" />
                {t("export.exporting")}
              </>
            ) : (
              <>
                <Download size={16} />
                {t("export.generateDocx")}
              </>
            )}
          </button>

          {/* Success message */}
          {exportStatus === "success" && exportPath && (
            <div
              className="flex items-start gap-2 rounded-lg"
              style={{
                marginTop: 12,
                padding: "10px 12px",
                background: "rgba(52, 211, 153, 0.1)",
                color: "#34d399",
                fontSize: 12,
              }}
            >
              <CheckCircle size={16} className="shrink-0" style={{ marginTop: 1 }} />
              <div className="min-w-0">
                <div className="font-medium">{t("export.success")}</div>
                <div
                  className="truncate"
                  style={{ color: "var(--text-3)", marginTop: 2 }}
                  title={exportPath}
                >
                  {exportPath}
                </div>
              </div>
            </div>
          )}

          {/* Error message */}
          {exportStatus === "error" && errorMsg && (
            <div
              className="flex items-start gap-2 rounded-lg"
              style={{
                marginTop: 12,
                padding: "10px 12px",
                background: "rgba(251, 113, 133, 0.1)",
                color: "#fb7185",
                fontSize: 12,
              }}
            >
              <AlertCircle size={16} className="shrink-0" style={{ marginTop: 1 }} />
              <div className="min-w-0">
                <div className="font-medium">{t("export.error")}</div>
                <div style={{ color: "var(--text-3)", marginTop: 2 }}>{errorMsg}</div>
              </div>
            </div>
          )}
        </div>

        {/* ASP.NET Stats */}
        {loading ? (
          <div
            className="flex items-center justify-center"
            style={{ padding: 40, color: "var(--text-3)" }}
          >
            <RefreshCw size={20} className="animate-spin" />
            <span className="text-sm" style={{ marginLeft: 8 }}>
              {t("export.loading")}
            </span>
          </div>
        ) : hasAspNet ? (
          <>
            <div className="flex items-center gap-2" style={{ marginBottom: 16 }}>
              <h3
                className="text-sm font-semibold"
                style={{
                  color: "var(--text-0)",
                  margin: 0,
                  fontFamily: "var(--font-display)",
                }}
              >
                {t("export.statsTitle")}
              </h3>
              <span
                className="text-[10px] font-medium rounded-full"
                style={{
                  padding: "2px 8px",
                  background: "rgba(52, 211, 153, 0.15)",
                  color: "#34d399",
                }}
              >
                {totalItems} {t("export.elements")}
              </span>
            </div>

            <div
              className="grid gap-3"
              style={{ gridTemplateColumns: "repeat(2, 1fr)" }}
            >
              <StatCard
                icon={Server}
                label={t("export.controllers")}
                value={stats!.controllers}
                color="#818cf8"
              />
              <StatCard
                icon={Route}
                label={t("export.actions")}
                value={stats!.actions}
                color="#67e8f9"
              />
              <StatCard
                icon={Globe}
                label={t("export.apiEndpoints")}
                value={stats!.apiEndpoints}
                color="#34d399"
              />
              <StatCard
                icon={Layout}
                label={t("export.razorViews")}
                value={stats!.views}
                color="#f472b6"
              />
              <StatCard
                icon={Table2}
                label={t("export.efEntities")}
                value={stats!.entities}
                color="#fb923c"
              />
              <StatCard
                icon={Database}
                label={t("export.dbContexts")}
                value={stats!.dbContexts}
                color="#fbbf24"
              />
              <StatCard
                icon={Layers}
                label={t("export.areas")}
                value={stats!.areas}
                color="#94a3b8"
              />
            </div>
          </>
        ) : (
          <div
            className="flex flex-col items-center justify-center text-center rounded-xl"
            style={{
              padding: "40px 24px",
              background: "var(--surface-0)",
              border: "1px solid var(--surface-border)",
            }}
          >
            <Server
              size={40}
              style={{ color: "var(--text-3)", marginBottom: 12, opacity: 0.4 }}
            />
            <p
              className="text-sm font-medium"
              style={{ color: "var(--text-2)", margin: "0 0 4px 0" }}
            >
              {t("export.noAspnet")}
            </p>
            <p
              className="text-xs"
              style={{ color: "var(--text-3)", margin: 0, maxWidth: 320 }}
            >
              {t("export.noAspnetDesc")}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
