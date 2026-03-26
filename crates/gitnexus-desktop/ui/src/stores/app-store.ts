import { create } from "zustand";
import type { ZoomLevel } from "../lib/tauri-commands";

export type SidebarTab = "repos" | "search" | "files" | "graph" | "impact" | "docs";
export type DetailTab = "context" | "code" | "properties";

interface AppState {
  activeRepo: string | null;
  setActiveRepo: (name: string | null) => void;

  selectedNodeId: string | null;
  setSelectedNodeId: (id: string | null) => void;

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
}

export const useAppStore = create<AppState>((set) => ({
  activeRepo: null,
  setActiveRepo: (name) => set({ activeRepo: name }),

  selectedNodeId: null,
  setSelectedNodeId: (id) => set({ selectedNodeId: id }),

  sidebarTab: "repos",
  setSidebarTab: (tab) => set({ sidebarTab: tab }),

  sidebarCollapsed: false,
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),

  detailTab: "context",
  setDetailTab: (tab) => set({ detailTab: tab }),

  zoomLevel: "package",
  setZoomLevel: (level) => set({ zoomLevel: level }),

  searchOpen: false,
  setSearchOpen: (open) => set({ searchOpen: open }),
}));
