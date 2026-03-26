import { useEffect } from "react";
import { useAppStore } from "../stores/app-store";

export function useKeyboardShortcuts() {
  const setSidebarTab = useAppStore((s) => s.setSidebarTab);
  const setSelectedNodeId = useAppStore((s) => s.setSelectedNodeId);
  const setSearchOpen = useAppStore((s) => s.setSearchOpen);
  const searchOpen = useAppStore((s) => s.searchOpen);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Ctrl+K → open search modal
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        setSearchOpen(!searchOpen);
      }
      // Ctrl+G → graph
      if ((e.ctrlKey || e.metaKey) && e.key === "g") {
        e.preventDefault();
        setSidebarTab("graph");
      }
      // Ctrl+B → toggle sidebar
      if ((e.ctrlKey || e.metaKey) && e.key === "b") {
        e.preventDefault();
        useAppStore.getState().toggleSidebar();
      }
      // Escape → deselect or close search
      if (e.key === "Escape") {
        if (searchOpen) {
          setSearchOpen(false);
        } else {
          setSelectedNodeId(null);
        }
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [setSidebarTab, setSelectedNodeId, setSearchOpen, searchOpen]);
}
