import { create } from "zustand";
import type { ZoomLevel } from "../lib/tauri-commands";

export type SidebarTab = "overview" | "repos" | "search" | "files" | "graph" | "impact" | "docs" | "export" | "git-analytics" | "chat" | "coverage" | "diagram" | "report";
export type DetailTab = "context" | "code" | "properties" | "layers" | "health";
export type ThemeMode = "dark" | "light" | "system";

interface HistoryEntry {
  nodeId: string;
  nodeName: string | null;
}

interface AppState {
  activeRepo: string | null;
  setActiveRepo: (name: string | null) => void;

  selectedNodeId: string | null;
  selectedNodeName: string | null;
  setSelectedNodeId: (id: string | null, name?: string | null) => void;

  // Navigation history (browser-style back/forward)
  navigationHistory: HistoryEntry[];
  historyIndex: number;
  canGoBack: boolean;
  canGoForward: boolean;
  goBack: () => void;
  goForward: () => void;

  sidebarTab: SidebarTab;
  setSidebarTab: (tab: SidebarTab) => void;

  sidebarCollapsed: boolean;
  toggleSidebar: () => void;

  detailTab: DetailTab;
  setDetailTab: (tab: DetailTab) => void;

  zoomLevel: ZoomLevel;
  setZoomLevel: (level: ZoomLevel) => void;

  searchOpen: boolean;
  setSearchOpen: (open: boolean) => void;

  searchQuery: string;
  setSearchQuery: (query: string) => void;

  settingsOpen: boolean;
  setSettingsOpen: (open: boolean) => void;

  commandPaletteOpen: boolean;
  setCommandPaletteOpen: (open: boolean) => void;

  theme: ThemeMode;
  setTheme: (theme: ThemeMode) => void;
}

const MAX_HISTORY = 50;

export const useAppStore = create<AppState>((set, get) => ({
  activeRepo: null,
  setActiveRepo: (name) => set({ activeRepo: name }),

  selectedNodeId: null,
  selectedNodeName: null,
  setSelectedNodeId: (id, name) => {
    const state = get();
    if (id && id !== state.selectedNodeId) {
      // Push to navigation history (truncate forward history)
      const truncated = state.navigationHistory.slice(0, state.historyIndex + 1);
      const entry: HistoryEntry = { nodeId: id, nodeName: name ?? null };
      const newHistory = [...truncated, entry].slice(-MAX_HISTORY);
      const newIndex = newHistory.length - 1;
      set({
        selectedNodeId: id,
        selectedNodeName: name ?? null,
        navigationHistory: newHistory,
        historyIndex: newIndex,
        canGoBack: newIndex > 0,
        canGoForward: false,
      });
    } else {
      set({ selectedNodeId: id, selectedNodeName: name ?? null });
    }
  },

  navigationHistory: [],
  historyIndex: -1,
  canGoBack: false,
  canGoForward: false,

  goBack: () => {
    const { navigationHistory, historyIndex } = get();
    if (historyIndex > 0) {
      const newIndex = historyIndex - 1;
      const entry = navigationHistory[newIndex];
      set({
        selectedNodeId: entry.nodeId,
        selectedNodeName: entry.nodeName,
        historyIndex: newIndex,
        canGoBack: newIndex > 0,
        canGoForward: true,
      });
    }
  },

  goForward: () => {
    const { navigationHistory, historyIndex } = get();
    if (historyIndex < navigationHistory.length - 1) {
      const newIndex = historyIndex + 1;
      const entry = navigationHistory[newIndex];
      set({
        selectedNodeId: entry.nodeId,
        selectedNodeName: entry.nodeName,
        historyIndex: newIndex,
        canGoBack: true,
        canGoForward: newIndex < navigationHistory.length - 1,
      });
    }
  },

  sidebarTab: "repos",
  setSidebarTab: (tab) => set(() => ({
    sidebarTab: tab,
    // Clear node selection when leaving graph-related tabs
    ...(tab !== "graph" && tab !== "impact" && tab !== "search"
      ? { selectedNodeId: null, selectedNodeName: null }
      : {}),
  })),

  sidebarCollapsed: false,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),

  detailTab: "context",
  setDetailTab: (tab) => set({ detailTab: tab }),

  zoomLevel: "package",
  setZoomLevel: (level) => set({ zoomLevel: level }),

  searchOpen: false,
  setSearchOpen: (open) => set({ searchOpen: open }),

  searchQuery: "",
  setSearchQuery: (query) => set({ searchQuery: query }),

  settingsOpen: false,
  setSettingsOpen: (open) => set({ settingsOpen: open }),

  commandPaletteOpen: false,
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),

  theme: (localStorage.getItem("gitnexus-theme") as ThemeMode) || "dark",
  setTheme: (theme) => {
    localStorage.setItem("gitnexus-theme", theme);
    document.documentElement.setAttribute("data-theme", theme);
    set({ theme });
  },
}));
