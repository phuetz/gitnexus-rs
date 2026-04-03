// ─── GitNexus i18n — Lightweight translation system ───
// No external dependency. Supports FR and EN with tooltip descriptions.

export type Locale = "fr" | "en";

export interface TranslationEntry {
  /** The displayed label / text */
  label: string;
  /** Optional tooltip explaining the feature */
  tip?: string;
}

type TranslationValue = string | TranslationEntry;

// ─── Translation dictionaries ───

const en = {
  // ── Sidebar ──
  "sidebar.workspace": "WORKSPACE",
  "sidebar.explore": "EXPLORE",
  "sidebar.analysis": "ANALYSIS",
  "sidebar.overview": "Overview",
  "sidebar.repositories": "Repositories",
  "sidebar.fileExplorer": "File Explorer",
  "sidebar.graphExplorer": "Graph Explorer",
  "sidebar.impactAnalysis": "Impact Analysis",
  "sidebar.documentation": "Documentation",
  "sidebar.export": "Export",
  "sidebar.gitAnalytics": "Git Analytics",
  "sidebar.chat": "Chat",
  "sidebar.coverage": "Coverage",
  "sidebar.diagram": "Diagrams",
  "sidebar.report": "Health Report",
  "sidebar.settings": "Settings",
  "sidebar.collapse": { label: "Collapse sidebar", tip: "Toggle the sidebar visibility (Ctrl+B)" },
  "sidebar.expand": { label: "Expand sidebar", tip: "Show the navigation sidebar (Ctrl+B)" },

  // ── Repo Manager ──
  "repos.title": "Repositories",
  "repos.indexed": "indexed",
  "repos.repository": "repository",
  "repos.repositories": "repositories",
  "repos.analyzeProject": { label: "Analyze Project", tip: "Select a folder to scan and build its knowledge graph" },
  "repos.refresh": { label: "Refresh", tip: "Reload the list of indexed repositories" },
  "repos.noRepos": "No repositories indexed",
  "repos.noReposDesc": "Start by analyzing a project to explore its code graph.",
  "repos.reindex": { label: "Re-index", tip: "Re-scan this project to update its knowledge graph" },
  "repos.generateWiki": { label: "Generate Wiki", tip: "Auto-generate a wiki from the codebase structure" },
  "repos.generateDocs": { label: "Generate Docs", tip: "Create technical documentation from code analysis" },
  "repos.generateAgents": { label: "Generate AGENTS.md", tip: "Generate an AI agent context file for this repo" },
  "repos.generateAll": { label: "Generate All", tip: "Run all generators (Wiki + Docs + AGENTS.md)" },
  "repos.onboarding.step1.title": "1. Choose a project",
  "repos.onboarding.step1.desc": "Click \"Analyze Project\" to select a folder on your machine.",
  "repos.onboarding.step2.title": "2. Automatic analysis",
  "repos.onboarding.step2.desc": "GitNexus scans your files, resolves imports, and builds a knowledge graph.",
  "repos.onboarding.step3.title": "3. Explore",
  "repos.onboarding.step3.desc": "Navigate through the interactive graph, browse files, and analyze impacts.",
  "repos.files": "files",
  "repos.nodes": "nodes",
  "repos.edges": "edges",
  "repos.communities": "communities",
  "repos.loading": "Loading repositories...",
  "repos.error": "Failed to load repositories",
  "repos.processing": "Processing...",
  "repos.opening": "Opening…",

  // ── Coverage ──
  "coverage.loading": "Loading coverage data...",
  "coverage.totalMethods": "Total Methods",
  "coverage.deadCode": "Dead Code",
  "coverage.coverageLabel": "Coverage",
  "coverage.deadCandidates": "Dead Code Candidates",
  "coverage.noDead": "No dead code candidates found",
  "coverage.method": "Method",
  "coverage.class": "Class",
  "coverage.file": "File",

  // ── Diagram ──
  "diagram.placeholder": "Enter class, controller, or service name...",
  "diagram.generate": "Generate",
  "diagram.generating": "Generating diagram...",
  "diagram.copied": "Copied!",
  "diagram.copyMermaid": "Copy Mermaid",
  "diagram.noDiagram": "No diagram generated. Symbol may not exist.",

  // ── Report ──
  "report.temporalCoupling": "Temporal Coupling (Top 10)",
  "report.distributedFiles": "Distributed Files (Top 10)",

  // ── Graph Explorer ──
  "graph.packages": { label: "Packages", tip: "Show top-level modules and packages" },
  "graph.modules": { label: "Modules", tip: "Show module-level symbols (structs, traits, classes)" },
  "graph.symbols": { label: "Symbols", tip: "Show all symbols (functions, types, constants)" },
  "graph.nodesCount": "nodes",
  "graph.edgesCount": "edges",
  "graph.fitView": { label: "Fit view", tip: "Zoom to fit all nodes in view (F)" },
  "graph.layout": { label: "Layout", tip: "Change the graph layout algorithm" },
  "graph.contextMenu.goToDefinition": { label: "Go to Definition", tip: "Jump to the source file of this symbol" },
  "graph.contextMenu.findReferences": { label: "Find All References", tip: "Show all places that reference this symbol" },
  "graph.contextMenu.expandNeighbors": { label: "Expand Neighbors", tip: "Reveal connected nodes" },
  "graph.contextMenu.hideNode": { label: "Hide Node", tip: "Remove this node from the current view" },
  "graph.contextMenu.copyName": { label: "Copy Name", tip: "Copy the symbol name to clipboard" },
  "graph.contextMenu.copyFilePath": { label: "Copy File Path", tip: "Copy the source file path to clipboard" },
  "graph.minimap.toggle": { label: "Minimap", tip: "Toggle the navigation minimap" },
  "graph.legend": { label: "Legend", tip: "Show/hide the color legend for node types" },
  "graph.noData": "No graph data available",
  "graph.analyzeFirst": "Analyze a repository first",
  "graph.computingLayout": "Computing layout...",
  "graph.backToFull": "Back to full graph",
  "graph.showingTopNodes": "Showing top {0} nodes by importance. Double-click a node to explore its neighborhood.",
  "graph.exportPng": "Export graph as PNG (Ctrl+E)",
  "graph.processFlows": "Process Flows",
  "graph.edgeFilters": "Edges",
  "graph.keyboardShortcuts": "Keyboard Shortcuts",
  "graph.viewImpact": "View Impact",
  "graph.clearImpact": "Clear Impact",
  "graph.impactOverlay": "Impact Overlay",
  "graph.shortcut.goToSymbol": "Go to symbol",
  "graph.shortcut.exportPng": "Export graph PNG",
  "graph.shortcut.screenshot": "Screenshot",
  "graph.shortcut.zoomInOutFit": "Zoom in/out/fit",
  "graph.shortcut.navigateBackForward": "Navigate back/forward",
  "graph.shortcut.clearSelection": "Clear selection",
  "graph.shortcut.focusSubgraph": "Focus subgraph",
  "graph.shortcut.toggleHelp": "Toggle this help",

  // ── File Explorer ──
  "files.title": "Files",
  "files.lines": "lines",
  "files.backToTree": { label: "Back", tip: "Return to the file tree" },

  // ── Detail Panel ──
  "detail.context": "Context",
  "detail.code": "Code",
  "detail.codeProperties": "Properties",
  "detail.layers": "Layers",
  "detail.health": "Health",
  "detail.callers": "CALLERS",
  "detail.callees": "CALLEES",
  "detail.community": "COMMUNITY",
  "detail.members": "members",
  "detail.cohesion": "Cohesion",
  "detail.noSelection": "Select a node in the graph to see its details here.",

  // ── Impact Analysis ──
  "impact.title": { label: "Impact Analysis", tip: "Understand how changes to a symbol ripple through your codebase" },
  "impact.placeholder": "Search symbol to analyze...",
  "impact.searchAndSelect": "Search and select a symbol to analyze its blast radius",
  "impact.analyzingImpact": "Analyzing impact...",
  "impact.upstream": "Upstream (callers)",
  "impact.downstream": "Downstream (callees)",
  "impact.affectedFiles": "Affected files",
  "impact.statUpstream": "Upstream",
  "impact.statDownstream": "Downstream",
  "impact.statFiles": "Files",
  "impact.directionUpstream": "Upstream",
  "impact.directionBoth": "Both",
  "impact.directionDownstream": "Downstream",
  "impact.impactDistribution": "Impact Distribution",
  "impact.moreItems": "+{0} more",

  // ── Command Bar ──
  "search.placeholder": "Search symbols, files, classes...",
  "search.shortcut": "Ctrl K",
  "search.navigate": "Navigate",
  "search.open": "Open",
  "search.close": "Close",
  "search.noResults": "No results found",
  "search.startTyping": "Start typing to search...",
  "search.ariaLabel": "Open search for symbols",

  // ── Command Bar breadcrumb tabs ──
  "commandBar.tab.repos": "Repositories",
  "commandBar.tab.search": "Search",
  "commandBar.tab.files": "Files",
  "commandBar.tab.graph": "Graph Explorer",
  "commandBar.tab.impact": "Impact Analysis",
  "commandBar.tab.docs": "Documentation",

  // ── Manage ──
  "manage.title": "Manage",
  "manage.repositories": "Repositories",
  "manage.export": "Export",
  "manage.documentation": "Documentation",
  "manage.settings": "Settings",
  "manage.theme.dark": "Dark",
  "manage.theme.light": "Light",
  "manage.theme.system": "System",

  // ── Settings ──
  "settings.title": "Settings",
  "settings.language": { label: "Language", tip: "Choose the display language for the interface" },
  "settings.theme": { label: "Theme", tip: "Switch between light and dark mode" },
  "settings.shortcuts": { label: "Keyboard Shortcuts", tip: "View all available keyboard shortcuts" },
  "settings.soon": "Soon",

  // ── Status Bar ──
  "status.noRepo": "No repository selected",
  "status.view": "View",
  "status.packageLevel": "Package level",
  "status.moduleLevel": "Module level",
  "status.symbolLevel": "Symbol level",
  "status.browseSourceTree": "Browse: Source tree",
  "status.modeDependencyAnalysis": "Dependency analysis",
  "status.docsWikiViewer": "Wiki viewer",
  "status.mode": "Mode",
  "status.docs": "Docs",
  "status.browse": "Browse",

  // ── Analyze Progress ──
  "analyze.analyzing": "Analyzing",
  "analyze.analysisFailed": "Analysis failed",
  "analyze.analysisComplete": "Analysis complete",
  "analyze.analyzingRepo": "Analyzing {name}...",
  "analyze.analyzeProject": "Analyze Project",
  "analyze.phase.idle": "Idle",
  "analyze.phase.extracting": "Extracting",
  "analyze.phase.structure": "Scanning files",
  "analyze.phase.parsing": "Parsing AST",
  "analyze.phase.imports": "Resolving imports",
  "analyze.phase.calls": "Analyzing calls",
  "analyze.phase.heritage": "Class hierarchy",
  "analyze.phase.communities": "Detecting communities",
  "analyze.phase.processes": "Tracing processes",
  "analyze.phase.enriching": "Enriching",
  "analyze.phase.complete": "Complete",
  "analyze.phase.error": "Error",
  "analyze.files": "files",
  "analyze.nodes": "nodes",

  // ── File Explorer ──
  "files.searchPlaceholder": "Search files...",
  "files.searchFiles": "Search files",
  "files.clearSearch": "Clear search",
  "files.noMatchingFiles": "No files found",
  "files.matchingFiles": "{0} file(s) found",
  "files.errorLoadingTree": "Error loading file tree",
  "files.noFilesFound": "No files found",
  "files.selectFileToPreview": "Select a file to preview its contents",
  "files.closePreview": "Close preview",
  "files.loadingFile": "Loading file...",
  "files.unableToRead": "Unable to read file",
  "files.highlighting": "Highlighting...",

  // ── Documentation ──
  "docs.title": "Documentation",
  "docs.noContent": "Select a documentation topic from the sidebar.",
  "docs.generateTitle": "Generate Documentation",
  "docs.generateDesc": "Analyze your codebase and generate interactive wiki-style documentation with architecture diagrams, module guides, and API references.",
  "docs.featureModules": "Module dependency maps",
  "docs.featureCrossRef": "Cross-reference call graphs",
  "docs.featureApiDocs": "Auto-generated API docs",
  "docs.featureChat": "Ask questions about your code",
  "docs.generating": "Generating...",
  "docs.generateButton": "Generate Docs",
  "docs.loadingDocs": "Loading documentation...",
  "docs.loadingPage": "Loading page...",
  "docs.selectPage": "Select a page from the navigation",
  "docs.askAboutCode": "Ask about code",
  "docs.regenerateTitle": "Regenerate documentation",
  "docs.onThisPage": "On this page",
  "docs.diagramError": "Diagram rendering error",
  "docs.statsFiles": "files",
  "docs.statsModules": "modules",
  "docs.searchPlaceholder": "Search docs...",
  "docs.noResults": "No results found",

  // ── Export Panel ──
  "export.title": "Export & ASP.NET",
  "export.subtitle": "DOCX documentation and ASP.NET MVC 5 / EF6 statistics",
  "export.noRepoDesc": "Open a repository from the Repositories tab to access DOCX export and ASP.NET statistics.",
  "export.refreshStats": "Refresh statistics",
  "export.docxTitle": "Export as DOCX",
  "export.docxDesc": "Generates a complete Word document with architecture, controllers, entities, routes, and ER diagrams.",
  "export.exporting": "Generating...",
  "export.generateDocx": "Generate DOCX",
  "export.success": "Export successful",
  "export.error": "Export error",
  "export.loading": "Loading...",
  "export.statsTitle": "ASP.NET MVC 5 / EF6",
  "export.noAspnet": "No ASP.NET elements detected",
  "export.noAspnetDesc": "Index an ASP.NET MVC 5 / .NET Framework project to see controllers, entities, and views.",
  "export.elements": "elements",
  "export.controllers": "Controllers",
  "export.actions": "Actions",
  "export.apiEndpoints": "API Endpoints",
  "export.razorViews": "Razor Views",
  "export.efEntities": "EF Entities",
  "export.dbContexts": "DbContexts",
  "export.areas": "Areas",

  // ── Graph Explorer ──
  "graph.loadingGraph": "Loading graph...",
  "graph.failedToLoad": "Failed to load graph",

  // ── Code Health ──
  "health.title": "Code Health",
  "health.hotspots": "Hotspots",
  "health.cohesion": "Cohesion",
  "health.tracing": "Tracing",
  "health.ownership": "Ownership",
  "health.complexity": "Complexity",

  // ── Cypher Query FAB ──
  "cypher.title": "Cypher Query",
  "cypher.hint": "Ctrl+Enter to run",
  "cypher.run": "Run Query",
  "cypher.running": "Running...",
  "cypher.results": "results",
  "cypher.result": "result",

  // ── Accessibility ──
  "a11y.skipToContent": "Skip to main content",
  "a11y.codeIntelligencePlatform": "GitNexus — Code Intelligence Platform",

  // ── Git Analytics ──
  "git.hotspots": "Hotspots",
  "git.coupling": "Coupling",
  "git.ownership": "Ownership",

  // ── Hotspots View ──
  "hotspots.loading": "Analyzing hotspots...",
  "hotspots.noData": "No hotspot data available. Make sure the repository has git history.",
  "hotspots.filesAnalyzed": "{0} files analyzed (last 90 days)",
  "hotspots.colRank": "#",
  "hotspots.colFile": "File",
  "hotspots.colCommits": "Commits",
  "hotspots.colChurn": "Churn",
  "hotspots.colAuthors": "Authors",
  "hotspots.colScore": "Score",

  // ── Coupling View ──
  "coupling.loading": "Analyzing coupling...",
  "coupling.noData": "No temporal coupling detected. Files change independently.",
  "coupling.pairsDetected": "{0} coupled pairs detected",
  "coupling.stronglyCoupled": "{0} strongly coupled (>70%)",
  "coupling.colRank": "#",
  "coupling.colFileA": "File A",
  "coupling.colFileB": "File B",
  "coupling.colShared": "Shared",
  "coupling.colStrength": "Strength",

  // ── Ownership View ──
  "ownership.loading": "Analyzing ownership...",
  "ownership.noData": "No ownership data available.",
  "ownership.authors": "Authors ({0})",
  "ownership.files": "files",
  "ownership.orphanWarning": "{0} files with no clear owner (<50% ownership)",
  "ownership.colFile": "File",
  "ownership.colPrimaryAuthor": "Primary Author",
  "ownership.colOwnership": "Ownership",
  "ownership.colAuthors": "Authors",

  // ── Export Panel (toast) ──
  "export.toastSuccess": "DOCX exported successfully",
  "export.toastError": "Export failed: {0}",
  "export.ariaRefresh": "Refresh statistics",
  "export.ariaExport": "Export documentation as DOCX",

  // ── Tooltips for common actions ──
  "tooltip.clickToOpen": "Click to open",
  "tooltip.rightClickForMenu": "Right-click for context menu",
  "tooltip.dragToMove": "Drag to reposition",
  "tooltip.scrollToZoom": "Scroll to zoom in/out",
} as const;

const fr: Record<keyof typeof en, TranslationValue> = {
  // ── Sidebar ──
  "sidebar.workspace": "ESPACE DE TRAVAIL",
  "sidebar.explore": "EXPLORER",
  "sidebar.analysis": "ANALYSE",
  "sidebar.overview": "Vue d'ensemble",
  "sidebar.repositories": "Dépôts",
  "sidebar.fileExplorer": "Explorateur de fichiers",
  "sidebar.graphExplorer": "Explorateur de graphe",
  "sidebar.impactAnalysis": "Analyse d'impact",
  "sidebar.documentation": "Documentation",
  "sidebar.export": "Export",
  "sidebar.gitAnalytics": "Git Analytics",
  "sidebar.chat": "Chat",
  "sidebar.coverage": "Couverture",
  "sidebar.diagram": "Diagrammes",
  "sidebar.report": "Rapport santé",
  "sidebar.settings": "Paramètres",
  "sidebar.collapse": { label: "Réduire le panneau", tip: "Afficher/masquer la barre latérale (Ctrl+B)" },
  "sidebar.expand": { label: "Agrandir le panneau", tip: "Afficher la barre de navigation (Ctrl+B)" },

  // ── Repo Manager ──
  "repos.title": "Dépôts",
  "repos.indexed": "indexé(s)",
  "repos.repository": "dépôt",
  "repos.repositories": "dépôts",
  "repos.analyzeProject": { label: "Analyser un projet", tip: "Sélectionnez un dossier pour scanner et construire son graphe de connaissances" },
  "repos.refresh": { label: "Actualiser", tip: "Recharger la liste des dépôts indexés" },
  "repos.noRepos": "Aucun dépôt indexé",
  "repos.noReposDesc": "Commencez par analyser un projet pour explorer son graphe de code.",
  "repos.reindex": { label: "Ré-indexer", tip: "Re-scanner ce projet pour mettre à jour son graphe de connaissances" },
  "repos.generateWiki": { label: "Générer le Wiki", tip: "Générer automatiquement un wiki à partir de la structure du code" },
  "repos.generateDocs": { label: "Générer la Doc", tip: "Créer une documentation technique à partir de l'analyse du code" },
  "repos.generateAgents": { label: "Générer AGENTS.md", tip: "Générer un fichier de contexte pour agents IA" },
  "repos.generateAll": { label: "Tout générer", tip: "Lancer tous les générateurs (Wiki + Docs + AGENTS.md)" },
  "repos.onboarding.step1.title": "1. Choisir un projet",
  "repos.onboarding.step1.desc": "Cliquez sur « Analyser un projet » pour sélectionner un dossier.",
  "repos.onboarding.step2.title": "2. Analyse automatique",
  "repos.onboarding.step2.desc": "GitNexus scanne vos fichiers, résout les imports et construit un graphe de connaissances.",
  "repos.onboarding.step3.title": "3. Explorer",
  "repos.onboarding.step3.desc": "Naviguez dans le graphe interactif, parcourez les fichiers et analysez les impacts.",
  "repos.files": "fichiers",
  "repos.nodes": "nœuds",
  "repos.edges": "arêtes",
  "repos.communities": "communautés",
  "repos.loading": "Chargement des dépôts…",
  "repos.error": "Impossible de charger les dépôts",
  "repos.processing": "Traitement en cours…",
  "repos.opening": "Ouverture…",

  // ── Coverage ──
  "coverage.loading": "Chargement des données de couverture…",
  "coverage.totalMethods": "Méthodes totales",
  "coverage.deadCode": "Code mort",
  "coverage.coverageLabel": "Couverture",
  "coverage.deadCandidates": "Candidats code mort",
  "coverage.noDead": "Aucun candidat code mort trouvé",
  "coverage.method": "Méthode",
  "coverage.class": "Classe",
  "coverage.file": "Fichier",

  // ── Diagram ──
  "diagram.placeholder": "Entrez un nom de classe, contrôleur ou service…",
  "diagram.generate": "Générer",
  "diagram.generating": "Génération du diagramme…",
  "diagram.copied": "Copié !",
  "diagram.copyMermaid": "Copier Mermaid",
  "diagram.noDiagram": "Aucun diagramme généré. Le symbole n'existe peut-être pas.",

  // ── Report ──
  "report.temporalCoupling": "Couplage temporel (Top 10)",
  "report.distributedFiles": "Fichiers distribués (Top 10)",

  // ── Graph Explorer ──
  "graph.packages": { label: "Packages", tip: "Afficher les modules et packages de premier niveau" },
  "graph.modules": { label: "Modules", tip: "Afficher les symboles au niveau module (structs, traits, classes)" },
  "graph.symbols": { label: "Symboles", tip: "Afficher tous les symboles (fonctions, types, constantes)" },
  "graph.nodesCount": "nœuds",
  "graph.edgesCount": "arêtes",
  "graph.fitView": { label: "Ajuster la vue", tip: "Zoomer pour afficher tous les nœuds (F)" },
  "graph.layout": { label: "Disposition", tip: "Changer l'algorithme de disposition du graphe" },
  "graph.contextMenu.goToDefinition": { label: "Aller à la définition", tip: "Aller au fichier source de ce symbole" },
  "graph.contextMenu.findReferences": { label: "Trouver les références", tip: "Afficher tous les endroits qui référencent ce symbole" },
  "graph.contextMenu.expandNeighbors": { label: "Développer les voisins", tip: "Révéler les nœuds connectés" },
  "graph.contextMenu.hideNode": { label: "Masquer le nœud", tip: "Retirer ce nœud de la vue actuelle" },
  "graph.contextMenu.copyName": { label: "Copier le nom", tip: "Copier le nom du symbole dans le presse-papier" },
  "graph.contextMenu.copyFilePath": { label: "Copier le chemin", tip: "Copier le chemin du fichier source dans le presse-papier" },
  "graph.minimap.toggle": { label: "Minicarte", tip: "Afficher/masquer la minicarte de navigation" },
  "graph.legend": { label: "Légende", tip: "Afficher/masquer la légende des couleurs par type de nœud" },
  "graph.noData": "Aucune donnée de graphe disponible",
  "graph.analyzeFirst": "Analysez un dépôt d'abord",
  "graph.computingLayout": "Calcul du layout…",
  "graph.backToFull": "Retour au graphe complet",
  "graph.showingTopNodes": "Affichage des {0} nœuds les plus importants. Double-cliquez pour explorer le voisinage.",
  "graph.exportPng": "Exporter le graphe en PNG (Ctrl+E)",
  "graph.processFlows": "Flux de processus",
  "graph.edgeFilters": "Arêtes",
  "graph.keyboardShortcuts": "Raccourcis clavier",
  "graph.viewImpact": "Voir l'impact",
  "graph.clearImpact": "Effacer l'impact",
  "graph.impactOverlay": "Overlay d'impact",
  "graph.shortcut.goToSymbol": "Aller au symbole",
  "graph.shortcut.exportPng": "Exporter le graphe PNG",
  "graph.shortcut.screenshot": "Capture d'écran",
  "graph.shortcut.zoomInOutFit": "Zoom avant/arrière/ajuster",
  "graph.shortcut.navigateBackForward": "Naviguer précédent/suivant",
  "graph.shortcut.clearSelection": "Effacer la sélection",
  "graph.shortcut.focusSubgraph": "Focus sous-graphe",
  "graph.shortcut.toggleHelp": "Afficher/masquer cette aide",

  // ── File Explorer ──
  "files.title": "Fichiers",
  "files.lines": "lignes",
  "files.backToTree": { label: "Retour", tip: "Revenir à l'arborescence des fichiers" },

  // ── Detail Panel ──
  "detail.context": "Contexte",
  "detail.code": "Code",
  "detail.codeProperties": "Propriétés",
  "detail.layers": "Couches",
  "detail.health": "Santé",
  "detail.callers": "APPELANTS",
  "detail.callees": "APPELÉS",
  "detail.community": "COMMUNAUTÉ",
  "detail.members": "membres",
  "detail.cohesion": "Cohésion",
  "detail.noSelection": "Sélectionnez un nœud dans le graphe pour voir ses détails ici.",

  // ── Impact Analysis ──
  "impact.title": { label: "Analyse d'impact", tip: "Comprendre comment les modifications d'un symbole se propagent dans votre codebase" },
  "impact.placeholder": "Rechercher un symbole à analyser…",
  "impact.searchAndSelect": "Recherchez et sélectionnez un symbole pour analyser son rayon d'impact",
  "impact.analyzingImpact": "Analyse d'impact en cours…",
  "impact.upstream": "Amont (appelants)",
  "impact.downstream": "Aval (appelés)",
  "impact.affectedFiles": "Fichiers affectés",
  "impact.statUpstream": "Amont",
  "impact.statDownstream": "Aval",
  "impact.statFiles": "Fichiers",
  "impact.directionUpstream": "Amont",
  "impact.directionBoth": "Les deux",
  "impact.directionDownstream": "Aval",
  "impact.impactDistribution": "Distribution d'impact",
  "impact.moreItems": "+{0} de plus",

  // ── Command Bar ──
  "search.placeholder": "Rechercher symboles, fichiers, classes…",
  "search.shortcut": "Ctrl K",
  "search.navigate": "Naviguer",
  "search.open": "Ouvrir",
  "search.close": "Fermer",
  "search.noResults": "Aucun résultat trouvé",
  "search.startTyping": "Commencez à taper pour rechercher…",
  "search.ariaLabel": "Ouvrir la recherche de symboles",

  // ── Command Bar breadcrumb tabs ──
  "commandBar.tab.repos": "Dépôts",
  "commandBar.tab.search": "Recherche",
  "commandBar.tab.files": "Fichiers",
  "commandBar.tab.graph": "Explorateur de graphe",
  "commandBar.tab.impact": "Analyse d'impact",
  "commandBar.tab.docs": "Documentation",

  // ── Manage ──
  "manage.title": "Gérer",
  "manage.repositories": "Dépôts",
  "manage.export": "Export",
  "manage.documentation": "Documentation",
  "manage.settings": "Paramètres",
  "manage.theme.dark": "Sombre",
  "manage.theme.light": "Clair",
  "manage.theme.system": "Système",

  // ── Settings ──
  "settings.title": "Paramètres",
  "settings.language": { label: "Langue", tip: "Choisir la langue d'affichage de l'interface" },
  "settings.theme": { label: "Thème", tip: "Basculer entre le mode clair et sombre" },
  "settings.shortcuts": { label: "Raccourcis clavier", tip: "Voir tous les raccourcis clavier disponibles" },
  "settings.soon": "Bientôt",

  // ── Status Bar ──
  "status.noRepo": "Aucun dépôt sélectionné",
  "status.view": "Vue",
  "status.packageLevel": "Niveau package",
  "status.moduleLevel": "Niveau module",
  "status.symbolLevel": "Niveau symbole",
  "status.browseSourceTree": "Parcourir : Arborescence",
  "status.modeDependencyAnalysis": "Analyse de dépendances",
  "status.docsWikiViewer": "Visionneuse wiki",
  "status.mode": "Mode",
  "status.docs": "Docs",
  "status.browse": "Parcourir",

  // ── Analyze Progress ──
  "analyze.analyzing": "Analyse en cours",
  "analyze.analysisFailed": "Échec de l'analyse",
  "analyze.analysisComplete": "Analyse terminée",
  "analyze.analyzingRepo": "Analyse de {name}…",
  "analyze.analyzeProject": "Analyser un projet",
  "analyze.phase.idle": "En attente",
  "analyze.phase.extracting": "Extraction",
  "analyze.phase.structure": "Scan des fichiers",
  "analyze.phase.parsing": "Analyse AST",
  "analyze.phase.imports": "Résolution des imports",
  "analyze.phase.calls": "Analyse des appels",
  "analyze.phase.heritage": "Hiérarchie de classes",
  "analyze.phase.communities": "Détection de communautés",
  "analyze.phase.processes": "Traçage des processus",
  "analyze.phase.enriching": "Enrichissement",
  "analyze.phase.complete": "Terminé",
  "analyze.phase.error": "Erreur",
  "analyze.files": "fichiers",
  "analyze.nodes": "nœuds",

  // ── File Explorer ──
  "files.searchPlaceholder": "Rechercher des fichiers\u2026",
  "files.searchFiles": "Rechercher des fichiers",
  "files.clearSearch": "Effacer la recherche",
  "files.noMatchingFiles": "Aucun fichier trouv\u00e9",
  "files.matchingFiles": "{0} fichier(s) trouv\u00e9(s)",
  "files.errorLoadingTree": "Erreur de chargement de l'arborescence",
  "files.noFilesFound": "Aucun fichier trouv\u00e9",
  "files.selectFileToPreview": "Sélectionnez un fichier pour afficher son contenu",
  "files.closePreview": "Fermer l'aperçu",
  "files.loadingFile": "Chargement du fichier…",
  "files.unableToRead": "Impossible de lire le fichier",
  "files.highlighting": "Coloration syntaxique…",

  // ── Documentation ──
  "docs.title": "Documentation",
  "docs.noContent": "Sélectionnez un sujet dans le panneau latéral.",
  "docs.generateTitle": "Générer la documentation",
  "docs.generateDesc": "Analysez votre codebase et générez une documentation interactive de type wiki avec des diagrammes d'architecture, des guides de modules et des références API.",
  "docs.featureModules": "Cartes de dépendances des modules",
  "docs.featureCrossRef": "Graphes d'appels croisés",
  "docs.featureApiDocs": "Documentation API auto-générée",
  "docs.featureChat": "Posez des questions sur votre code",
  "docs.generating": "Génération…",
  "docs.generateButton": "Générer la doc",
  "docs.loadingDocs": "Chargement de la documentation…",
  "docs.loadingPage": "Chargement de la page…",
  "docs.selectPage": "Sélectionnez une page dans la navigation",
  "docs.askAboutCode": "Poser une question sur le code",
  "docs.regenerateTitle": "Regénérer la documentation",
  "docs.onThisPage": "Sur cette page",
  "docs.diagramError": "Erreur de rendu du diagramme",
  "docs.statsFiles": "fichiers",
  "docs.statsModules": "modules",
  "docs.searchPlaceholder": "Rechercher dans la doc…",
  "docs.noResults": "Aucun résultat",

  // ── Export Panel ──
  "export.title": "Export & ASP.NET",
  "export.subtitle": "Documentation DOCX et statistiques ASP.NET MVC 5 / EF6",
  "export.noRepoDesc": "Ouvrez un dépôt depuis l'onglet Repositories pour accéder à l'export DOCX et aux statistiques ASP.NET.",
  "export.refreshStats": "Rafraîchir les statistiques",
  "export.docxTitle": "Exporter en DOCX",
  "export.docxDesc": "Génère un document Word complet avec l'architecture, les contrôleurs, les entités, les routes et les diagrammes ER.",
  "export.exporting": "Génération en cours…",
  "export.generateDocx": "Générer le DOCX",
  "export.success": "Export réussi",
  "export.error": "Erreur d'export",
  "export.loading": "Chargement…",
  "export.statsTitle": "ASP.NET MVC 5 / EF6",
  "export.noAspnet": "Aucun élément ASP.NET détecté",
  "export.noAspnetDesc": "Indexez un projet ASP.NET MVC 5 / .NET Framework pour voir les contrôleurs, entités et vues.",
  "export.elements": "éléments",
  "export.controllers": "Contrôleurs",
  "export.actions": "Actions",
  "export.apiEndpoints": "API Endpoints",
  "export.razorViews": "Vues Razor",
  "export.efEntities": "Entités EF",
  "export.dbContexts": "DbContexts",
  "export.areas": "Areas",

  // ── Graph Explorer ──
  "graph.loadingGraph": "Chargement du graphe…",
  "graph.failedToLoad": "Impossible de charger le graphe",

  // ── Code Health ──
  "health.title": "Santé du code",
  "health.hotspots": "Points chauds",
  "health.cohesion": "Cohésion",
  "health.tracing": "Traçabilité",
  "health.ownership": "Propriété",
  "health.complexity": "Complexité",

  // ── Cypher Query FAB ──
  "cypher.title": "Requ\u00eate Cypher",
  "cypher.hint": "Ctrl+Entr\u00e9e pour ex\u00e9cuter",
  "cypher.run": "Ex\u00e9cuter",
  "cypher.running": "Ex\u00e9cution\u2026",
  "cypher.results": "r\u00e9sultats",
  "cypher.result": "r\u00e9sultat",

  // ── Accessibility ──
  "a11y.skipToContent": "Aller au contenu principal",
  "a11y.codeIntelligencePlatform": "GitNexus — Plateforme d'intelligence de code",

  // ── Git Analytics ──
  "git.hotspots": "Points chauds",
  "git.coupling": "Couplage",
  "git.ownership": "Propriété",

  // ── Hotspots View ──
  "hotspots.loading": "Analyse des points chauds…",
  "hotspots.noData": "Aucune donnée de points chauds disponible. Assurez-vous que le dépôt possède un historique git.",
  "hotspots.filesAnalyzed": "{0} fichiers analysés (90 derniers jours)",
  "hotspots.colRank": "#",
  "hotspots.colFile": "Fichier",
  "hotspots.colCommits": "Commits",
  "hotspots.colChurn": "Churn",
  "hotspots.colAuthors": "Auteurs",
  "hotspots.colScore": "Score",

  // ── Coupling View ──
  "coupling.loading": "Analyse du couplage…",
  "coupling.noData": "Aucun couplage temporel détecté. Les fichiers changent indépendamment.",
  "coupling.pairsDetected": "{0} paires couplées détectées",
  "coupling.stronglyCoupled": "{0} fortement couplées (>70%)",
  "coupling.colRank": "#",
  "coupling.colFileA": "Fichier A",
  "coupling.colFileB": "Fichier B",
  "coupling.colShared": "Partagés",
  "coupling.colStrength": "Force",

  // ── Ownership View ──
  "ownership.loading": "Analyse de la propriété…",
  "ownership.noData": "Aucune donnée de propriété disponible.",
  "ownership.authors": "Auteurs ({0})",
  "ownership.files": "fichiers",
  "ownership.orphanWarning": "{0} fichiers sans propriétaire clair (<50% de propriété)",
  "ownership.colFile": "Fichier",
  "ownership.colPrimaryAuthor": "Auteur principal",
  "ownership.colOwnership": "Propriété",
  "ownership.colAuthors": "Auteurs",

  // ── Export Panel (toast) ──
  "export.toastSuccess": "DOCX exporté avec succès",
  "export.toastError": "Échec de l'export : {0}",
  "export.ariaRefresh": "Rafraîchir les statistiques",
  "export.ariaExport": "Exporter la documentation en DOCX",

  // ── Tooltips for common actions ──
  "tooltip.clickToOpen": "Cliquer pour ouvrir",
  "tooltip.rightClickForMenu": "Clic droit pour le menu contextuel",
  "tooltip.dragToMove": "Glisser pour repositionner",
  "tooltip.scrollToZoom": "Molette pour zoomer",
};

const dictionaries: Record<Locale, Record<string, TranslationValue>> = { en, fr };

// ─── Runtime state ───

let currentLocale: Locale = (typeof localStorage !== "undefined" && localStorage.getItem("gitnexus-locale") as Locale) || "fr";
const listeners = new Set<() => void>();

export function getLocale(): Locale {
  return currentLocale;
}

export function setLocale(locale: Locale) {
  currentLocale = locale;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("gitnexus-locale", locale);
  }
  listeners.forEach((fn) => fn());
}

export function subscribe(fn: () => void) {
  listeners.add(fn);
  return () => { listeners.delete(fn); };
}

/** Get a translated string (label only). */
export function t(key: string): string {
  const dict = dictionaries[currentLocale] ?? dictionaries.en;
  const val = dict[key] ?? dictionaries.en[key];
  if (val === undefined) {
    if (import.meta.env.DEV) {
      console.warn(`[i18n] Missing translation key: "${key}" (locale: ${currentLocale})`);
    }
    return key;
  }
  if (typeof val === "string") return val;
  return val.label;
}

/** Get translated label + optional tooltip. */
export function tt(key: string): TranslationEntry {
  const dict = dictionaries[currentLocale] ?? dictionaries.en;
  const val = dict[key] ?? dictionaries.en[key];
  if (!val) return { label: key };
  if (typeof val === "string") return { label: val };
  return val;
}
