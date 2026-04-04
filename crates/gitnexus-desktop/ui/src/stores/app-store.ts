import { create } from "zustand";
import type { ZoomLevel } from "../lib/tauri-commands";

export type DetailTab = "context" | "code" | "properties" | "layers" | "health";
export type ThemeMode = "dark" | "light" | "system";
export type AppMode = 'explorer' | 'analyze' | 'chat' | 'manage';
export type AnalyzeView = 'overview' | 'hotspots' | 'coupling' | 'ownership' | 'coverage' | 'diagram' | 'report' | 'health';
export type LensType = 'all' | 'calls' | 'structure' | 'heritage' | 'impact' | 'dead-code' | 'tracing';

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

  searchMatchIds: string[];
  setSearchMatchIds: (ids: string[]) => void;

  theme: ThemeMode;
  setTheme: (theme: ThemeMode) => void;

  mode: AppMode;
  setMode: (mode: AppMode) => void;
  analyzeView: AnalyzeView;
  setAnalyzeView: (view: AnalyzeView) => void;
  activeLens: LensType;
  setActiveLens: (lens: LensType) => void;
  egoDepth: 1 | 2 | 3;
  setEgoDepth: (depth: 1 | 2 | 3) => void;
  explorerLeftCollapsed: boolean;
  setExplorerLeftCollapsed: (collapsed: boolean) => void;

  sidebarExpanded: boolean;
  setSidebarExpanded: (expanded: boolean) => void;
  toggleSidebar: () => void;

  selectedFeatures: Set<string>;
  toggleFeature: (id: string) => void;
  resetFeatures: () => void;
}

const MAX_HISTORY = 50;

export const useAppStore = create<AppState>((set, get) => ({
  activeRepo: null,
  setActiveRepo: (name) => {
    if (name) {
      localStorage.setItem("gitnexus-active-repo", name);
    } else {
      localStorage.removeItem("gitnexus-active-repo");
    }
    set({ activeRepo: name });
  },

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

  searchMatchIds: [],
  setSearchMatchIds: (ids) => set({ searchMatchIds: ids }),

  theme: (() => {
    const saved = localStorage.getItem("gitnexus-theme");
    return saved === "dark" || saved === "light" || saved === "system" ? saved : "dark";
  })(),
  setTheme: (theme) => {
    localStorage.setItem("gitnexus-theme", theme);
    const resolved = theme === "system"
      ? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
      : theme;
    document.documentElement.setAttribute("data-theme", resolved);
    set({ theme });
  },

  mode: 'explorer',
  setMode: (mode) => set({ mode }),
  analyzeView: 'overview',
  setAnalyzeView: (view) => set({ analyzeView: view }),
  activeLens: 'all',
  setActiveLens: (lens) => set({ activeLens: lens }),
  egoDepth: 2,
  setEgoDepth: (depth) => set({ egoDepth: depth }),
  explorerLeftCollapsed: false,
  setExplorerLeftCollapsed: (collapsed) => set({ explorerLeftCollapsed: collapsed }),

  sidebarExpanded: false,
  setSidebarExpanded: (expanded) => set({ sidebarExpanded: expanded }),
  toggleSidebar: () => set((s) => ({ sidebarExpanded: !s.sidebarExpanded })),

  selectedFeatures: new Set<string>(),
  toggleFeature: (id) => set((s) => {
    const next = new Set(s.selectedFeatures);
    if (next.has(id)) next.delete(id); else next.add(id);
    return { selectedFeatures: next };
  }),
  resetFeatures: () => set({ selectedFeatures: new Set<string>() }),
}));
