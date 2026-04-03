import { ModeBar } from "./components/layout/ModeBar";
import { ModeRouter } from "./components/layout/ModeRouter";
import { StatusBar } from "./components/layout/StatusBar";
import { SearchModal } from "./components/search/SearchModal";
import { SettingsModal } from "./components/layout/SettingsModal";
import { CommandPalette } from "./components/layout/CommandPalette";
import { useKeyboardShortcuts } from "./hooks/use-keyboard-shortcuts";
import { useScreenCapture } from "./hooks/use-screen-capture";
import { useI18n } from "./hooks/use-i18n";

function App() {
  const { t } = useI18n();
  useKeyboardShortcuts();
  useScreenCapture();

  return (
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
  );
}

export default App;
