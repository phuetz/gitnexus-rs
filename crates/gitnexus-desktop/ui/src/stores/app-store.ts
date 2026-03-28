import { create } from "zustand";
import type { ZoomLevel } from "../lib/tauri-commands";

export type SidebarTab = "overview" | "repos" | "search" | "files" | "graph" | "impact" | "docs" | "export";
export type DetailTab = "context" | "code" | "properties";

interface AppState {
  activeRepo: string | null;
  setActiveRepo: (name: string | null) => void;

  selectedNodeId: string | null;
  selectedNodeName: string | null;
  setSelectedNodeId: (id: string | null, name?: string | null) => void;

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
}

export const useAppStore = create<AppState>((set) => ({
  activeRepo: null,
  setActiveRepo: (name) => set({ activeRepo: name }),

  selectedNodeId: null,
  selectedNodeName: null,
  setSelectedNodeId: (id, name) => set({ selectedNodeId: id, selectedNodeName: name ?? null }),

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
}));
