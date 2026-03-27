/**
 * Chat Intelligence Store — Zustand state for the smart chat system.
 *
 * Manages:
 * - Context filters (files, symbols, modules, languages)
 * - Active research plan and step execution state
 * - Filter modal visibility
 * - Quick-pick search state
 */

import { create } from "zustand";
import type {
  ChatContextFilter,
  ResearchPlan,
  QueryAnalysis,
  QueryComplexity,
  FileQuickPick,
  SymbolQuickPick,
  ModuleQuickPick,
} from "../lib/tauri-commands";

// ─── Types ───────────────────────────────────────────────────────────

export type FilterModalType = "files" | "symbols" | "modules" | null;

interface ChatStoreState {
  // ── Context Filters ──────────────────────────────────────────
  filters: ChatContextFilter;
  setFilters: (filters: ChatContextFilter) => void;
  addFileFilter: (file: string) => void;
  removeFileFilter: (file: string) => void;
  addSymbolFilter: (symbol: string) => void;
  removeSymbolFilter: (symbol: string) => void;
  addModuleFilter: (module: string) => void;
  removeModuleFilter: (module: string) => void;
  addLanguageFilter: (lang: string) => void;
  removeLanguageFilter: (lang: string) => void;
  clearFilters: () => void;
  hasActiveFilters: () => boolean;

  // ── Filter Modals ────────────────────────────────────────────
  activeModal: FilterModalType;
  openModal: (modal: FilterModalType) => void;
  closeModal: () => void;

  // ── Research Plan ────────────────────────────────────────────
  activePlan: ResearchPlan | null;
  setActivePlan: (plan: ResearchPlan | null) => void;
  updatePlanStep: (stepId: string, update: Partial<ResearchPlan["steps"][0]>) => void;

  // ── Query Analysis ───────────────────────────────────────────
  lastAnalysis: QueryAnalysis | null;
  setLastAnalysis: (analysis: QueryAnalysis | null) => void;

  // ── Deep Research Mode ───────────────────────────────────────
  deepResearchEnabled: boolean;
  toggleDeepResearch: () => void;
  setDeepResearch: (enabled: boolean) => void;

  // ── Plan Panel Visibility ────────────────────────────────────
  planPanelOpen: boolean;
  setPlanPanelOpen: (open: boolean) => void;

  // ── Quick Pick Results (cached for display) ──────────────────
  filePickResults: FileQuickPick[];
  setFilePickResults: (results: FileQuickPick[]) => void;
  symbolPickResults: SymbolQuickPick[];
  setSymbolPickResults: (results: SymbolQuickPick[]) => void;
  modulePickResults: ModuleQuickPick[];
  setModulePickResults: (results: ModuleQuickPick[]) => void;
}

// ─── Default Filter ──────────────────────────────────────────────────

const emptyFilter: ChatContextFilter = {
  files: [],
  symbols: [],
  modules: [],
  languages: [],
  labels: [],
};

// ─── Store ───────────────────────────────────────────────────────────

export const useChatStore = create<ChatStoreState>((set, get) => ({
  // ── Context Filters ──────────────────────────────────────────
  filters: { ...emptyFilter },
  setFilters: (filters) => set({ filters }),

  addFileFilter: (file) =>
    set((s) => ({
      filters: {
        ...s.filters,
        files: s.filters.files.includes(file)
          ? s.filters.files
          : [...s.filters.files, file],
      },
    })),

  removeFileFilter: (file) =>
    set((s) => ({
      filters: {
        ...s.filters,
        files: s.filters.files.filter((f) => f !== file),
      },
    })),

  addSymbolFilter: (symbol) =>
    set((s) => ({
      filters: {
        ...s.filters,
        symbols: s.filters.symbols.includes(symbol)
          ? s.filters.symbols
          : [...s.filters.symbols, symbol],
      },
    })),

  removeSymbolFilter: (symbol) =>
    set((s) => ({
      filters: {
        ...s.filters,
        symbols: s.filters.symbols.filter((sym) => sym !== symbol),
      },
    })),

  addModuleFilter: (module) =>
    set((s) => ({
      filters: {
        ...s.filters,
        modules: s.filters.modules.includes(module)
          ? s.filters.modules
          : [...s.filters.modules, module],
      },
    })),

  removeModuleFilter: (module) =>
    set((s) => ({
      filters: {
        ...s.filters,
        modules: s.filters.modules.filter((m) => m !== module),
      },
    })),

  addLanguageFilter: (lang) =>
    set((s) => ({
      filters: {
        ...s.filters,
        languages: s.filters.languages.includes(lang)
          ? s.filters.languages
          : [...s.filters.languages, lang],
      },
    })),

  removeLanguageFilter: (lang) =>
    set((s) => ({
      filters: {
        ...s.filters,
        languages: s.filters.languages.filter((l) => l !== lang),
      },
    })),

  clearFilters: () => set({ filters: { ...emptyFilter } }),

  hasActiveFilters: () => {
    const { filters } = get();
    return (
      filters.files.length > 0 ||
      filters.symbols.length > 0 ||
      filters.modules.length > 0 ||
      filters.languages.length > 0 ||
      filters.labels.length > 0
    );
  },

  // ── Filter Modals ────────────────────────────────────────────
  activeModal: null,
  openModal: (modal) => set({ activeModal: modal }),
  closeModal: () => set({ activeModal: null }),

  // ── Research Plan ────────────────────────────────────────────
  activePlan: null,
  setActivePlan: (plan) =>
    set({ activePlan: plan, planPanelOpen: plan !== null }),

  updatePlanStep: (stepId, update) =>
    set((s) => {
      if (!s.activePlan) return {};
      return {
        activePlan: {
          ...s.activePlan,
          steps: s.activePlan.steps.map((step) =>
            step.id === stepId ? { ...step, ...update } : step
          ),
        },
      };
    }),

  // ── Query Analysis ───────────────────────────────────────────
  lastAnalysis: null,
  setLastAnalysis: (analysis) => set({ lastAnalysis: analysis }),

  // ── Deep Research Mode ───────────────────────────────────────
  deepResearchEnabled: false,
  toggleDeepResearch: () =>
    set((s) => ({ deepResearchEnabled: !s.deepResearchEnabled })),
  setDeepResearch: (enabled) => set({ deepResearchEnabled: enabled }),

  // ── Plan Panel Visibility ────────────────────────────────────
  planPanelOpen: false,
  setPlanPanelOpen: (open) => set({ planPanelOpen: open }),

  // ── Quick Pick Results ───────────────────────────────────────
  filePickResults: [],
  setFilePickResults: (results) => set({ filePickResults: results }),
  symbolPickResults: [],
  setSymbolPickResults: (results) => set({ symbolPickResults: results }),
  modulePickResults: [],
  setModulePickResults: (results) => set({ modulePickResults: results }),
}));
