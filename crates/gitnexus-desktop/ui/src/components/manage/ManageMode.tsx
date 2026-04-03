import React from "react";
import { Database, Download, BookOpen, Settings, Globe, Sun, Moon, Monitor } from "lucide-react";
import { useAppStore, type ThemeMode } from "../../stores/app-store";
import { useI18n, type Locale } from "../../hooks/use-i18n";
import { RepoManager } from "../repos/RepoManager";
import { ExportPanel } from "../export/ExportPanel";
import { DocsViewer } from "../docs/DocsViewer";

// ─── Section wrapper ─────────────────────────────────────────────────

function Section({ icon: Icon, title, children }: { icon: typeof Database; title: string; children: React.ReactNode }) {
  return (
    <section
      className="rounded-xl p-5 mb-4"
      style={{
        background: "var(--glass-bg)",
        backdropFilter: "blur(var(--glass-blur))",
        border: "1px solid var(--glass-border)",
      }}
    >
      <h2 className="flex items-center gap-2 mb-4" style={{ fontFamily: "var(--font-display)", fontSize: 16, fontWeight: 600, color: "var(--text-0)" }}>
        <Icon size={18} style={{ color: "var(--accent)" }} />
        {title}
      </h2>
      {children}
    </section>
  );
}

// ─── Settings content (extracted from SettingsModal) ─────────────────

const LANGUAGES: { code: Locale; label: string; flag: string }[] = [
  { code: "fr", label: "Français", flag: "FR" },
  { code: "en", label: "English", flag: "EN" },
];

const THEMES: { mode: ThemeMode; icon: typeof Sun; label: string; labelFr: string }[] = [
  { mode: "dark", icon: Moon, label: "Dark", labelFr: "Sombre" },
  { mode: "light", icon: Sun, label: "Light", labelFr: "Clair" },
  { mode: "system", icon: Monitor, label: "System", labelFr: "Système" },
];

function SettingsContent() {
  const { locale, setLocale, tt } = useI18n();
  const theme = useAppStore((s) => s.theme);
  const setTheme = useAppStore((s) => s.setTheme);

  const langEntry = tt("settings.language");

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      {/* Language selector */}
      <div
        className="rounded-lg"
        style={{
          background: "var(--bg-1)",
          border: "1px solid var(--surface-border)",
          padding: "16px 20px",
        }}
      >
        <div className="flex items-center" style={{ gap: 10, marginBottom: 12 }}>
          <Globe size={16} style={{ color: "var(--accent)" }} />
          <div>
            <p className="text-sm font-medium" style={{ color: "var(--text-0)" }}>
              {langEntry.label}
            </p>
            {langEntry.tip && (
              <p className="text-[11px]" style={{ color: "var(--text-3)", marginTop: 2 }}>
                {langEntry.tip}
              </p>
            )}
          </div>
        </div>
        <div className="flex" style={{ gap: 8 }}>
          {LANGUAGES.map((lang) => {
            const isActive = locale === lang.code;
            return (
              <button
                key={lang.code}
                onClick={() => setLocale(lang.code)}
                className="rounded-md text-xs font-medium"
                style={{
                  padding: "8px 16px",
                  background: isActive ? "var(--accent)" : "var(--bg-3)",
                  color: isActive ? "#fff" : "var(--text-2)",
                  border: isActive ? "1px solid var(--accent)" : "1px solid var(--surface-border)",
                  cursor: "pointer",
                  flex: 1,
                }}
              >
                {lang.flag}  {lang.label}
              </button>
            );
          })}
        </div>
      </div>

      {/* Theme selector */}
      <div
        className="rounded-lg"
        style={{
          background: "var(--bg-1)",
          border: "1px solid var(--surface-border)",
          padding: "16px 20px",
        }}
      >
        <div className="flex items-center" style={{ gap: 10, marginBottom: 12 }}>
          <Sun size={16} style={{ color: "var(--amber)" }} />
          <p className="text-sm font-medium" style={{ color: "var(--text-0)" }}>
            {locale === "fr" ? "Thème" : "Theme"}
          </p>
        </div>
        <div className="flex" style={{ gap: 8 }}>
          {THEMES.map((t) => {
            const isActive = theme === t.mode;
            const Icon = t.icon;
            return (
              <button
                key={t.mode}
                onClick={() => setTheme(t.mode)}
                className="rounded-md text-xs font-medium transition-all"
                style={{
                  padding: "8px 16px",
                  background: isActive ? "var(--accent)" : "var(--bg-3)",
                  color: isActive ? "#fff" : "var(--text-2)",
                  border: isActive ? "1px solid var(--accent)" : "1px solid var(--surface-border)",
                  cursor: "pointer",
                  flex: 1,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  gap: 6,
                }}
              >
                <Icon size={14} />
                {locale === "fr" ? t.labelFr : t.label}
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

// ─── ManageMode ───────────────────────────────────────────────────────

export function ManageMode() {
  return (
    <div className="h-full overflow-auto p-6" style={{ maxWidth: 900, margin: "0 auto" }}>
      <h1 className="mb-6" style={{ fontFamily: "var(--font-display)", fontSize: 24, fontWeight: 700, color: "var(--text-0)" }}>
        Manage
      </h1>

      <Section icon={Database} title="Repositories">
        <RepoManager />
      </Section>

      <Section icon={Download} title="Export">
        <ExportPanel />
      </Section>

      <Section icon={BookOpen} title="Documentation">
        <DocsViewer />
      </Section>

      <Section icon={Settings} title="Settings">
        <SettingsContent />
      </Section>
    </div>
  );
}
