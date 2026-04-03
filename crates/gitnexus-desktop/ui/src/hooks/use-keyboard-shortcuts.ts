import { useEffect } from "react";
import { useAppStore } from "../stores/app-store";
import { useChatStore } from "../stores/chat-store";

function isInputElement(target: HTMLElement): boolean {
  return (
    target.tagName === "INPUT" ||
    target.tagName === "TEXTAREA" ||
    target.isContentEditable
  );
}

export function useKeyboardShortcuts() {
  const setMode = useAppStore((s) => s.setMode);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const setSearchOpen = useAppStore((s) => s.setSearchOpen);
  const setZoomLevel = useAppStore((s) => s.setZoomLevel);
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen);
  const setExplorerLeftCollapsed = useAppStore((s) => s.setExplorerLeftCollapsed);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      const isInput = isInputElement(target);

      // Ctrl+K → open command palette
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        useAppStore.getState().setCommandPaletteOpen(!useAppStore.getState().commandPaletteOpen);
      }

      // Ctrl+\ → toggle detail panel (deselect node)
      if ((e.ctrlKey || e.metaKey) && e.key === "\\") {
        e.preventDefault();
        useAppStore.getState().setSelectedNodeId(null);
      }

      // Ctrl+1 → Switch to Explorer mode
      if ((e.ctrlKey || e.metaKey) && e.key === "1") {
        e.preventDefault();
        setMode("explorer");
      }

      // Ctrl+2 → Switch to Analyze mode
      if ((e.ctrlKey || e.metaKey) && e.key === "2") {
        e.preventDefault();
        setMode("analyze");
      }

      // Ctrl+3 → Switch to Chat mode
      if ((e.ctrlKey || e.metaKey) && e.key === "3") {
        e.preventDefault();
        setMode("chat");
      }

      // Ctrl+4 → Switch to Manage mode
      if ((e.ctrlKey || e.metaKey) && e.key === "4") {
        e.preventDefault();
        setMode("manage");
      }

      // Ctrl+B → toggle explorer left panel
      if ((e.ctrlKey || e.metaKey) && e.key === "b") {
        e.preventDefault();
        setExplorerLeftCollapsed(!useAppStore.getState().explorerLeftCollapsed);
      }

      // Escape → close chat modals, deselect, close search/palette
      if (e.key === "Escape") {
        const chatState = useChatStore.getState();
        const appState = useAppStore.getState();
        if (chatState.activeModal) {
          chatState.closeModal();
        } else if (appState.commandPaletteOpen) {
          appState.setCommandPaletteOpen(false);
        } else if (appState.settingsOpen) {
          appState.setSettingsOpen(false);
        } else if (appState.searchOpen) {
          appState.setSearchOpen(false);
        } else {
          appState.setSelectedNodeId(null);
        }
      }

      // Ctrl+Shift+D → Toggle deep research mode
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "D") {
        e.preventDefault();
        useChatStore.getState().toggleDeepResearch();
      }

      // F → Fit graph to screen (when not in an input, explorer mode only)
      if (!isInput && e.key.toLowerCase() === "f") {
        if (useAppStore.getState().mode === "explorer") {
          e.preventDefault();
          window.dispatchEvent(new CustomEvent("gitnexus:fit-graph"));
        }
      }

      // 1 → Switch zoom level to package (when not in an input, explorer mode only)
      if (!isInput && e.key === "1" && !e.ctrlKey && !e.metaKey) {
        if (useAppStore.getState().mode === "explorer") {
          e.preventDefault();
          setZoomLevel("package");
        }
      }

      // 2 → Switch zoom level to module (when not in an input, explorer mode only)
      if (!isInput && e.key === "2" && !e.ctrlKey && !e.metaKey) {
        if (useAppStore.getState().mode === "explorer") {
          e.preventDefault();
          setZoomLevel("module");
        }
      }

      // 3 → Switch zoom level to symbol (when not in an input, explorer mode only)
      if (!isInput && e.key === "3" && !e.ctrlKey && !e.metaKey) {
        if (useAppStore.getState().mode === "explorer") {
          e.preventDefault();
          setZoomLevel("symbol");
        }
      }

      // L → Cycle graph layouts (when not in an input, explorer mode only)
      if (!isInput && e.key.toLowerCase() === "l") {
        if (useAppStore.getState().mode === "explorer") {
          e.preventDefault();
          window.dispatchEvent(new CustomEvent("gitnexus:cycle-layout"));
        }
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [
    setMode,
    setSelectedNodeId,
    setSearchOpen,
    setZoomLevel,
    setSettingsOpen,
    setExplorerLeftCollapsed,
  ]);
}
