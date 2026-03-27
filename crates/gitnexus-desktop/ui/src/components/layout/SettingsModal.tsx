import { X, Info, Globe } from "lucide-react";
import { useAppStore } from "../../stores/app-store";
import { useI18n, type Locale } from "../../hooks/use-i18n";

const LANGUAGES: { code: Locale; label: string; flag: string }[] = [
  { code: "fr", label: "Français", flag: "FR" },
  { code: "en", label: "English", flag: "EN" },
];

export function SettingsModal() {
  const settingsOpen = useAppStore((s) => s.settingsOpen);
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen);
  const { t, tt, locale, setLocale } = useI18n();

  if (!settingsOpen) return null;

  const langEntry = tt("settings.language");

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ backgroundColor: "rgba(0, 0, 0, 0.6)", backdropFilter: "blur(4px)" }}
      onClick={(e) => {
        if (e.target === e.currentTarget) setSettingsOpen(false);
      }}
    >
      <div
        className="rounded-xl shadow-2xl overflow-hidden"
        style={{
          width: 520,
          maxHeight: "80vh",
          background: "var(--bg-0)",
          border: "1px solid var(--surface-border)",
        }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between"
          style={{ borderBottom: "1px solid var(--surface-border)", background: "var(--bg-1)", padding: "16px 24px" }}
        >
          <h2
            className="text-base font-semibold"
            style={{ color: "var(--text-0)", fontFamily: "var(--font-display)" }}
          >
            {t("settings.title")}
          </h2>
          <button
            onClick={() => setSettingsOpen(false)}
            className="rounded-md transition-colors"
            style={{ color: "var(--text-3)", padding: 6 }}
            title={t("search.close")}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "var(--surface-hover)";
              e.currentTarget.style.color = "var(--text-1)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "transparent";
              e.currentTarget.style.color = "var(--text-3)";
            }}
          >
            <X size={16} />
          </button>
        </div>

        {/* Content */}
        <div style={{ padding: 24 }}>
          {/* ── Language selector (active) ── */}
          <div
            className="rounded-lg"
            style={{
              background: "var(--bg-1)",
              border: "1px solid var(--surface-border)",
              padding: "16px 20px",
              marginBottom: 20,
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
                    className="rounded-md text-xs font-medium transition-all"
                    style={{
                      padding: "8px 16px",
                      background: isActive ? "var(--accent)" : "var(--bg-3)",
                      color: isActive ? "#fff" : "var(--text-2)",
                      border: isActive ? "1px solid var(--accent)" : "1px solid var(--surface-border)",
                      cursor: "pointer",
                      flex: 1,
                    }}
                    onMouseEnter={(e) => {
                      if (!isActive) e.currentTarget.style.background = "var(--bg-4)";
                    }}
                    onMouseLeave={(e) => {
                      if (!isActive) e.currentTarget.style.background = "var(--bg-3)";
                    }}
                  >
                    {lang.flag}  {lang.label}
                  </button>
                );
              })}
            </div>
          </div>

          {/* ── Info notice ── */}
          <div
            className="flex items-start rounded-lg"
            style={{
              gap: 12,
              padding: 16,
              background: "var(--accent-subtle)",
              border: "1px solid rgba(122, 162, 247, 0.15)",
              marginBottom: 20,
            }}
          >
            <Info size={18} style={{ color: "var(--accent)", flexShrink: 0, marginTop: 1 }} />
            <div>
              <p className="text-sm font-medium" style={{ color: "var(--text-0)", marginBottom: 4 }}>
                {locale === "fr" ? "Plus de paramètres bientôt" : "More settings coming soon"}
              </p>
              <p className="text-xs" style={{ color: "var(--text-2)" }}>
                {locale === "fr"
                  ? "Nous travaillons sur les préférences de graphe, la personnalisation du thème, les raccourcis clavier et la configuration du serveur MCP."
                  : "We're working on graph layout defaults, theme customization, keyboard shortcuts, and MCP server configuration."}
              </p>
            </div>
          </div>

          {/* ── Preview of future sections ── */}
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {[
              { titleKey: "settings.theme", desc: locale === "fr" ? "Thème, taille de police, couleurs du graphe" : "Theme, font size, graph colors" },
              { titleKey: "settings.shortcuts", desc: locale === "fr" ? "Personnaliser les raccourcis clavier" : "Customize key bindings" },
              { titleKey: "", title: "MCP Server", desc: locale === "fr" ? "Transport, port, authentification" : "Transport, port, authentication" },
            ].map((section) => {
              const entry = section.titleKey ? tt(section.titleKey) : { label: section.title ?? "" };
              return (
                <div
                  key={entry.label}
                  className="flex items-center justify-between rounded-lg"
                  style={{
                    padding: "12px 16px",
                    background: "var(--bg-1)",
                    border: "1px solid var(--surface-border)",
                    opacity: 0.5,
                  }}
                >
                  <div>
                    <p className="text-sm font-medium" style={{ color: "var(--text-1)" }}>
                      {entry.label}
                    </p>
                    <p className="text-xs" style={{ color: "var(--text-3)", marginTop: 2 }}>
                      {section.desc}
                    </p>
                  </div>
                  <span
                    className="text-[10px] rounded-full font-medium"
                    style={{ background: "var(--bg-3)", color: "var(--text-3)", padding: "4px 8px" }}
                  >
                    {locale === "fr" ? "Bientôt" : "Soon"}
                  </span>
                </div>
              );
            })}
          </div>
        </div>

        {/* Footer */}
        <div
          className="flex justify-end"
          style={{ borderTop: "1px solid var(--surface-border)", background: "var(--bg-1)", padding: "12px 24px" }}
        >
          <button
            onClick={() => setSettingsOpen(false)}
            className="rounded-lg text-xs font-medium transition-all"
            style={{
              padding: "8px 16px",
              background: "var(--accent)",
              color: "#fff",
              cursor: "pointer",
            }}
          >
            {t("search.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
