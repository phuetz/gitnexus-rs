# PRD — Modernize GitNexus

## Objective
Faire de GitNexus une plateforme de compréhension de code locale, agentique et desktop-first capable de rivaliser avec Sourcegraph, CodeSee, CodeScene et NDepend, tout en capitalisant sur les forces uniques du dépôt et de l’écosystème GitHub `phuetz`.

## Positioning
GitNexus doit devenir:
- le meilleur outil local pour comprendre rapidement une codebase complexe
- le meilleur assistant de modernisation de code legacy .NET
- un cockpit visuel + conversationnel pour humains et agents

## Strategic principles
1. Local-first before cloud-first
2. Workflow-first before feature-first
3. Fast onboarding before deep customization
4. Polished desktop before sprawling surfaces
5. Evidence-driven quality before vanity AI

## Competitive north star
- Sourcegraph-level “deep search” workflows
- CodeSee-level visual code understanding and review maps
- CodeScene/NDepend-level hotspot and architecture governance
- `code-buddy`-level AI ergonomics
- `FileCommander`-level desktop polish

## 30-step Ralph roadmap

### Phase 1 — Product clarity and onboarding
1. Clarify the one-line value proposition across README, UI hero, CLI help, and website copy.
2. Redesign first-run onboarding around 3 explicit journeys: explore, audit, modernize.
3. Add “Open sample repo / demo workspace” mode for instant value without indexing.
4. Add onboarding checklists per persona: architect, maintainer, AI-agent user, legacy .NET owner.
5. Add guided first-use tours in desktop for graph, chat, docs, and impact analysis.

### Phase 2 — Desktop experience and workflow polish
6. Make the desktop app feel like a primary product, not a shell around features.
7. Add persistent workspace/session concepts with resumable context.
8. Add action-oriented dashboards instead of raw views: “what changed?”, “where to start?”, “what is risky?”.
9. Add richer keyboard-first navigation inspired by IDEs and polished desktop tools.
10. Add shareable exports/screenshots/reports from every major surface.

### Phase 3 — AI workflows and research ergonomics
11. Upgrade chat from Q&A to guided “deep understanding” and “change planning” workflows.
12. Add explicit research templates: explain feature, trace incident, estimate refactor, prepare migration.
13. Add multi-step traceable answer cards with evidence, confidence, and next recommended actions.
14. Add “apply to graph / open files / run impact” follow-ups directly from chat answers.
15. Add a unified search bar combining natural language, symbol search, and graph query assist.

### Phase 4 — Graph, map, and architecture differentiation
16. Add task-focused graph presets: onboarding, architecture, hotspots, data-flow, dead code, legacy web stack.
17. Add collapsible architecture maps with richer semantic layers and domain boundaries.
18. Add review maps and “change blast radius cards” for planned modifications.
19. Add ownership / hotspots / coupling overlays directly on graph and docs flows.
20. Add custom labels, annotations, saved views, and shareable “guided maps” inspired by CodeSee.

### Phase 5 — Governance, health, and modernization
21. Introduce architecture rules / quality gates with visible pass-fail status.
22. Add modernization scorecards for legacy systems: tracing, layering, dead code, coupling, external dependencies.
23. Add prioritized refactoring plans based on hotspots + impact + ownership + dead code.
24. Add issue-oriented health dashboards for engineering leads and maintainers.
25. Add migration assistants for ASP.NET MVC / EF6 / jQuery/Telerik modernization paths.

### Phase 6 — Distribution, growth, and market readiness
26. Improve packaging/distribution: signed binaries, installers, Winget, Homebrew, desktop releases.
27. Add example datasets, benchmark repos, and public demo material.
28. Add a public comparison matrix vs Sourcegraph / CodeSee / CodeScene / NDepend.
29. Add analytics/telemetry architecture that stays privacy-safe and opt-in.
30. Define a launch narrative and release cadence around one flagship workflow: “understand and modernize legacy systems with AI”.

## Execution workstreams
- Workstream A: UX and onboarding
- Workstream B: AI/deep search workflows
- Workstream C: graph/maps/review UX
- Workstream D: governance and quality
- Workstream E: packaging, benchmarks, and GTM

## Initial staffing guidance
- `ralph` lane:
  - sequential modernization with verification after each workstream
  - best for risky UI/desktop/backend integration changes
- `team` lane:
  - parallel lanes for onboarding, chat UX, graph UX, packaging, docs
  - best after workstream boundaries are fixed

## Available agent types roster
- `planner`
- `architect`
- `critic`
- `executor`
- `test-engineer`
- `verifier`
- `writer`
- `researcher`
- `designer`

## Suggested reasoning levels by lane
- Product framing / roadmap: high
- UI/UX architecture: medium-high
- Code implementation: medium-high
- Testing / verification: medium
- Packaging / CI: medium

## ADR
- Decision:
  - Modernize GitNexus around workflow-led understanding + modernization, not just more raw analysis features.
- Drivers:
  - Strong technical core already exists.
  - Competitors win on usability, guided workflows, and market narrative.
  - The `phuetz` repo ecosystem shows reusable strengths in AI ergonomics and desktop polish.
- Alternatives considered:
  - Stay purely technical / CLI-first.
  - Pivot to cloud-first collaboration.
  - Narrow only to ASP.NET legacy analysis.
- Why chosen:
  - Keeps the strongest differentiators while widening the product surface where competitors currently win.
- Consequences:
  - Requires tighter UX, better packaging, and stronger test discipline.
  - Increases product scope; must be phased carefully.
- Follow-ups:
  - Execute the 30-step roadmap in bounded slices with hard verification gates.
