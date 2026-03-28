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
  const setSidebarTab = useAppStore((s) => s.setSidebarTab);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const setSearchOpen = useAppStore((s) => s.setSearchOpen);
  const setZoomLevel = useAppStore((s) => s.setZoomLevel);
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      const isInput = isInputElement(target);

      // Ctrl+K → open search modal
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        useAppStore.getState().setSearchOpen(!useAppStore.getState().searchOpen);
      }

      // Ctrl+\ → toggle detail panel (deselect node)
      if ((e.ctrlKey || e.metaKey) && e.key === "\\") {
        e.preventDefault();
        useAppStore.getState().setSelectedNodeId(null);
      }

      // Ctrl+1 → Switch to Repositories tab
      if ((e.ctrlKey || e.metaKey) && e.key === "1") {
        e.preventDefault();
        setSidebarTab("repos");
      }

      // Ctrl+2 → Switch to File Explorer tab
      if ((e.ctrlKey || e.metaKey) && e.key === "2") {
        e.preventDefault();
        setSidebarTab("files");
      }

      // Ctrl+3 → Switch to Graph Explorer tab
      if ((e.ctrlKey || e.metaKey) && e.key === "3") {
        e.preventDefault();
        setSidebarTab("graph");
      }

      // Ctrl+4 → Switch to Impact Analysis tab
      if ((e.ctrlKey || e.metaKey) && e.key === "4") {
        e.preventDefault();
        setSidebarTab("impact");
      }

      // Ctrl+5 → Switch to Documentation tab
      if ((e.ctrlKey || e.metaKey) && e.key === "5") {
        e.preventDefault();
        setSidebarTab("docs");
      }

      // Ctrl+B → toggle sidebar
      if ((e.ctrlKey || e.metaKey) && e.key === "b") {
        e.preventDefault();
        useAppStore.getState().toggleSidebar();
      }

      // Escape → close chat modals, deselect, close search
      if (e.key === "Escape") {
        const chatState = useChatStore.getState();
        const appState = useAppStore.getState();
        if (chatState.activeModal) {
          chatState.closeModal();
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

      // F → Fit graph to screen (when not in an input)
      if (!isInput && e.key.toLowerCase() === "f") {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("gitnexus:fit-graph"));
      }

      // 1 → Switch zoom level to package (when not in an input)
      if (!isInput && e.key === "1" && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        setZoomLevel("package");
      }

      // 2 → Switch zoom level to module (when not in an input)
      if (!isInput && e.key === "2" && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        setZoomLevel("module");
      }

      // 3 → Switch zoom level to symbol (when not in an input)
      if (!isInput && e.key === "3" && !e.ctrlKey && !e.metaKey) {
        e.preventDefault();
        setZoomLevel("symbol");
      }

      // L → Cycle graph layouts (when not in an input)
      if (!isInput && e.key.toLowerCase() === "l") {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("gitnexus:cycle-layout"));
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [
    setSidebarTab,
    setSelectedNodeId,
    setSearchOpen,
    setZoomLevel,
    setSettingsOpen,
  ]);
}
