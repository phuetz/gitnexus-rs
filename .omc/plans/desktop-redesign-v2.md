# GitNexus Desktop — Complete Redesign Plan

**Date**: 2026-04-03
**Scope**: Full UI/UX redesign inspired by Sourcetrail, Neo4j Bloom, Linear, Zed
**Style**: Sourcetrail modernisé — 3 synchronized panels, 2026 dark-first design
**Navigation**: Mode-based (4 modes)
**Graph**: Ego-network + context (focus + transparency)
**Chat**: Dedicated full-screen mode

---

## Requirements Summary

Redesign the GitNexus desktop app from a 13-tab sidebar layout to a 4-mode architecture inspired by Sourcetrail (synchronized panels), Neo4j Bloom (perspectives/lenses), and Linear/Zed (dark-first, keyboard-first, minimal chrome). The graph must use an ego-network interaction model where selecting a symbol shows its N-hop neighborhood in full while dimming the rest.

---

## Architecture: 4 Modes

### Mode 1 — Explorer (core experience)

**Layout**: 3 synchronized panels

```
┌─────────────┬───────────────────────────┬─────────────────┐
│  Tree+Search │       Graph Canvas        │   Code+Details  │
│              │                           │                 │
│  File tree   │   Sigma.js ego-network    │  Source code     │
│  Symbol list │   with ForceAtlas2        │  (Shiki)        │
│  Quick search│   + minimap               │                 │
│              │   + legend                │  Context tabs:   │
│              │   + lens selector         │  - Callers       │
│              │                           │  - Properties    │
│              │                           │  - Health        │
├──────────────┴───────────────────────────┴─────────────────┤
│  Status Bar                                                │
└────────────────────────────────────────────────────────────┘
```

- **Left panel** (min 180px, max 320px): File tree + inline search + symbol list
- **Center panel** (min 400px): Sigma.js graph with ego-network, minimap, lens toolbar
- **Right panel** (min 250px, max 450px): Code preview (Shiki) + collapsible detail tabs
- **Synchronization**: Clicking anything (tree node, graph node, search result) updates all 3 panels
- **Lens selector** (toolbar): Call Graph | Structure | Heritage | Impact | Dead Code — filters `RelationshipType` on the graph

**Components migrated here**:
- `GraphExplorer.tsx` → center panel (refactored for ego-network)
- `GraphToolbar.tsx` → lens selector + zoom controls
- `FileTreeView.tsx` + `FilePreview.tsx` → left panel tree
- `DetailPanel.tsx` (Context/Code/Properties/Health tabs) → right panel
- `CodeInspectorPanel.tsx` → merged into right panel
- `NodeHoverCard.tsx` → kept as overlay
- `FeatureNavigator.tsx` → left panel community filter
- `SearchModal.tsx` → left panel inline search
- `ImpactView.tsx` → Impact lens on the graph (not separate view)
- `CypherQueryFAB.tsx` → command palette action
- `ProcessFlowModal.tsx` → kept as modal
- `ViewModeToggle.tsx` → toolbar toggle (graph/treemap)
- `TreemapView.tsx` → alternate view mode

### Mode 2 — Analyze

**Layout**: Dashboard with cards/tabs

```
┌─────────────┬──────────────────────────────────────────────┐
│  Sidebar     │  Active Analysis View                       │
│  (sub-nav)   │                                             │
│              │  [Hotspots | Coupling | Ownership |         │
│  - Hotspots  │   Coverage | Diagram | Report |             │
│  - Coupling  │   Code Health]                              │
│  - Ownership │                                             │
│  - Coverage  │                                             │
│  - Diagrams  │                                             │
│  - Report    │                                             │
│  - Health    │                                             │
├──────────────┴──────────────────────────────────────────────┤
│  Status Bar                                                │
└────────────────────────────────────────────────────────────┘
```

**Components migrated here**:
- `GitAnalyticsDashboard.tsx` → sub-nav router
- `HotspotsView.tsx`, `CouplingView.tsx`, `OwnershipView.tsx` → sub-views
- `CoverageView.tsx` → sub-view
- `DiagramView.tsx` → sub-view
- `ReportView.tsx` → sub-view
- `CodeHealthCard.tsx` → sub-view (expanded)
- `RepoDashboard.tsx` → **Overview** sub-view (default landing)

### Mode 3 — Chat

**Layout**: Full-screen conversational UI

```
┌────────────────────────────────────────────────────────────┐
│  Chat Header (model selector, context chips, settings)     │
├──────────────────────────────────┬─────────────────────────┤
│  Message History                 │  Source Panel            │
│                                  │  (expandable)           │
│  [user message]                  │                         │
│  [assistant + code blocks +      │  Referenced code         │
│   source citations]              │  snippets               │
│  [research plan viewer]          │  (click → Explorer)     │
│                                  │                         │
├──────────────────────────────────┴─────────────────────────┤
│  Input area + file/symbol/module filter buttons            │
└────────────────────────────────────────────────────────────┘
```

**Components migrated here**:
- `ChatPanel.tsx` → main view (refactored for full-screen)
- `ChatContextBar.tsx` → header chips
- `ChatSettings.tsx` → header settings
- `CodeSnippetRenderer.tsx` → inline in messages
- `FileFilterModal.tsx`, `SymbolFilterModal.tsx`, `ModuleFilterModal.tsx` → filter buttons
- `ResearchPlanViewer.tsx` → inline in messages
- `SourceReferences.tsx` → right source panel

### Mode 4 — Manage

**Layout**: Settings/admin single-column

```
┌────────────────────────────────────────────────────────────┐
│  Manage Header                                             │
├──────────────────────────────────────────────────────────────┤
│  Repositories    [list, analyze, open, remove]             │
│  Export          [graph, report, CSV]                       │
│  Documentation   [generated docs viewer]                   │
│  Settings        [theme, keyboard, LLM config]             │
└────────────────────────────────────────────────────────────┘
```

**Components migrated here**:
- `RepoManager.tsx` + `AnalyzeProgress.tsx` → Repositories section
- `ExportPanel.tsx` → Export section
- `DocsViewer.tsx` + `DocsNav.tsx` + `DocsContent.tsx` → Documentation section
- `SettingsModal.tsx` → Settings section (full page, not modal)

---

## Mode Switcher UI

Replace the 13-tab sidebar with a compact **mode bar** (left edge, 48px wide):

```
┌──┐
│ 🔍│  Explorer  (Ctrl+1)
│ 📊│  Analyze   (Ctrl+2)
│ 💬│  Chat      (Ctrl+3)
│ ⚙ │  Manage    (Ctrl+4)
│  │
│  │
│  │  (spacer)
│  │
│ 🔎│  Cmd+K     (command palette)
└──┘
```

- Icons only (no text), tooltip on hover
- Active mode highlighted with accent bar
- Command palette accessible from any mode via Cmd+K

---

## Ego-Network Graph Interaction

### Behavior spec

1. **On node click**: Center camera on node (200ms animation), compute N-hop neighborhood (default N=2, configurable 1-3 via toolbar slider)
2. **Ego nodes** (within N hops): Full opacity, full color, labels visible
3. **Context nodes** (outside N hops): 10% opacity, no labels, no hover interaction
4. **Context edges**: 5% opacity
5. **Edge bundling**: When >3 edges connect the same pair of nodes/groups, bundle into a single thick edge with count label
6. **Disconnected selected node** (0 edges, dead code): Show full graph at reduced density with a banner "This symbol has no connections. It may be dead code."

### Implementation approach (V1 — using existing Sigma.js capabilities)

1. **Backend**: Extend `get_subgraph` response to include `depth` per node
   - File: `crates/gitnexus-desktop/src/commands/graph.rs:172-237`
   - Add `depth: u32` field to `CytoNode` struct
   - During BFS traversal, track and return the hop distance from center
   
2. **Frontend nodeReducer**: Extend existing `nodeReducer` in `use-sigma.ts:142`
   - If node is in ego set (depth <= N): render at full opacity with type color
   - If node is outside ego set: render at 0.10 opacity via existing `dimColor()` function
   - Graduated opacity by depth: depth 1 = 100%, depth 2 = 80%, depth N = 60%, outside = 10%
   
3. **Frontend edgeReducer**: Same pattern for edges
   - Edges connecting two ego nodes: full opacity
   - Edges with one ego node: 30% opacity
   - Edges with zero ego nodes: 5% opacity

4. **Camera**: Use existing `camera.animate()` with `duration: 200` (currently 400ms at `use-sigma.ts:358`)

5. **Debouncing**: Add 150ms debounce on node selection to prevent rapid-click stacking

### Lenses (relationship type filters)

| Lens | Visible RelationshipTypes | Use case |
|------|--------------------------|----------|
| Call Graph | `Calls` | Who calls whom |
| Structure | `HasMethod`, `HasProperty`, `ContainedIn`, `DefinedIn` | Code organization |
| Heritage | `Extends`, `Implements` | Class hierarchy |
| Impact | `Calls`, `Imports`, `DependsOn` | Change impact |
| Dead Code | All, but highlight `is_dead_candidate=true` nodes in red | Find unused code |
| Tracing | All, but highlight `is_traced=true` nodes in green | StackLogger coverage |

---

## State Management Migration

### Current → New store schema

```typescript
// NEW app-store.ts

interface AppState {
  // Mode (replaces sidebarTab)
  mode: 'explorer' | 'analyze' | 'chat' | 'manage';
  analyzeView: 'overview' | 'hotspots' | 'coupling' | 'ownership' | 'coverage' | 'diagram' | 'report' | 'health';
  
  // Graph state (shared, survives mode switches)
  activeRepo: string | null;
  selectedNodeId: string | null;
  navigationHistory: HistoryEntry[];  // preserved across modes
  
  // Explorer-specific
  explorerLeftCollapsed: boolean;
  explorerRightCollapsed: boolean;
  activeLens: 'calls' | 'structure' | 'heritage' | 'impact' | 'dead-code' | 'tracing';
  egoDepth: 1 | 2 | 3;
  
  // Shared UI
  commandPaletteOpen: boolean;
  theme: 'dark' | 'light' | 'system';
  
  // Removed: sidebarTab, sidebarCollapsed, detailTab, searchOpen, searchQuery, zoomLevel
}
```

### Migration strategy

1. Add new fields (`mode`, `analyzeView`, `activeLens`, `egoDepth`) alongside existing ones
2. Create a `ModeRouter.tsx` component that renders the active mode
3. Migrate components one mode at a time (Explorer first, then Analyze, Chat, Manage)
4. Remove old `sidebarTab` references once all modes are complete
5. Delete `Sidebar.tsx` and `MainView.tsx` (replaced by mode bar + `ModeRouter`)

---

## Critical Architecture Decision: Sigma.js Lifecycle

**Decision**: Keep `GraphExplorer` always mounted, use CSS `visibility: hidden` + `pointer-events: none` when not in Explorer mode.

**Why**:
- Sigma.js initialization + ForceAtlas2 layout takes 20-30s for large graphs
- Destroying and recreating on every mode switch is unacceptable
- CSS hiding preserves the WebGL context, node positions, and camera state
- The Sigma canvas consumes ~50-100MB GPU memory when hidden — acceptable tradeoff

**Implementation**:
```tsx
// ModeRouter.tsx
<div style={{ visibility: mode === 'explorer' ? 'visible' : 'hidden',
              pointerEvents: mode === 'explorer' ? 'auto' : 'none',
              position: mode === 'explorer' ? 'relative' : 'absolute',
              inset: 0 }}>
  <ExplorerMode />
</div>
{mode === 'analyze' && <AnalyzeMode />}
{mode === 'chat' && <ChatMode />}
{mode === 'manage' && <ManageMode />}
```

---

## Design System Update

### New tokens (extending existing Obsidian Observatory)

```css
/* Glass morphism */
--glass-bg: rgba(13, 16, 23, 0.72);
--glass-border: rgba(148, 163, 194, 0.12);
--glass-blur: 16px;

/* Glow effects */
--glow-accent: 0 0 20px rgba(106, 161, 248, 0.15);
--glow-success: 0 0 16px rgba(74, 222, 128, 0.12);
--glow-danger: 0 0 16px rgba(251, 113, 133, 0.12);

/* Mode bar */
--mode-bar-width: 48px;
--mode-bar-bg: var(--bg-0);
--mode-icon-size: 20px;
--mode-active-bar: 3px solid var(--accent);

/* Panel constraints */
--panel-tree-min: 180px;
--panel-tree-max: 320px;
--panel-graph-min: 400px;
--panel-code-min: 250px;
--panel-code-max: 450px;

/* Ego-network */
--ego-full-opacity: 1.0;
--ego-depth2-opacity: 0.8;
--ego-depth3-opacity: 0.6;
--ego-context-opacity: 0.10;
--ego-context-edge-opacity: 0.05;

/* Transitions */
--transition-mode: 200ms ease-out;
--transition-ego-focus: 200ms ease-out;
```

### Typography (unchanged — Outfit, DM Sans, JetBrains Mono are already good)

### Color scheme (unchanged — Obsidian Observatory palette is solid, just add glass/glow tokens)

---

## Implementation Phases

### Phase 0 — Foundation (backend + infra) [~2 days]

| Step | File(s) | Change |
|------|---------|--------|
| 0.1 | `crates/gitnexus-desktop/src/commands/graph.rs` | Add `depth: u32` to `CytoNode`, track BFS depth in `get_subgraph` |
| 0.2 | `ui/src/stores/app-store.ts` | Add `mode`, `analyzeView`, `activeLens`, `egoDepth` fields alongside existing fields |
| 0.3 | `ui/src/index.css` | Add glass/glow/ego/mode-bar design tokens |
| 0.4 | `ui/src/hooks/use-sigma.ts` | Extend `nodeReducer`/`edgeReducer` for ego-network (depth-based opacity) |
| 0.5 | `ui/src/hooks/use-sigma.ts:358` | Reduce `focusNode` animation from 400ms → 200ms |
| 0.6 | `ui/src/hooks/use-sigma.ts` | Add 150ms debounce on node selection |

**Acceptance criteria**:
- `cargo test -p gitnexus-desktop` passes with new `depth` field
- Ego-network reducer dims nodes outside N-hop at 10% opacity
- Node focus animation completes in ≤200ms

### Phase 1 — Mode Router + Mode Bar [~1 day]

| Step | File(s) | Change |
|------|---------|--------|
| 1.1 | `ui/src/components/layout/ModeBar.tsx` | **New**: 48px left bar with 4 mode icons + Cmd+K |
| 1.2 | `ui/src/components/layout/ModeRouter.tsx` | **New**: Renders active mode, keeps Explorer always-mounted (hidden) |
| 1.3 | `ui/src/App.tsx` | Replace `Sidebar` + `MainView` with `ModeBar` + `ModeRouter` |
| 1.4 | `ui/src/hooks/use-keyboard-shortcuts.ts` | Remap Ctrl+1..4 to modes, keep Cmd+K for palette |

**Acceptance criteria**:
- 4 modes switchable via mode bar and Ctrl+1..4
- Mode switch completes rendering in <200ms
- Explorer's Sigma.js canvas preserves state across mode switches (test: select node in Explorer → switch to Analyze → switch back → node still selected and centered)

### Phase 2 — Explorer Mode (Sourcetrail layout) [~3 days]

| Step | File(s) | Change |
|------|---------|--------|
| 2.1 | `ui/src/components/explorer/ExplorerMode.tsx` | **New**: 3-panel layout with `react-resizable-panels` |
| 2.2 | `ui/src/components/explorer/ExplorerLeftPanel.tsx` | **New**: Tree + search + symbol list (merge FileTreeView + SearchModal) |
| 2.3 | `ui/src/components/explorer/ExplorerRightPanel.tsx` | **New**: Code preview + detail tabs (merge CodeInspectorPanel + DetailPanel) |
| 2.4 | `ui/src/components/explorer/LensSelector.tsx` | **New**: Toolbar dropdown to select active lens |
| 2.5 | `ui/src/components/explorer/EgoDepthSlider.tsx` | **New**: 1-2-3 slider for ego-network depth |
| 2.6 | `ui/src/components/graph/GraphExplorer.tsx` | Refactor: remove self-contained toolbar (moved to LensSelector), integrate ego-network |
| 2.7 | Synchronization logic | Wire: tree click → setSelectedNodeId → graph centers + code panel loads |

**Acceptance criteria**:
- Clicking a symbol in the tree panel centers the graph AND loads source code in <300ms
- Clicking a node in the graph highlights the file in the tree AND loads source code in <300ms
- Lens selector filters visible edges by `RelationshipType`
- Left panel min 180px / max 320px, right panel min 250px / max 450px
- Panels collapsible via drag or keyboard (Cmd+B left, Cmd+Shift+B right)

### Phase 3 — Analyze Mode [~2 days]

| Step | File(s) | Change |
|------|---------|--------|
| 3.1 | `ui/src/components/analyze/AnalyzeMode.tsx` | **New**: Left sub-nav + main content area |
| 3.2 | `ui/src/components/analyze/AnalyzeNav.tsx` | **New**: Sub-navigation (7 views) |
| 3.3 | Move existing views | Move `HotspotsView`, `CouplingView`, `OwnershipView`, `CoverageView`, `DiagramView`, `ReportView`, `CodeHealthCard` into `analyze/` |
| 3.4 | `ui/src/components/analyze/OverviewView.tsx` | **New**: Merge `RepoDashboard` as default landing |

**Acceptance criteria**:
- All 7 analysis views render correctly
- Sub-nav highlights active view
- Data loads from existing Tauri IPC commands (no backend changes)
- Overview shows repo stats + code health gauge

### Phase 4 — Chat Mode [~2 days]

| Step | File(s) | Change |
|------|---------|--------|
| 4.1 | `ui/src/components/chat/ChatMode.tsx` | **New**: Full-screen layout with optional source panel |
| 4.2 | `ui/src/components/chat/ChatMode.tsx` | Persist messages to Zustand/localStorage (cap 200 per repo) |
| 4.3 | Cross-mode navigation | Source reference click → `setMode('explorer')` + `setSelectedNodeId(ref.id)` |
| 4.4 | No-LLM empty state | Show setup prompt when no LLM configured |

**Acceptance criteria**:
- Chat messages persist across mode switches
- Source reference click navigates to Explorer with symbol centered in <500ms
- Empty state shows LLM setup prompt (not blank screen)
- Research plans and code snippets render correctly at full width

### Phase 5 — Manage Mode [~1 day]

| Step | File(s) | Change |
|------|---------|--------|
| 5.1 | `ui/src/components/manage/ManageMode.tsx` | **New**: Single-column sections layout |
| 5.2 | Move components | Move `RepoManager`, `ExportPanel`, `DocsViewer` + nav + content, `SettingsModal` → full page |

**Acceptance criteria**:
- All manage functions accessible
- Settings is a full page (not modal)
- Repo analysis progress shows correctly

### Phase 6 — Cleanup + Polish [~2 days]

| Step | Change |
|------|--------|
| 6.1 | Delete `Sidebar.tsx`, `MainView.tsx`, old tab-based routing code |
| 6.2 | Remove `sidebarTab` from app-store, migrate all 32 file references |
| 6.3 | Update `CommandPalette.tsx` to index all modes, views, and actions |
| 6.4 | Apply glass-morphism tokens consistently across all panels |
| 6.5 | Update keyboard shortcut help in command palette |
| 6.6 | Test all responsive breakpoints (compact <900px, narrow <700px) |
| 6.7 | Verify all 32 Tauri IPC commands still function |

**Acceptance criteria**:
- No references to old `sidebarTab` type
- Command palette finds all features by name
- Glass-morphism applied consistently (no per-component hacks)
- `cargo test -p gitnexus-desktop` passes
- All 32 IPC commands return correct data

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Sigma.js canvas destroyed on mode switch → 20-30s relayout | Critical | Always-mount with CSS hiding (Phase 1, step 1.2) |
| ForceAtlas2 disorientation on ego-network refocus | High | Use incremental layout: freeze non-ego nodes, only layout ego nodes. Fall back to camera animation only if layout is too slow |
| `get_subgraph` BFS fans out to 1000+ nodes for hub symbols | High | Hard cap at 500 ego nodes, "Show more" button for additional hops |
| State migration breaks 32 files referencing `sidebarTab` | High | Add new fields alongside old, migrate incrementally, delete old in Phase 6 |
| Glass-morphism polish spiral (infinite CSS tweaking) | Medium | Strict design token system defined in Phase 0, apply mechanically |
| Chat messages lost on mode switch | Medium | Persist to Zustand store + localStorage in Phase 4 |
| Rapid node clicks stack IPC requests | Medium | 150ms debounce + cancel in-flight requests (Phase 0) |
| `react-resizable-panels` nested groups issue | Low | Test early in Phase 2, fall back to flexbox if needed |
| Keyboard shortcut conflicts between modes | Low | Audit and remap in Phase 1 |

---

## Verification Steps

1. **Mode switching**: Switch between all 4 modes 10 times rapidly → no crashes, no blank screens, <200ms each
2. **Ego-network**: Select a hub node (e.g., a base Service class with 50+ callees) → graph centers in <200ms, only N-hop nodes at full opacity, rest at 10%
3. **3-panel sync**: In Explorer, click a method in the tree → graph centers on it AND code panel shows its source
4. **Lens switching**: Toggle between Call Graph and Structure lens → graph shows only relevant edge types
5. **Chat persistence**: Send 5 messages in Chat → switch to Explorer → switch back → all 5 messages visible
6. **Cross-mode nav**: In Chat, click a source reference → app switches to Explorer with that symbol centered
7. **Backend regression**: Run `cargo test -p gitnexus-desktop` → all pass
8. **Large graph**: Load a repo with 1000+ nodes → Explorer mode renders in <3s, ego-network transitions in <300ms
9. **Responsive**: Resize window to 800px wide → panels collapse gracefully, mode bar remains visible
10. **Command palette**: Open Cmd+K → type "hotspots" → navigates to Analyze > Hotspots

---

## Component Migration Map

| Current Component | Destination | Notes |
|------------------|-------------|-------|
| `Sidebar.tsx` | **DELETE** | Replaced by ModeBar |
| `MainView.tsx` | **DELETE** | Replaced by ModeRouter |
| `CommandBar.tsx` | **DELETE** | Merged into mode-specific headers |
| `GraphExplorer.tsx` | Explorer (center) | Refactor for ego-network |
| `GraphToolbar.tsx` | Explorer (LensSelector) | Split into lens + ego controls |
| `ViewModeToggle.tsx` | Explorer (toolbar) | Keep |
| `TreemapView.tsx` | Explorer (alternate view) | Keep |
| `FeatureNavigator.tsx` | Explorer (left panel) | Keep |
| `NodeHoverCard.tsx` | Explorer (overlay) | Keep |
| `ProcessFlowModal.tsx` | Explorer (modal) | Keep |
| `CypherQueryFAB.tsx` | Command palette action | Refactor |
| `FileTreeView.tsx` | Explorer (left panel) | Merge into ExplorerLeftPanel |
| `FilePreview.tsx` | Explorer (right panel) | Merge into ExplorerRightPanel |
| `DetailPanel.tsx` | Explorer (right panel) | Merge into ExplorerRightPanel |
| `CodeInspectorPanel.tsx` | Explorer (right panel) | Merge |
| `CodePanel.tsx` | Explorer (right panel) | Merge |
| `ImpactView.tsx` | Explorer (Impact lens) | Refactor as lens overlay |
| `SearchModal.tsx` | Explorer (left panel search) | Inline |
| `LayersTab.tsx` | Explorer (right panel tab) | Keep |
| `CodeHealthCard.tsx` | Analyze (Health view) | Move |
| `GitAnalyticsDashboard.tsx` | Analyze (router) | Move |
| `HotspotsView.tsx` | Analyze (sub-view) | Move |
| `CouplingView.tsx` | Analyze (sub-view) | Move |
| `OwnershipView.tsx` | Analyze (sub-view) | Move |
| `CoverageView.tsx` | Analyze (sub-view) | Move |
| `DiagramView.tsx` | Analyze (sub-view) | Move |
| `ReportView.tsx` | Analyze (sub-view) | Move |
| `RepoDashboard.tsx` | Analyze (Overview) | Move |
| `ChatPanel.tsx` | Chat (main) | Refactor for full-screen |
| `ChatContextBar.tsx` | Chat (header) | Move |
| `ChatSettings.tsx` | Chat (header) | Move |
| `CodeSnippetRenderer.tsx` | Chat (inline) | Keep |
| `FileFilterModal.tsx` | Chat (filter) | Keep |
| `SymbolFilterModal.tsx` | Chat (filter) | Keep |
| `ModuleFilterModal.tsx` | Chat (filter) | Keep |
| `ResearchPlanViewer.tsx` | Chat (inline) | Keep |
| `SourceReferences.tsx` | Chat (source panel) | Refactor |
| `RepoManager.tsx` | Manage (repos) | Move |
| `AnalyzeProgress.tsx` | Manage (repos) | Move |
| `ExportPanel.tsx` | Manage (export) | Move |
| `DocsViewer.tsx` | Manage (docs) | Move |
| `DocsNav.tsx` | Manage (docs) | Move |
| `DocsContent.tsx` | Manage (docs) | Move |
| `SettingsModal.tsx` | Manage (settings page) | Refactor modal → page |
| `CommandPalette.tsx` | Shared (global) | Update to index modes/views |
| `StatusBar.tsx` | Shared (global) | Keep |
| `PanelSeparator.tsx` | Shared (global) | Keep |
| `ErrorBoundary.tsx` | Shared (global) | Keep |
| `Tooltip.tsx` | Shared (global) | Keep |
| `LoadingOrbs.tsx` | Shared (global) | Keep |
| `Toaster.tsx` | Shared (global) | Keep |
| `Breadcrumb.tsx` | Shared (global) | Keep |
| `NodeIcon.tsx` | Shared (global) | Keep |
| `motion.tsx` | Shared (global) | Keep |

---

## New Files Summary

| File | Phase | Purpose |
|------|-------|---------|
| `components/layout/ModeBar.tsx` | 1 | 48px mode switcher bar |
| `components/layout/ModeRouter.tsx` | 1 | Mode rendering + Explorer always-mount |
| `components/explorer/ExplorerMode.tsx` | 2 | 3-panel Sourcetrail layout |
| `components/explorer/ExplorerLeftPanel.tsx` | 2 | Tree + search + symbols |
| `components/explorer/ExplorerRightPanel.tsx` | 2 | Code + details |
| `components/explorer/LensSelector.tsx` | 2 | Relationship type filter |
| `components/explorer/EgoDepthSlider.tsx` | 2 | 1-2-3 hop depth control |
| `components/analyze/AnalyzeMode.tsx` | 3 | Dashboard layout |
| `components/analyze/AnalyzeNav.tsx` | 3 | Sub-navigation |
| `components/analyze/OverviewView.tsx` | 3 | Dashboard landing |
| `components/chat/ChatMode.tsx` | 4 | Full-screen chat layout |
| `components/manage/ManageMode.tsx` | 5 | Settings/admin layout |

---

## Out of Scope (V2)

- Animated ego-network transitions (nodes flying in/out) — V1 uses instant opacity change
- 3D graph mode (Graphia-style) — would require Three.js or Sigma 3D extension
- Collaborative features (shared graph sessions)
- Plugin/extension system
- Custom themes beyond dark/light
- Edge bundling visualization (V1 uses standard edges)
- Natural language graph queries (Neo4j Bloom-style "Show me callers of X")
