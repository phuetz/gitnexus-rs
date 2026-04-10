# GitNexus Desktop — Post-Redesign Audit

**Date**: 2026-04-03
**Scope**: Full UX, code quality, and architecture audit after 4-mode redesign
**Benchmark**: Sourcetrail, Neo4j Bloom, Linear, Zed, Gephi

---

## Audit Summary

| Severity | Count |
|----------|-------|
| CRITICAL | 1 |
| HIGH | 9 |
| MEDIUM | 14 |
| LOW | 12 |

**Positives confirmed**: Always-mounted Explorer (correct), navigation history (well-implemented), design token system (comprehensive), ErrorBoundary coverage (good), Sigma lifecycle (proper), clean TypeScript (zero errors), i18n bilingual support, staleTime strategy (sound).

---

## CRITICAL

### C1. GraphExplorer.tsx is a God Object
**File**: `components/graph/GraphExplorer.tsx` (1080+ lines, 19 useState, 9 useEffect)
**Issue**: Single component manages view mode, legend, minimap, impact overlay, focus mode, hidden edges, depth filter, shortcuts panel, layout, feature selection, lens filtering, context menu, hover card, and more.
**Fix**: Extract into sub-components: `GraphCanvas`, `GraphOverlays`, `GraphContextMenu`, `GraphLegend`, `GraphShortcutsPanel`. Consolidate 19 state variables into a `useGraphReducer` custom hook.
**Test**: After extraction, no single file exceeds 300 lines; all graph features still functional.

---

## HIGH

### H1. Ego-network feature not wired (UI visible but non-functional)
**Files**: `GraphExplorer.tsx` (never reads `egoDepth`), `use-sigma.ts:159-182` (ready but unused)
**Issue**: `EgoDepthSlider` writes to `egoDepth` in the store but `GraphExplorer` never reads it, never computes `egoNodeIds`/`egoDepthMap`, never passes them to `useSigma`. The reducer code in `use-sigma.ts` is ready but unwired.
**Fix**: In `GraphExplorer`, read `egoDepth` from store. On `selectedNodeId` change, perform local BFS on `graphRef.current` out to `egoDepth` hops. Build `Set<string>` + `Map<string, number>`, pass to `useSigma`.
**Test**: Select a node with `egoDepth=1` → only direct neighbors at full opacity, rest at 10%.

### H2. Missing cache invalidation after re-index
**File**: `hooks/use-tauri-query.ts:12-20`
**Issue**: After `analyze_repo`, only `["graph"]` and `["file-tree"]` caches are invalidated. Context, impact, subgraph, file content, search, coverage, features, code health all have `staleTime: Infinity` and serve stale data.
**Fix**: After successful re-indexing, call `queryClient.invalidateQueries()` (no filter) to clear all caches.
**Test**: Re-analyze a repo → context/impact data reflects new analysis immediately.

### H3. Duplicated FeatureNavigator state
**Files**: `ExplorerLeftPanel.tsx:6` and `GraphExplorer.tsx:55`
**Issue**: Two independent `selectedFeatures` useState instances. Left panel selection doesn't affect graph, and vice versa.
**Fix**: Lift `selectedFeatures` to `app-store.ts` as explorer-mode state.
**Test**: Toggle a community in left panel → graph filters accordingly.

### H4. CustomEvent bus for lens changes
**Files**: `LensSelector.tsx:39-43`, `GraphExplorer.tsx:385-392`
**Issue**: Lens changes use `window.dispatchEvent(CustomEvent)` instead of store subscription. Bypasses React data flow.
**Fix**: In `GraphExplorer`, subscribe to `activeLens` from store, derive `visibleEdgeTypes` via `useMemo` from `activeLens` + `LENS_EDGE_TYPES` map. Remove CustomEvent dispatch/listener.
**Test**: Change lens → edges filter correctly without CustomEvent.

### H5. Stale closure in keyboard shortcuts
**File**: `hooks/use-keyboard-shortcuts.ts:66`
**Issue**: `explorerLeftCollapsed` captured at subscription time. Rapid Ctrl+B presses race between old/new effect registrations.
**Fix**: Use `useAppStore.getState().explorerLeftCollapsed` inline.
**Test**: Rapid Ctrl+B toggles → panel toggles correctly every time.

### H6. No responsive behavior despite hook existing
**Files**: `hooks/use-responsive.ts` (unused), `components/explorer/ExplorerMode.tsx`
**Issue**: `useResponsive` defines breakpoints (900px, 700px) but is never imported. Explorer panels never auto-collapse.
**Fix**: In `ExplorerMode`, auto-collapse left panel at <900px, hide right panel at <700px.
**Test**: Resize window to 800px → left panel collapses. 650px → right panel collapses.

### H7. Missing accessibility attributes
**Files**: `EgoDepthSlider.tsx:22-32`, `AnalyzeNav.tsx:37-52`, `LensSelector.tsx:33-57`
**Issue**: Ego slider buttons have no `aria-label`. AnalyzeNav buttons lack `aria-current`. LensSelector `<select>` has no label.
**Fix**: Add `aria-label="Ego network depth N"`, `aria-current="page"`, `aria-label="Graph lens filter"`.
**Test**: Screen reader announces purpose of each control.

### H8. ChatMode config loading flash
**File**: `components/chat/ChatMode.tsx:132-134`
**Issue**: While config loads, code falls through to chat UI. Then if no API key, flashes to setup card.
**Fix**: Show loading spinner while `configLoading` is true.
**Test**: Open Chat mode with slow config → spinner shown, no flash.

### H9. AnalyzeMode wrappers lack error handling
**File**: `components/analyze/AnalyzeMode.tsx:15-40`
**Issue**: `HotspotsWrapper`, `CouplingWrapper`, `OwnershipWrapper` don't pass `error` from `useQuery`. Failed queries show infinite loading.
**Fix**: Destructure `error`, show error state to user.
**Test**: Simulate IPC error → user sees error message, not infinite spinner.

---

## MEDIUM

### M1. CytoNode TS type missing `depth` field
**File**: `lib/tauri-commands.ts:21-40`
**Fix**: Add `depth?: number;` to CytoNode interface.

### M2. Keyboard shortcuts fire in wrong modes
**File**: `hooks/use-keyboard-shortcuts.ts:92-119`
**Fix**: Guard graph shortcuts with `if (mode === 'explorer')`.

### M3. `explorerRightCollapsed` store state never consumed
**File**: `stores/app-store.ts:65-66`
**Fix**: Wire to ExplorerMode right panel or remove.

### M4. Silent error swallowing in impact overlay
**File**: `components/graph/GraphExplorer.tsx:304`
**Fix**: Add toast notification on failure.

### M5. Legend label counting runs on every render
**File**: `components/graph/GraphExplorer.tsx:1068-1079`
**Fix**: Wrap in `useMemo(() => ..., [data])`.

### M6. `buildCommands()` called on every render of CommandPalette
**File**: `components/layout/CommandPalette.tsx:269`
**Fix**: Wrap in `useMemo`.

### M7. No `React.lazy` for non-Explorer modes
**File**: `components/layout/ModeRouter.tsx:3-6`
**Fix**: Use `React.lazy` + `<Suspense>` for Analyze, Chat, Manage modes.

### M8. No mode transition animations
**File**: `components/layout/ModeRouter.tsx:8-47`
**Fix**: Wrap in `<AnimatePresence>` from framer-motion (already a dependency).

### M9. Empty state messages not internationalized
**Files**: `ExplorerMode.tsx:27-37`, `ExplorerRightPanel.tsx:19`
**Fix**: Use `t("explorer.noRepo")` etc.

### M10. Hardcoded Sigma colors bypass design tokens
**File**: `hooks/use-sigma.ts:88-148`
**Fix**: Read CSS custom properties at init time, re-initialize on theme change.

### M11. Context menu not keyboard-navigable
**File**: `components/graph/GraphExplorer.tsx:712-813`
**Fix**: Add `role="menu"`, `role="menuitem"`, arrow-key navigation, focus trap.

### M12. `dangerouslySetInnerHTML` for SVG
**File**: `components/docs/DocsContent.tsx:168`
**Fix**: Verify `sanitizeSvg` uses DOMPurify or equivalent.

### M13. O(E) full-scan for edge collection in Rust
**Files**: `graph.rs:145-154`, `impact.rs:63-73`
**Fix**: Use `indexes.outgoing` for O(selected * degree) instead of O(total_edges).

### M14. No Zustand persist middleware
**File**: `stores/app-store.ts`
**Fix**: Add `persist` with whitelist: `theme`, `mode`, `analyzeView`, `activeLens`, `egoDepth`, `zoomLevel`.

---

## LOW (12 items)

| # | Issue | File | Fix |
|---|-------|------|-----|
| L1 | Duplicated `node_to_cyto` helper | `graph.rs:287`, `impact.rs:166` | Extract shared fn |
| L2 | Error state ordering wrong | `GraphExplorer.tsx:532` | Move error check before empty state |
| L3 | CommandPalette duplicate heading | `CommandPalette.tsx:336-338` | Remove extra div |
| L4 | AnalyzeNav fixed 160px width | `AnalyzeNav.tsx:24` | Add icon-only collapsed mode |
| L5 | Zoom controls use `title` not `<Tooltip>` | `GraphExplorer.tsx:971-1009` | Use `<Tooltip>` component |
| L6 | Legend close button is letter "x" | `GraphExplorer.tsx:1059` | Use `<X>` icon + aria-label |
| L7 | ManageMode inline i18n fallbacks | `ManageMode.tsx:110` | Use `t()` function |
| L8 | No `React.memo` on leaf components | Various | Apply to EgoDepthSlider, LensSelector, StatusBar |
| L9 | Clipboard API without try/catch | `GraphExplorer.tsx:794,805` | Add try/catch + toast |
| L10 | ErrorBoundary missing `componentDidCatch` | `ErrorBoundary.tsx` | Add logging |
| L11 | No global error toast for IPC failures | `main.tsx:25` | Add QueryClient `onError` |
| L12 | `drawMinimap` empty deps with eslint-disable | `GraphExplorer.tsx:460` | Document or fix |

---

## Missing Features vs. Competitors

| Feature | Status | Priority |
|---------|--------|----------|
| Breadcrumb trail for graph navigation | Missing | Medium |
| Saved graph views / bookmarks | Missing | Low |
| Minimap click-to-navigate | Display only | Medium |
| Graph search with inline autocomplete (not modal) | Missing | High |
| Node type filtering checkboxes (custom, not just presets) | Missing | Medium |
| Animation between graph states | Missing | Low |
| Keyboard-navigable graph (Tab between nodes) | Missing | Medium |
| Multi-select nodes | Missing | Low |
| Drag-to-select region | Missing | Low |
| Export to SVG (vector) | Missing (PNG only) | Low |
| Theme-aware graph canvas (light/dark) | Missing (dark only) | Medium |
| Command palette with recent items / history | Missing | Medium |

---

## Recommended Fix Order

### Phase A — Critical fixes (blocks user-facing quality)
1. **H1**: Wire ego-network (the signature feature)
2. **H2**: Cache invalidation after re-index
3. **H3**: Lift selectedFeatures to store
4. **H4**: Replace CustomEvent with store subscription
5. **M1**: Add `depth` to TS CytoNode type

### Phase B — UX quality (polish for production)
6. **H6**: Responsive behavior
7. **H7**: Accessibility attributes
8. **H8**: Chat loading flash
9. **H9**: Analyze error handling
10. **M8**: Mode transition animations
11. **M9**: i18n empty states
12. **M14**: Zustand persist

### Phase C — Architecture (maintainability)
13. **C1**: Split GraphExplorer God Object
14. **H5**: Fix stale closure
15. **M2**: Guard mode-specific shortcuts
16. **M5-M7**: Performance (memos, lazy loading)
17. **M10**: Theme-aware Sigma colors
18. **M13**: O(E) edge scan optimization

### Phase D — Polish (nice-to-have)
19. All LOW items
20. Missing competitor features (start with inline graph search + breadcrumb trail)
