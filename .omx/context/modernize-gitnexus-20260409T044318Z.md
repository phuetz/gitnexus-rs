# Context Snapshot — Modernize GitNexus

## Task statement
Moderniser GitNexus pour en faire une application leader du marché, en le comparant aux concurrents actuels et aux dépôts GitHub de `phuetz`.

## Desired outcome
- Transformer GitNexus d’un excellent moteur local + desktop technique en une plateforme de compréhension de code compétitive sur:
  - onboarding
  - UX desktop
  - workflows IA
  - collaboration
  - qualité / gouvernance
  - distribution / adoption
- Définir une feuille de route exécutable en 30 étapes.

## Known facts / evidence
- Le socle technique actuel est solide: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npm run lint`, `npm run build`, `npm run test:e2e`, `npm audit --audit-level=moderate` passent.
- GitNexus a déjà des points différenciants forts:
  - graphe de connaissances local
  - support legacy ASP.NET MVC / EF6 / Telerik
  - desktop app Tauri/React
  - recherche hybride, analyse d’impact, docs HTML
  - MCP server pour agents
- Les plus gros chunks UI restants concernent surtout `cytoscape`, `wasm`, `cpp`, `katex`.

## Competitor benchmark
- Sourcegraph:
  - se positionne comme “code understanding platform”
  - met en avant Deep Search, Code Search, Batch Changes, Monitors, Insights, sécurité enterprise
  - supporte des workflows multi-repo / multi-code-host / enterprise
- CodeSee:
  - propose des codebase maps collaboratives
  - review maps dans les PRs
  - labels, ownership, tours, annotation visuelle, partage
- CodeScene:
  - pousse hotspot analysis, code health, quality gates, behavioral code analysis
- NDepend:
  - apporte diagrammes de dépendances, navigation architecture, règles structurelles, export de diagrammes

## GitHub repo comparison
- `phuetz/code-buddy`:
  - force: expérience terminal-first, multi-provider AI agent, orientation product claire
  - GitNexus doit reprendre cette clarté dans l’UX “chat / ask / automation”
- `phuetz/open-cowork`:
  - force: logique de cowork agentique desktop
  - GitNexus doit intégrer des workflows guidés, pas seulement des vues analytiques
- `phuetz/FileCommander`:
  - force: produit desktop cross-platform orienté usage quotidien
  - GitNexus doit emprunter le niveau de finition desktop: navigation, polish, raccourcis, feedback
- `phuetz/TurboQuant`:
  - force: dashboards / lisibilité de données / posture produit
  - GitNexus doit mieux transformer ses métriques en tableaux de bord exécutifs

## Constraints
- Préserver la proposition “local-first, privacy-first, AI-agent-ready”.
- Ne pas diluer la spécialisation ASP.NET legacy qui reste un vrai moat.
- Garder une base installable et maintenable pour Windows en priorité.
- Moderniser sans casser le socle CLI/MCP.

## Unknowns / open questions
- Quelle part du marché viser en premier:
  - teams enterprise .NET legacy
  - développeurs IA/agents
  - maintainers open source
- Faut-il faire du SaaS plus tard ou rester 100% local / self-hosted?
- Quelle hiérarchie produit entre desktop, CLI et MCP?
- Quelle stratégie de distribution grand public: binaire, winget, brew, Docker, extensions?

## Likely codebase touchpoints
- `crates/gitnexus-desktop/ui/src/**/*`
- `crates/gitnexus-desktop/src/**/*`
- `crates/gitnexus-mcp/src/**/*`
- `crates/gitnexus-cli/src/**/*`
- `README.md`
- `README.fr.md`
- `.github/workflows/ci.yml`

## Suggested modernization thesis
Le chemin pour devenir “leader” n’est pas d’ajouter des features isolées, mais de fusionner:
- la profondeur d’analyse locale de GitNexus
- la clarté workflow d’un `code-buddy`
- le polish desktop d’un `FileCommander`
- les maps/review workflows d’un CodeSee
- les quality gates d’un CodeScene/NDepend
- et la recherche agentique type Sourcegraph Deep Search
