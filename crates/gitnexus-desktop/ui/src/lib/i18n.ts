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
  "chat.conversationCleared": "Conversation cleared",
  "chat.executingResearch": "executing research plan...",
  "chat.searchingContext": "searching filtered context...",
  "chat.thinking": "thinking...",
  "chat.confirmClear": "Clear all messages in this conversation?",
  "chat.generatingResponse": "generating response...",
  "chat.deepResearchHint": "Deep Research: multi-step analysis with plan execution. Ctrl+P for files.",
  "chat.inputHint": "Powered by knowledge graph context. Enter to send, Shift+Enter for new line.",
  "chat.you": "You",
  "chat.copiedToClipboard": "Copied to clipboard",
  "chat.quickAnswer": "Quick answer",
  "chat.multiSource": "Multi-source",
  "chat.deepResearch": "Deep research",
  "chat.welcomeTitle": "Ask about your code",
  "chat.welcomeDesc": "Ask questions about architecture, dependencies, or code quality.",
  "chat.suggestion.entryPoints": "What are the main entry points?",
  "chat.suggestion.complex": "Which classes are the most complex?",
  "chat.suggestion.architecture": "Explain the project architecture",
  "chat.suggestion.deadCode": "Find dead code candidates",
  "chat.repoSwitched": "Switched to {0}",
  "chat.repoSwitchFailed": "Failed to switch repo: {0}",
  "chat.loadingConfig": "Loading assistant configuration...",
  "chat.newChat": "New Chat",
  "chat.recentChats": "Recent Chats",
  "chat.noRecentChats": "No recent chats",
  "chat.renameChat": "Rename chat",
  "chat.deleteChat": "Delete chat",
  "chat.copyFailed": "Failed to copy",
  "chat.copyCode": "Copy code",
  "chat.exportedAsMarkdown": "Chat exported as Markdown",
  "chat.exportChatMarkdown": "Export chat as Markdown",
  "chat.exportResponseMarkdown": "Export response as Markdown",
  "chat.exportFailed": "Failed to export: {0}",
  "chat.responseExported": "Response exported successfully",
  "chat.saveFailed": "Failed to save: {0}",
  "chat.navigateToNode": "Navigate to node in graph",
  "chat.apiKeyPlaceholder": "sk-... (leave empty for Ollama)",
  "chat.selectRepo": "Select Repository",
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
  "welcome.tauriRequired": "Folder picker requires the Tauri desktop app.",
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
  "repos.repoOpened": "Opened {0}",
  "repos.analysisFailed": "Analysis failed: {0}",

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
  "diagram.copyFailed": "Copy failed",

  // ── Report ──
  "report.temporalCoupling": "Temporal Coupling (Top 10)",
  "report.distributedFiles": "Distributed Files (Top 10)",
  "report.file": "File",
  "report.commits": "Commits",
  "report.churn": "Churn",
  "report.score": "Score",
  "report.fileA": "File A",
  "report.fileB": "File B",
  "report.shared": "Shared",
  "report.strength": "Strength",
  "report.primaryAuthor": "Primary Author",
  "report.authors": "Authors",
  "report.ownership": "Ownership",

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
  "graph.noTreemapData": "No graph data to display as treemap.",
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
  "graph.copiedToClipboard": "Copied to clipboard",
  "graph.copyFailed": "Copy failed",
  "graph.impactFailed": "Impact analysis failed",
  "graph.impactOverlay": "Impact Overlay",
  "graph.edges": "Edges",
  "graph.depth": "Depth",
  "graph.all": "All",
  "graph.shortcut.goToSymbol": "Go to symbol",
  "graph.shortcut.exportPng": "Export graph PNG",
  "graph.shortcut.screenshot": "Screenshot",
  "graph.shortcut.zoomInOutFit": "Zoom in/out/fit",
  "graph.shortcut.clearSelection": "Clear selection",
  "graph.shortcut.focusSubgraph": "Focus subgraph",
  "graph.shortcut.toggleHelp": "Toggle this help",

  // ── Explorer Mode ──
  "explorer.noRepo": "No repository selected",
  "explorer.noRepoHint": "Open a repo from the Manage tab to start exploring.",

  // ── File Explorer ──
  "files.title": "Files",
  "files.lines": "lines",
  "files.backToTree": { label: "Back", tip: "Return to the file tree" },

  // ── Detail Panel ──
  "detail.noSelection": "Select a symbol",
  "detail.noSelectionHint": "Click a node in the graph or file tree to inspect its callers, dependencies, and code.",
  "detail.context": "Context",
  "detail.code": "Code",
  "detail.codeProperties": "Properties",
  "detail.layers": "Layers",
  "detail.health": "Health",
  "detail.collapse": "Collapse",
  "detail.preview": "Preview",
  "detail.callers": "CALLERS",
  "codeInspector.title": "Code Inspector",
  "codeInspector.selectNode": "Select a node in the graph to inspect its code",
  "codeInspector.loading": "Loading...",
  "code.selectSymbol": "Select a symbol to view its code",
  "code.noFile": "No file associated with this symbol",
  "code.loading": "Loading code...",
  "detail.callees": "CALLEES",
  "detail.community": "COMMUNITY",
  "detail.members": "members",
  "detail.cohesion": "Cohesion",
  "detail.imports": "Imports",
  "detail.importedBy": "Imported By",
  "detail.inherits": "Inherits",
  "detail.inheritedBy": "Inherited By",
  "analyze.openRepo": "Open a repository to view analytics",
  "analyze.errorTitle": "Analysis Error",
  "analyze.codeHealth": "Code Health",
  "detail.cyclomaticComplexity": "Cyclomatic Complexity",

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

  // ── Symbol Autocomplete ──
  "symbol.columnType": "Type",
  "symbol.columnName": "Name",
  "symbol.columnFile": "File",
  "symbol.columnLines": "Lines",

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
  "settings.quickSetup": "Quick Setup",
  "settings.baseUrl": "Base URL",
  "settings.model": "Model",
  "settings.apiKey": "API Key",
  "settings.maxTokens": "Max Tokens",
  "settings.thinking": "Thinking / Reasoning",
  "settings.thinkingHint": "For models with thinking support (Gemini, o1, etc.)",
  "settings.save": "Save",
  "settings.cancel": "Cancel",
  "settings.chatAiTitle": "Chat AI Settings",
  "settings.securityNote": "Your API key is stored locally and never shared.",

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
  "status.nodes": "nodes",
  "status.docs": "Docs",
  "status.browse": "Browse",
  "status.aiChat": "Code Intelligence Chat",
  "status.reposSettings": "Repos & Settings",

  // ── Analyze Nav ──
  "analyze.nav.title": "Analytics",
  "analyze.nav.overview": "Overview",
  "analyze.nav.hotspots": "Hotspots",
  "analyze.nav.coupling": "Coupling",
  "analyze.nav.ownership": "Ownership",
  "analyze.nav.coverage": "Coverage",
  "analyze.nav.diagrams": "Diagrams",
  "analyze.nav.report": "Report",
  "analyze.nav.snapshots": "Snapshots",
  "analyze.nav.health": "Health",
  "analyze.nav.processes": "Process Flows",

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
  "analyze.insights": "Insights",
  "analyze.phase.processes": "Tracing processes",
  "analyze.phase.enriching": "Enriching",
  "analyze.phase.complete": "Complete",
  "analyze.phase.error": "Error",
  "analyze.files": "files",
  "analyze.nodes": "nodes",

  // ── Process Flows ──
  "analyze.processFlows": "Process Flows",
  "analyze.flowsDesc": "{count} high-level business processes identified.",
  "analyze.noFlowsTitle": "No Process Flows Found",
  "analyze.noFlowsDesc": "Automatic process tracing requires instrumented methods or specific business patterns in the code.",
  "analyze.stepCount": "{count} steps",
  "analyze.flowDiagram": "Interactive Diagram",
  "analyze.flowSteps": "Step Sequence",
  "analyze.viewCode": "View Code",
  "analyze.noStepsMessage": "Diagram only — no individual steps available for this process.",

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
  "export.obsidianTitle": "Obsidian Vault (Digital Brain)",
  "export.obsidianDesc": "Export the knowledge graph as a structured vault of Markdown files. Perfect for Andrej Karpathy's method of maintaining a digital brain of your codebase.",
  "export.generateObsidian": "Export Obsidian Vault",
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

  // ── Communities Panel ──
  "communities.title": "Functional Groups",
  "communities.showAll": "Show all",
  "communities.hint": "Click to isolate · Ctrl+Click to combine",

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

  // ── Mode Bar ──
  "mode.explorer": "Explorer",
  "mode.analyze": "Analyze",
  "mode.chat": "Chat",
  "mode.manage": "Manage",
  "mode.commandPalette": "Command Palette",
  "mode.collapse": "Collapse",

  // ── Lens Selector ──
  "lens.all": "All",
  "lens.all.desc": "Show all relationships",
  "lens.calls": "Calls",
  "lens.calls.desc": "Function/method calls",
  "lens.structure": "Structure",
  "lens.structure.desc": "HasMethod, HasProperty, ContainedIn",
  "lens.heritage": "Heritage",
  "lens.heritage.desc": "Extends, Implements",
  "lens.impact": "Impact",
  "lens.impact.desc": "Calls, Imports, DependsOn",
  "lens.deadCode": "Dead Code",
  "lens.deadCode.desc": "Highlight dead code candidates",
  "lens.tracing": "Tracing",
  "lens.tracing.desc": "Highlight traced methods",
  "lens.hotspots": "Hotspots",
  "lens.hotspots.desc": "Highlight frequently changed files",
  "lens.risk": "Risk",
  "lens.risk.desc": "Composite risk: churn + dead code + missing tracing",
  "lens.ariaLabel": "Graph lens filter",

  // ── Cypher Presets ──
  "cypher.preset.allFunctions": "All Functions",
  "cypher.preset.callGraph": "Call Graph",
  "cypher.preset.controllers": "Controllers",
  "cypher.preset.deadCode": "Dead Code",
  "cypher.preset.topCallers": "Top Callers",
  "cypher.preset.services": "Services",
  "cypher.preset.communities": "Communities",

  // ── Graph Zoom ──
  "zoom.in": "Zoom in (Ctrl+=)",
  "zoom.out": "Zoom out (Ctrl+-)",
  "zoom.fit": "Fit view (Ctrl+0)",
  "zoom.inLabel": "Zoom in",
  "zoom.outLabel": "Zoom out",
  "zoom.fitLabel": "Fit view",

  // ── Graph Toolbar extras ──
  "graph.truncated": "truncated",
  "graph.granularity": "Graph granularity level",
  "graph.collapseLegend": "Collapse legend",

  // ── Command Palette ──
  "cmd.placeholder": "Type a command or search...",
  "cmd.switchTo": "Switch to",
  "cmd.view": "View",
  "cmd.lens": "Lens:",
  "cmd.openSettings": "Open Settings",
  "cmd.toggleDeepResearch": "Toggle Deep Research",
  "cmd.group.modes": "Modes",
  "cmd.group.analyzeViews": "Analyze Views",
  "cmd.group.lenses": "Lenses",
  "cmd.group.actions": "Actions",
  "cmd.group.userCommands": "User Commands",
  // New entries shipped recently — actions group
  "cmd.renameRefactor": "Rename refactor…",
  "cmd.exportHtml": "Export interactive HTML",
  "cmd.generateWiki": "Generate wiki (Markdown per module)",
  "cmd.generateWikiLlm": "Generate wiki with LLM overviews (slower)",
  "cmd.openNotebooks": "Cypher notebooks",
  "cmd.openDashboards": "Custom dashboards",
  "cmd.openWorkflows": "Workflow editor",
  "cmd.openUserCommands": "User slash commands",
  "cmd.bundleExport": "Export user data bundle…",
  "cmd.bundleImport": "Import user data bundle…",

  // ── Accessibility ──
  "a11y.skipToContent": "Skip to main content",
  "a11y.codeIntelligencePlatform": "GitNexus — Code Intelligence Platform",

  // ── Errors ──
  "error.somethingWentWrong": "Something went wrong",
  "error.retry": "Retry",

  // ── Git Analytics ──
  "git.hotspots": "Hotspots",
  "git.coupling": "Coupling",
  "git.ownership": "Ownership",

  // ── Hotspots View ──
  "hotspots.loading": "Analyzing hotspots...",
  "hotspots.noData": "No hotspot data available",
  "hotspots.noDataHint": "Make sure the repository has git history to analyze file change frequency.",
  "hotspots.filesAnalyzed": "{0} files analyzed (last 90 days)",
  "hotspots.colRank": "#",
  "hotspots.colFile": "File",
  "hotspots.colCommits": "Commits",
  "hotspots.colChurn": "Churn",
  "hotspots.colAuthors": "Authors",
  "hotspots.colScore": "Score",

  // ── Coupling View ──
  "coupling.loading": "Analyzing coupling...",
  "coupling.noData": "No temporal coupling detected",
  "coupling.noDataHint": "Files change independently. Coupling is detected when files are frequently modified together.",
  "coupling.pairsDetected": "{0} coupled pairs detected",
  "coupling.stronglyCoupled": "{0} strongly coupled (>70%)",
  "coupling.colRank": "#",
  "coupling.colFileA": "File A",
  "coupling.colFileB": "File B",
  "coupling.colShared": "Shared",
  "coupling.colStrength": "Strength",

  // ── Ownership View ──
  "ownership.loading": "Analyzing ownership...",
  "ownership.noData": "No ownership data available",
  "ownership.noDataHint": "Analyze a repository with git history to see author distribution per file.",
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

  // ── Dashboard ──
  "dashboard.nodeTypeDistribution": "Node Type Distribution",
  "dashboard.topConnectedNodes": "Top Connected Nodes",
  "dashboard.mostComplexFunctions": "Most Complex Functions",
  "dashboard.healthy": "Healthy",
  "dashboard.growing": "Growing",
  "dashboard.small": "Small",

  // ── Tooltips for common actions ──
  "tooltip.clickToOpen": "Click to open",
  "tooltip.rightClickForMenu": "Right-click for context menu",
  "tooltip.dragToMove": "Drag to reposition",
  "tooltip.scrollToZoom": "Scroll to zoom in/out",

  // ── Filter modals ──
  "filters.searchFiles": "Search files... (type to filter)",
  "filters.noFilesFound": "No files found",
  "filters.typeToSearchFiles": "Type to search files...",
  "filters.searchSymbols": "Search symbols... (@function, #class)",
  "filters.noSymbolsFound": "No symbols found",
  "filters.typeToSearchSymbols": "Type to search symbols...",
  "filters.searchModules": "Search modules/communities...",
  "filters.noModulesFound": "No modules found",
  "filters.loadingModules": "Loading modules...",

  // ── Detail panel ──
  "detail.emptyHint": "Click a node in the graph to see its details",
  "detail.loadingContext": "Loading context...",
  "detail.exported": "exported",
  "detail.entryPoint": "Entry Point",
  "detail.traced": "Traced",
  "detail.architectureLayer": "Architecture Layer",

  // ── Node hover card ──
  "hover.source": "Source",
  "hover.impact": "Impact",

  // ── Graph toolbar ──
  "toolbar.complexity": "Cplx",
  "toolbar.gitRange": "Git",

  // ── Source references ──
  "sources.title": "Sources",
  "sources.showMore": "Show {0} more sources",
  "sources.showFewer": "Show fewer sources",

  // ── Research plan viewer ──
  "research.planTitle": "Research Plan",
  "research.toolSearch": "Symbol Search",
  "research.toolContext": "Context Analysis",
  "research.toolRead": "Read File",
  "research.toolCypher": "Graph Query",
  "research.toolImpact": "Impact Analysis",

  // ── Manage ──
  "manage.multiRepoOverview": "Multi-repo overview",

  // ── Comments ──
  "comments.emptyHint": "No notes yet. Add one to record context for your team.",

  // ── Cypher ──
  "cypher.emptyQuery": "Nothing to save — write a query first",

  // ── Rename ──
  "rename.searching": "Searching…",
  "rename.preview": "Preview",

  // ── Common ──
  "common.loading": "Loading",
  "common.noRows": "No data to display",
  "common.retry": "Retry",
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
  "chat.conversationCleared": "Conversation effacée",
  "chat.executingResearch": "exécution du plan de recherche...",
  "chat.searchingContext": "recherche du contexte filtré...",
  "chat.thinking": "réflexion...",
  "chat.confirmClear": "Effacer tous les messages de cette conversation ?",
  "chat.generatingResponse": "génération de la réponse...",
  "chat.deepResearchHint": "Recherche approfondie : analyse multi-étapes avec exécution de plan. Ctrl+P pour les fichiers.",
  "chat.inputHint": "Contexte via graphe de connaissances. Entrée pour envoyer, Maj+Entrée pour nouvelle ligne.",
  "chat.you": "Vous",
  "chat.copiedToClipboard": "Copié dans le presse-papiers",
  "chat.quickAnswer": "Réponse rapide",
  "chat.multiSource": "Multi-source",
  "chat.deepResearch": "Recherche approfondie",
  "chat.welcomeTitle": "Interrogez votre code",
  "chat.welcomeDesc": "Posez des questions sur l'architecture, les dépendances ou la qualité du code.",
  "chat.suggestion.entryPoints": "Quels sont les points d'entrée principaux ?",
  "chat.suggestion.complex": "Quelles classes sont les plus complexes ?",
  "chat.suggestion.architecture": "Explique l'architecture du projet",
  "chat.suggestion.deadCode": "Trouve les candidats au code mort",
  "chat.repoSwitched": "Basculé vers {0}",
  "chat.repoSwitchFailed": "Échec du changement de dépôt : {0}",
  "chat.loadingConfig": "Chargement de la configuration de l'assistant...",
  "chat.newChat": "Nouveau chat",
  "chat.recentChats": "Chats récents",
  "chat.noRecentChats": "Aucun chat récent",
  "chat.renameChat": "Renommer le chat",
  "chat.deleteChat": "Supprimer le chat",
  "chat.copyFailed": "Échec de la copie",
  "chat.copyCode": "Copier le code",
  "chat.exportedAsMarkdown": "Chat exporté en Markdown",
  "chat.exportChatMarkdown": "Exporter le chat en Markdown",
  "chat.exportResponseMarkdown": "Exporter la réponse en Markdown",
  "chat.exportFailed": "Échec de l'export : {0}",
  "chat.responseExported": "Réponse exportée avec succès",
  "chat.saveFailed": "Échec de la sauvegarde : {0}",
  "chat.navigateToNode": "Naviguer vers le nœud dans le graphe",
  "chat.apiKeyPlaceholder": "sk-... (laisser vide pour Ollama)",
  "chat.selectRepo": "Sélectionner un dépôt",
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
  "welcome.tauriRequired": "Le sélecteur de dossier nécessite l'application Tauri.",
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
  "repos.repoOpened": "Ouvert : {0}",
  "repos.analysisFailed": "Échec de l'analyse : {0}",

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
  "diagram.copyFailed": "Échec de la copie",

  // ── Report ──
  "report.temporalCoupling": "Couplage temporel (Top 10)",
  "report.distributedFiles": "Fichiers distribués (Top 10)",
  "report.file": "Fichier",
  "report.commits": "Commits",
  "report.churn": "Churn",
  "report.score": "Score",
  "report.fileA": "Fichier A",
  "report.fileB": "Fichier B",
  "report.shared": "Partagés",
  "report.strength": "Force",
  "report.primaryAuthor": "Auteur principal",
  "report.authors": "Auteurs",
  "report.ownership": "Propriété",

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
  "graph.noTreemapData": "Aucune donnée de graphe à afficher en treemap.",
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
  "graph.copiedToClipboard": "Copié dans le presse-papiers",
  "graph.copyFailed": "Echec de la copie",
  "graph.impactFailed": "Echec de l'analyse d'impact",
  "graph.impactOverlay": "Overlay d'impact",
  "graph.edges": "Arêtes",
  "graph.depth": "Profondeur",
  "graph.all": "Tout",
  "graph.shortcut.goToSymbol": "Aller au symbole",
  "graph.shortcut.exportPng": "Exporter le graphe PNG",
  "graph.shortcut.screenshot": "Capture d'écran",
  "graph.shortcut.zoomInOutFit": "Zoom avant/arrière/ajuster",
  "graph.shortcut.clearSelection": "Effacer la sélection",
  "graph.shortcut.focusSubgraph": "Focus sous-graphe",
  "graph.shortcut.toggleHelp": "Afficher/masquer cette aide",

  // ── Explorer Mode ──
  "explorer.noRepo": "Aucun dépôt sélectionné",
  "explorer.noRepoHint": "Ouvrez un dépôt depuis l'onglet Gérer pour commencer.",

  // ── File Explorer ──
  "files.title": "Fichiers",
  "files.lines": "lignes",
  "files.backToTree": { label: "Retour", tip: "Revenir à l'arborescence des fichiers" },

  // ── Detail Panel ──
  "detail.noSelection": "Sélectionnez un symbole",
  "detail.noSelectionHint": "Cliquez sur un nœud du graphe ou de l'arborescence des fichiers pour inspecter ses appelants, dépendances et code.",
  "detail.context": "Contexte",
  "detail.code": "Code",
  "detail.codeProperties": "Propriétés",
  "detail.layers": "Couches",
  "detail.health": "Santé",
  "detail.collapse": "Réduire",
  "detail.preview": "Aperçu",
  "detail.callers": "APPELANTS",
  "codeInspector.title": "Inspecteur de code",
  "codeInspector.selectNode": "Sélectionnez un nœud du graphe pour inspecter son code",
  "codeInspector.loading": "Chargement...",
  "code.selectSymbol": "Sélectionnez un symbole pour voir son code",
  "code.noFile": "Aucun fichier associé à ce symbole",
  "code.loading": "Chargement du code...",
  "detail.callees": "APPELÉS",
  "detail.community": "COMMUNAUTÉ",
  "detail.members": "membres",
  "detail.cohesion": "Cohésion",
  "detail.imports": "Imports",
  "detail.importedBy": "Importé par",
  "detail.inherits": "Hérite de",
  "detail.inheritedBy": "Hérité par",
  "analyze.openRepo": "Ouvrez un dépôt pour voir les analytics",
  "analyze.errorTitle": "Erreur d'analyse",
  "analyze.codeHealth": "Santé du code",
  "detail.cyclomaticComplexity": "Complexité cyclomatique",

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

  // ── Symbol Autocomplete ──
  "symbol.columnType": "Type",
  "symbol.columnName": "Nom",
  "symbol.columnFile": "Fichier",
  "symbol.columnLines": "Lignes",

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
  "settings.quickSetup": "Configuration rapide",
  "settings.baseUrl": "URL de base",
  "settings.model": "Modèle",
  "settings.apiKey": "Clé API",
  "settings.maxTokens": "Tokens max",
  "settings.thinking": "Réflexion / Raisonnement",
  "settings.thinkingHint": "Pour les modèles avec support de réflexion (Gemini, o1, etc.)",
  "settings.save": "Enregistrer",
  "settings.cancel": "Annuler",
  "settings.chatAiTitle": "Paramètres du Chat IA",
  "settings.securityNote": "Votre clé API est stockée localement et jamais partagée.",

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
  "status.nodes": "nœuds",
  "status.docs": "Docs",
  "status.browse": "Parcourir",
  "status.aiChat": "Chat Intelligence de Code",
  "status.reposSettings": "Dépôts & Paramètres",

  // ── Analyze Progress ──
  // ── Analyze Nav ──
  "analyze.nav.title": "Analytiques",
  "analyze.nav.overview": "Vue d'ensemble",
  "analyze.nav.hotspots": "Points chauds",
  "analyze.nav.coupling": "Couplage",
  "analyze.nav.ownership": "Propriété",
  "analyze.nav.coverage": "Couverture",
  "analyze.nav.diagrams": "Diagrammes",
  "analyze.nav.report": "Rapport",
  "analyze.nav.snapshots": "Instantanés",
  "analyze.nav.health": "Santé",
  "analyze.nav.processes": "Flux Métier",

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
  "analyze.insights": "Insights",
  "analyze.phase.processes": "Traçage des processus",
  "analyze.phase.enriching": "Enrichissement",
  "analyze.phase.complete": "Terminé",
  "analyze.phase.error": "Erreur",
  "analyze.files": "fichiers",
  "analyze.nodes": "nœuds",

  // ── Process Flows ──
  "analyze.processFlows": "Flux de Processus",
  "analyze.flowsDesc": "{count} processus métier identifiés.",
  "analyze.noFlowsTitle": "Aucun flux de processus trouvé",
  "analyze.noFlowsDesc": "Le traçage automatique nécessite des méthodes instrumentées ou des patterns métier spécifiques dans le code.",
  "analyze.stepCount": "{count} étapes",
  "analyze.flowDiagram": "Diagramme Interactif",
  "analyze.flowSteps": "Séquence des Étapes",
  "analyze.viewCode": "Voir le Code",
  "analyze.noStepsMessage": "Diagramme uniquement — aucune étape individuelle disponible pour ce processus.",

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
  "export.obsidianTitle": "Vault Obsidian (Cerveau Numérique)",
  "export.obsidianDesc": "Exporte le graphe de connaissances sous forme de notes Markdown interconnectées. Idéal pour la méthode Karpathy de gestion du patrimoine logiciel.",
  "export.generateObsidian": "Exporter le Vault Obsidian",
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

  // ── Communities Panel ──
  "communities.title": "Groupes fonctionnels",
  "communities.showAll": "Tout afficher",
  "communities.hint": "Clic pour isoler · Ctrl+Clic pour combiner",

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

  // ── Mode Bar ──
  "mode.explorer": "Explorateur",
  "mode.analyze": "Analyser",
  "mode.chat": "Chat",
  "mode.manage": "Gérer",
  "mode.commandPalette": "Palette de commandes",
  "mode.collapse": "Réduire",

  // ── Lens Selector ──
  "lens.all": "Tout",
  "lens.all.desc": "Afficher toutes les relations",
  "lens.calls": "Appels",
  "lens.calls.desc": "Appels de fonctions/méthodes",
  "lens.structure": "Structure",
  "lens.structure.desc": "HasMethod, HasProperty, ContainedIn",
  "lens.heritage": "Héritage",
  "lens.heritage.desc": "Extends, Implements",
  "lens.impact": "Impact",
  "lens.impact.desc": "Appels, Imports, DependsOn",
  "lens.deadCode": "Code mort",
  "lens.deadCode.desc": "Mettre en surbrillance les candidats code mort",
  "lens.tracing": "Traçage",
  "lens.tracing.desc": "Mettre en surbrillance les méthodes tracées",
  "lens.hotspots": "Points chauds",
  "lens.hotspots.desc": "Mettre en surbrillance les fichiers fréquemment modifiés",
  "lens.risk": "Risque",
  "lens.risk.desc": "Score composite : churn + code mort + traçage manquant",
  "lens.ariaLabel": "Filtre de lentille du graphe",

  // ── Cypher Presets ──
  "cypher.preset.allFunctions": "Toutes les fonctions",
  "cypher.preset.callGraph": "Graphe d'appels",
  "cypher.preset.controllers": "Contrôleurs",
  "cypher.preset.deadCode": "Code mort",
  "cypher.preset.topCallers": "Top appelants",
  "cypher.preset.services": "Services",
  "cypher.preset.communities": "Communautés",

  // ── Graph Zoom ──
  "zoom.in": "Zoom avant (Ctrl+=)",
  "zoom.out": "Zoom arrière (Ctrl+-)",
  "zoom.fit": "Ajuster la vue (Ctrl+0)",
  "zoom.inLabel": "Zoom avant",
  "zoom.outLabel": "Zoom arrière",
  "zoom.fitLabel": "Ajuster la vue",

  // ── Graph Toolbar extras ──
  "graph.truncated": "tronqué",
  "graph.granularity": "Niveau de granularité du graphe",
  "graph.collapseLegend": "Réduire la légende",

  // ── Command Palette ──
  "cmd.placeholder": "Tapez une commande ou recherchez…",
  "cmd.switchTo": "Basculer vers",
  "cmd.view": "Voir",
  "cmd.lens": "Lentille :",
  "cmd.openSettings": "Ouvrir les paramètres",
  "cmd.toggleDeepResearch": "Activer/désactiver la recherche approfondie",
  "cmd.group.modes": "Modes",
  "cmd.group.analyzeViews": "Vues d'analyse",
  "cmd.group.lenses": "Lentilles",
  "cmd.group.actions": "Actions",
  "cmd.group.userCommands": "Commandes utilisateur",
  // Nouvelles entrées récentes — groupe actions
  "cmd.renameRefactor": "Renommer (refactor)…",
  "cmd.exportHtml": "Exporter en HTML interactif",
  "cmd.generateWiki": "Générer le wiki (Markdown par module)",
  "cmd.generateWikiLlm": "Générer le wiki avec aperçus LLM (plus lent)",
  "cmd.openNotebooks": "Notebooks Cypher",
  "cmd.openDashboards": "Tableaux de bord personnalisés",
  "cmd.openWorkflows": "Éditeur de workflow",
  "cmd.openUserCommands": "Commandes slash personnalisées",
  "cmd.bundleExport": "Exporter le bundle de données utilisateur…",
  "cmd.bundleImport": "Importer un bundle de données utilisateur…",

  // ── Accessibility ──
  "a11y.skipToContent": "Aller au contenu principal",
  "a11y.codeIntelligencePlatform": "GitNexus — Plateforme d'intelligence de code",

  // ── Errors ──
  "error.somethingWentWrong": "Une erreur est survenue",
  "error.retry": "Réessayer",

  // ── Git Analytics ──
  "git.hotspots": "Points chauds",
  "git.coupling": "Couplage",
  "git.ownership": "Propriété",

  // ── Hotspots View ──
  "hotspots.loading": "Analyse des points chauds…",
  "hotspots.noData": "Aucune donnée de points chauds",
  "hotspots.noDataHint": "Assurez-vous que le dépôt possède un historique git pour analyser la fréquence de changement.",
  "hotspots.filesAnalyzed": "{0} fichiers analysés (90 derniers jours)",
  "hotspots.colRank": "#",
  "hotspots.colFile": "Fichier",
  "hotspots.colCommits": "Commits",
  "hotspots.colChurn": "Churn",
  "hotspots.colAuthors": "Auteurs",
  "hotspots.colScore": "Score",

  // ── Coupling View ──
  "coupling.loading": "Analyse du couplage…",
  "coupling.noData": "Aucun couplage temporel détecté",
  "coupling.noDataHint": "Les fichiers changent indépendamment. Le couplage est détecté quand des fichiers sont modifiés ensemble.",
  "coupling.pairsDetected": "{0} paires couplées détectées",
  "coupling.stronglyCoupled": "{0} fortement couplées (>70%)",
  "coupling.colRank": "#",
  "coupling.colFileA": "Fichier A",
  "coupling.colFileB": "Fichier B",
  "coupling.colShared": "Partagés",
  "coupling.colStrength": "Force",

  // ── Ownership View ──
  "ownership.loading": "Analyse de la propriété…",
  "ownership.noData": "Aucune donnée de propriété",
  "ownership.noDataHint": "Analysez un dépôt avec historique git pour voir la distribution par auteur.",
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

  // ── Dashboard ──
  "dashboard.nodeTypeDistribution": "Répartition par type de nœud",
  "dashboard.topConnectedNodes": "Nœuds les plus connectés",
  "dashboard.mostComplexFunctions": "Fonctions les plus complexes",
  "dashboard.healthy": "Sain",
  "dashboard.growing": "En croissance",
  "dashboard.small": "Petit",

  // ── Tooltips for common actions ──
  "tooltip.clickToOpen": "Cliquer pour ouvrir",
  "tooltip.rightClickForMenu": "Clic droit pour le menu contextuel",
  "tooltip.dragToMove": "Glisser pour repositionner",
  "tooltip.scrollToZoom": "Molette pour zoomer",

  // ── Filter modals ──
  "filters.searchFiles": "Rechercher des fichiers...",
  "filters.noFilesFound": "Aucun fichier trouvé",
  "filters.typeToSearchFiles": "Tapez pour rechercher...",
  "filters.searchSymbols": "Rechercher des symboles... (@fonction, #classe)",
  "filters.noSymbolsFound": "Aucun symbole trouvé",
  "filters.typeToSearchSymbols": "Tapez pour rechercher des symboles...",
  "filters.searchModules": "Rechercher des modules...",
  "filters.noModulesFound": "Aucun module trouvé",
  "filters.loadingModules": "Chargement des modules...",

  // ── Detail panel ──
  "detail.emptyHint": "Cliquez sur un nœud du graphe pour voir ses détails",
  "detail.loadingContext": "Chargement du contexte...",
  "detail.exported": "exporté",
  "detail.entryPoint": "Point d'entrée",
  "detail.traced": "Tracé",
  "detail.architectureLayer": "Couche architecturale",

  // ── Node hover card ──
  "hover.source": "Source",
  "hover.impact": "Impact",

  // ── Graph toolbar ──
  "toolbar.complexity": "Cplx",
  "toolbar.gitRange": "Git",

  // ── Source references ──
  "sources.title": "Sources",
  "sources.showMore": "Afficher {0} sources de plus",
  "sources.showFewer": "Afficher moins de sources",

  // ── Research plan viewer ──
  "research.planTitle": "Plan de recherche",
  "research.toolSearch": "Recherche de symboles",
  "research.toolContext": "Analyse du contexte",
  "research.toolRead": "Lecture de fichier",
  "research.toolCypher": "Requête graphe",
  "research.toolImpact": "Analyse d'impact",

  // ── Manage ──
  "manage.multiRepoOverview": "Vue multi-dépôts",

  // ── Comments ──
  "comments.emptyHint": "Aucune note. Ajoutez-en une pour garder du contexte pour votre équipe.",

  // ── Cypher ──
  "cypher.emptyQuery": "Rien à sauvegarder — écrivez une requête d'abord",

  // ── Rename ──
  "rename.searching": "Recherche…",
  "rename.preview": "Aperçu",

  // ── Common ──
  "common.loading": "Chargement",
  "common.noRows": "Aucune donnée à afficher",
  "common.retry": "Réessayer",
};

const dictionaries: Record<Locale, Record<string, TranslationValue>> = { en, fr };

// ─── Runtime state ───

function detectDefaultLocale(): Locale {
  if (typeof localStorage !== "undefined") {
    const saved = localStorage.getItem("gitnexus-locale") as Locale | null;
    if (saved && (saved === "en" || saved === "fr")) return saved;
  }
  if (typeof navigator !== "undefined" && navigator.language?.startsWith("fr")) return "fr";
  return "en";
}

let currentLocale: Locale = detectDefaultLocale();
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
