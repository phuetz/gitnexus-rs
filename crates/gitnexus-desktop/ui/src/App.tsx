import { useEffect } from "react";
import { MotionConfig, useReducedMotion } from "framer-motion";
import { ModeBar } from "./components/layout/ModeBar";
import { ModeRouter } from "./components/layout/ModeRouter";
import { StatusBar } from "./components/layout/StatusBar";
import { SearchModal } from "./components/search/SearchModal";
import { SettingsModal } from "./components/layout/SettingsModal";
import { CommandPalette } from "./components/layout/CommandPalette";
import { useKeyboardShortcuts } from "./hooks/use-keyboard-shortcuts";
import { useScreenCapture } from "./hooks/use-screen-capture";
import { useI18n } from "./hooks/use-i18n";
import { useAppStore } from "./stores/app-store";
import { commands } from "./lib/tauri-commands";

function App() {
  const { t, locale } = useI18n();
  const shouldReduceMotion = useReducedMotion();
  useKeyboardShortcuts();
  useScreenCapture();

  // Set HTML lang attribute based on locale
  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);

  // Listen for OS theme changes when "system" theme is selected
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => {
      if (useAppStore.getState().theme === "system") {
        document.documentElement.setAttribute("data-theme", mq.matches ? "dark" : "light");
      }
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  // Auto-restore the last active repo on startup
  useEffect(() => {
    let ignore = false;
    const saved = localStorage.getItem("gitnexus-active-repo");
    if (saved && !useAppStore.getState().activeRepo) {
      // Must load registry first — openRepo reads from the in-memory registry
      commands.listRepos().then(() =>
        commands.openRepo(saved)
      ).then(() => {
        if (!ignore) useAppStore.getState().setActiveRepo(saved);
      }).catch((err) => {
        console.warn("Failed to restore repo:", err);
        if (!ignore) localStorage.removeItem("gitnexus-active-repo");
      });
    }
    return () => { ignore = true; };
  }, []);

  return (
    <MotionConfig reducedMotion={shouldReduceMotion ? "always" : "never"}>
    <div className="flex flex-col h-screen w-screen overflow-hidden" style={{ background: "var(--bg-0)" }}>
      <a href="#main-content" className="sr-only focus:not-sr-only focus:absolute focus:z-[9999]" style={{ top: 8, left: 8, padding: "8px 16px", backgroundColor: "var(--accent)", color: "white", borderRadius: 8, fontWeight: 600, fontSize: 13 }}>
        {t("a11y.skipToContent")}
      </a>
      <h1 className="sr-only">{t("a11y.codeIntelligencePlatform")}</h1>

      <div className="flex flex-1 min-h-0">
        <nav aria-label="Mode navigation">
          <ModeBar />
        </nav>
        <main id="main-content" className="flex-1 min-w-0 relative">
          <ModeRouter />
        </main>
      </div>

      <footer>
        <StatusBar />
      </footer>

      <SearchModal />
      <SettingsModal />
      <CommandPalette />
    </div>
    </MotionConfig>
  );
}

export default App;
