# Test Spec — Modernize GitNexus

## Goal
Définir les preuves de réussite pour la modernisation produit de GitNexus.

## Success criteria by workstream

### A. Onboarding and UX
- New users can reach value in under 3 minutes on a fresh install.
- Desktop first-run clearly exposes 3 workflows: explore, audit, modernize.
- Demo/sample mode works without local indexing.
- Keyboard-first navigation remains fully functional.

### B. AI and deep research workflows
- Chat supports structured, reproducible workflows beyond plain Q&A.
- Every multi-step answer includes evidence, source references, and status.
- Filters, plans, and follow-up actions remain deterministic and testable.
- Provider configuration remains secure; secrets never persist to disk.

### C. Graph and map experience
- The main graph remains responsive on large repos.
- Saved views / task presets load correctly.
- Impact overlays and map annotations remain stable after layout changes.
- Graph exports and review-map style outputs are reproducible.

### D. Governance and quality
- Health dashboards aggregate hotspots, ownership, coupling, and structural rules.
- Architecture rule failures are visible and testable.
- Modernization scorecards are reproducible from the same repo snapshot.

### E. Distribution and readiness
- Desktop and CLI packaging works on Windows at minimum.
- CI validates lint, build, tests, audit, and browser E2E.
- Public-facing docs and positioning remain aligned with actual product capabilities.

## Test matrix

### Unit
- config serialization/deserialization
- secret redaction / env secret hydration
- graph reducers and overlays
- impact / coverage / health calculations
- architecture rule evaluation
- roadmap/export formatting helpers

### Integration
- CLI analyze -> snapshot -> desktop load
- desktop repo open -> graph render -> detail panel
- MCP HTTP auth / path exposure / bounded limits
- docs generation -> docs viewer -> chat in docs mode
- modernization scorecard generation from indexed repo

### E2E browser
- welcome screen
- open repo
- command palette
- chat guard when no API key configured
- docs empty state
- theme switching
- future:
  - saved views
  - review maps
  - modernization dashboard
  - report export

### E2E desktop/Tauri
- open desktop binary
- analyze local repo from picker
- open graph and run impact flow
- open docs and ask a code question
- export report/doc artifact

### Observability / performance
- desktop boot time on browser mock mode
- time-to-first-graph-render
- time-to-first-chat-response
- graph interaction FPS on representative repo sizes
- memory ceiling for graph and docs workflows

## Verification gates

### Gate 1 — Quality floor
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `npm run lint`
- `npm run build`
- `npm audit --audit-level=moderate`
- `npm run test:e2e`

### Gate 2 — Product proof
- at least one guided onboarding workflow complete
- at least one deep research workflow complete
- at least one governance dashboard complete
- at least one modernization flow complete

### Gate 3 — Release proof
- CI green on all required jobs
- updated public docs and comparison narrative
- release checklist for desktop + CLI packaging

## Recommended implementation order
1. Onboarding / UX clarity
2. AI workflow cards and deep research polish
3. graph/maps/review UX
4. governance dashboards and modernization scorecards
5. packaging and GTM material
