/**
 * Type-safe wrappers for Tauri IPC commands.
 * These types mirror the Rust response types in src/types.rs.
 */
import { safeInvoke as invoke } from "./tauri-env";

// ─── Types ───────────────────────────────────────────────────────────────

export interface RepoInfo {
  name: string;
  path: string;
  indexedAt: string;
  lastCommit: string;
  files?: number;
  nodes?: number;
  edges?: number;
  communities?: number;
}

export interface CytoNode {
  id: string;
  label: string;
  name: string;
  filePath: string;
  startLine?: number;
  endLine?: number;
  isExported?: boolean;
  community?: string;
  language?: string;
  description?: string;
  parameterCount?: number;
  returnType?: string;
  layerType?: string;
  entryPointScore?: number;
  entryPointReason?: string;
  isTraced?: boolean;
  traceCallCount?: number;
  isDeadCandidate?: boolean;
  complexity?: number;
  depth?: number;
}

export interface CytoEdge {
  id: string;
  source: string;
  target: string;
  relType: string;
  confidence: number;
}

export interface GraphStats {
  nodeCount: number;
  edgeCount: number;
  truncated: boolean;
}

export interface GraphPayload {
  nodes: CytoNode[];
  edges: CytoEdge[];
  stats: GraphStats;
}

export type ZoomLevel = "package" | "module" | "symbol";

export interface GraphFilter {
  zoomLevel: ZoomLevel;
  labels?: string[];
  filePaths?: string[];
  maxNodes?: number;
}

export interface SearchResult {
  nodeId: string;
  name: string;
  label: string;
  filePath: string;
  score: number;
  startLine?: number;
  endLine?: number;
}

export interface RelatedNode {
  id: string;
  name: string;
  label: string;
  filePath: string;
}

export interface CommunityInfo {
  id: string;
  name: string;
  description?: string;
  memberCount?: number;
  cohesion?: number;
}

export interface SymbolContext {
  node: CytoNode;
  callers: RelatedNode[];
  callees: RelatedNode[];
  imports: RelatedNode[];
  importedBy: RelatedNode[];
  inherits: RelatedNode[];
  inheritedBy: RelatedNode[];
  community?: CommunityInfo;
}

export interface ImpactNode {
  node: CytoNode;
  depth: number;
  path: string[];
}

export interface ImpactSummary {
  upstreamCount: number;
  downstreamCount: number;
  affectedFilesCount: number;
  maxDepth: number;
}

export interface ImpactResult {
  target: CytoNode;
  upstream: ImpactNode[];
  downstream: ImpactNode[];
  graph: GraphPayload;
  affectedFiles: string[];
  summary: ImpactSummary;
}

export interface FileTreeNode {
  name: string;
  path: string;
  isDir: boolean;
  children: FileTreeNode[];
}

export interface FileContent {
  path: string;
  content: string;
  language?: string;
  totalLines: number;
}

// ─── Documentation Types ────────────────────────────────────────────

export interface DocPage {
  id: string;
  title: string;
  path?: string;
  icon?: string;
  children?: DocPage[];
}

export interface DocStats {
  files: number;
  nodes: number;
  edges: number;
  modules: number;
}

export interface DocIndex {
  title: string;
  generatedAt: string;
  stats: DocStats;
  pages: DocPage[];
}

export interface DocContent {
  path: string;
  content: string;
  title: string;
}

// ─── Chat Types ─────────────────────────────────────────────────────

export interface ChatMessage {
  role: string;
  content: string;
}

export interface ChatRequest {
  question: string;
  history: ChatMessage[];
}

export interface ChatSource {
  nodeId: string;
  symbolName: string;
  symbolType: string;
  filePath: string;
  startLine?: number;
  endLine?: number;
  snippet?: string;
  callers?: string[];
  callees?: string[];
  community?: string;
  relevanceScore: number;
}

export interface ChatResponse {
  answer: string;
  sources: ChatSource[];
  model?: string | null;
}

export interface ChatConfig {
  provider: string;
  apiKey: string;
  baseUrl: string;
  model: string;
  maxTokens: number;
  reasoningEffort: string;
}

// ─── Chat Intelligence Types ────────────────────────────────────────

export type QueryComplexity = "simple" | "medium" | "complex";

export interface ChatContextFilter {
  files: string[];
  symbols: string[];
  modules: string[];
  languages: string[];
  labels: string[];
}

export interface ChatSmartRequest {
  question: string;
  history: ChatMessage[];
  filters?: ChatContextFilter;
  deepResearch?: boolean;
}

export interface QueryAnalysis {
  complexity: QueryComplexity;
  suggestedTools: string[];
  estimatedSteps: number;
  reasoning: string;
  keywords: string[];
  needsCrossFile: boolean;
  needsImpact: boolean;
}

export type PlanStatus = "pending" | "running" | "completed" | "failed";
export type StepStatus = "pending" | "running" | "completed" | "failed" | "skipped";

export interface StepResult {
  summary: string;
  sources: ChatSource[];
  data?: unknown;
  durationMs: number;
}

export interface ResearchStep {
  id: string;
  order: number;
  tool: string;
  description: string;
  params: Record<string, unknown>;
  dependsOn: string[];
  status: StepStatus;
  result?: StepResult;
}

export interface ResearchPlan {
  id: string;
  query: string;
  analysis: QueryAnalysis;
  steps: ResearchStep[];
  status: PlanStatus;
}

export interface ChatSmartResponse {
  answer: string;
  sources: ChatSource[];
  model?: string | null;
  plan?: ResearchPlan;
  complexity: QueryComplexity;
}

export interface ProcessStep {
  nodeId: string;
  name: string;
  label: string;
  filePath: string;
}

export interface ProcessFlow {
  id: string;
  name: string;
  processType: string;
  stepCount: number;
  steps: ProcessStep[];
  mermaid: string;
}

export interface GitHotspot {
  path: string;
  commitCount: number;
  linesAdded: number;
  linesRemoved: number;
  churn: number;
  score: number;
  lastModified: string;
  authorCount: number;
}

export interface GitCoupling {
  fileA: string;
  fileB: string;
  sharedCommits: number;
  couplingStrength: number;
}

export interface GitAuthorContribution {
  name: string;
  email: string;
  commits: number;
  pct: number;
}

export interface GitOwnership {
  path: string;
  primaryAuthor: string;
  ownershipPct: number;
  authorCount: number;
  authors: GitAuthorContribution[];
}

export interface CodeHealth {
  overallScore: number;
  grade: string;
  hotspotScore: number;
  cohesionScore: number;
  tracingCoverage: number;
  ownershipScore: number;
  fileCount: number;
  nodeCount: number;
  edgeCount: number;
  avgComplexity: number;
  maxComplexity: number;
}

export interface FeatureInfo {
  id: string;
  name: string;
  description?: string;
  memberCount: number;
  cohesion?: number;
}

export interface FileQuickPick {
  path: string;
  name: string;
  language?: string;
  symbolCount: number;
}

export interface SymbolQuickPick {
  nodeId: string;
  name: string;
  kind: string;
  filePath: string;
  container?: string;
  startLine?: number;
}

export interface ModuleQuickPick {
  communityId: string;
  name: string;
  memberCount: number;
  description?: string;
}

// ─── Pipeline Progress Types ─────────────────────────────────────────────

export type PipelinePhase =
  | "idle"
  | "extracting"
  | "structure"
  | "parsing"
  | "imports"
  | "calls"
  | "heritage"
  | "communities"
  | "processes"
  | "enriching"
  | "complete"
  | "error";

export interface PipelineStats {
  filesProcessed: number;
  totalFiles: number;
  nodesCreated: number;
}

export interface PipelineProgress {
  phase: PipelinePhase;
  percent: number;
  message: string;
  detail?: string;
  stats?: PipelineStats;
}

/** Human-readable labels for each pipeline phase */
export const PHASE_LABELS: Record<PipelinePhase, string> = {
  idle: "Idle",
  extracting: "Extracting",
  structure: "Scanning files",
  parsing: "Parsing AST",
  imports: "Resolving imports",
  calls: "Analyzing calls",
  heritage: "Class hierarchy",
  communities: "Detecting communities",
  processes: "Tracing processes",
  enriching: "Enriching",
  complete: "Complete",
  error: "Error",
};

/** Overall weight of each phase for global progress bar (must sum to 100) */
export const PHASE_WEIGHTS: Partial<Record<PipelinePhase, number>> = {
  structure: 5,
  parsing: 45,
  imports: 15,
  calls: 15,
  heritage: 5,
  communities: 10,
  processes: 5,
};

// ─── Commands ────────────────────────────────────────────────────────────

export const commands = {
  // Repos
  listRepos: () => invoke<RepoInfo[]>("list_repos"),
  openRepo: (name: string) => invoke<RepoInfo>("open_repo", { name }),
  analyzeRepo: (path: string) => invoke<string>("analyze_repo", { path }),
  generateDocs: (what: string, path: string) =>
    invoke<string>("generate_docs", { what, path }),

  // Graph
  getGraphData: (filter: GraphFilter) =>
    invoke<GraphPayload>("get_graph_data", { filter }),
  getSubgraph: (centerNodeId: string, depth?: number) =>
    invoke<GraphPayload>("get_subgraph", { centerNodeId, depth }),

  // Search
  searchSymbols: (query: string, limit?: number) =>
    invoke<SearchResult[]>("search_symbols", { query, limit }),

  // Context
  getSymbolContext: (nodeId: string) =>
    invoke<SymbolContext>("get_symbol_context", { nodeId }),

  // Impact
  getImpactAnalysis: (
    targetId: string,
    direction?: string,
    maxDepth?: number
  ) =>
    invoke<ImpactResult>("get_impact_analysis", {
      targetId,
      direction,
      maxDepth,
    }),

  // Files
  getFileTree: () => invoke<FileTreeNode[]>("get_file_tree"),
  readFileContent: (
    filePath: string,
    startLine?: number,
    endLine?: number
  ) =>
    invoke<FileContent>("read_file_content", { filePath, startLine, endLine }),

  // Cypher
  executeCypher: (query: string) =>
    invoke<unknown[]>("execute_cypher", { query }),

  // Documentation
  getDocIndex: () =>
    invoke<DocIndex | null>("get_doc_index"),
  readDoc: (path: string) =>
    invoke<DocContent>("read_doc", { path }),

  // Chat Q&A
  chatAsk: (request: ChatRequest) =>
    invoke<ChatResponse>("chat_ask", { request }),
  chatGetConfig: () =>
    invoke<ChatConfig>("chat_get_config"),
  chatSetConfig: (config: ChatConfig) =>
    invoke<void>("chat_set_config", { config }),

  // Chat Intelligence (Executor)
  chatExecuteStep: (planId: string, stepId: string) =>
    invoke<StepResult>("chat_execute_step", { planId, stepId }),
  chatExecutePlan: (request: ChatSmartRequest) =>
    invoke<ChatSmartResponse>("chat_execute_plan", { request }),

  // Quick Picks (IDE-style search)
  chatPickFiles: (query: string, limit?: number) =>
    invoke<FileQuickPick[]>("chat_pick_files", { query, limit }),
  chatPickSymbols: (query: string, fileFilter?: string, limit?: number) =>
    invoke<SymbolQuickPick[]>("chat_pick_symbols", { query, fileFilter, limit }),
  chatPickModules: (query: string, limit?: number) =>
    invoke<ModuleQuickPick[]>("chat_pick_modules", { query, limit }),

  // Export
  exportDocsDocx: () =>
    invoke<string>("export_docs_docx"),
  getAspnetStats: () =>
    invoke<AspNetStats>("get_aspnet_stats"),

  // Process Flows
  getProcessFlows: () =>
    invoke<ProcessFlow[]>("get_process_flows"),

  // Code Health
  getCodeHealth: () =>
    invoke<CodeHealth>("get_code_health"),

  // Git Analytics
  getHotspots: (sinceDays?: number) =>
    invoke<GitHotspot[]>("get_hotspots", { sinceDays }),
  getCoupling: (minShared?: number) =>
    invoke<GitCoupling[]>("get_coupling", { minShared }),
  getOwnership: () =>
    invoke<GitOwnership[]>("get_ownership"),

  // Features (community-based)
  getFeatures: () =>
    invoke<FeatureInfo[]>("get_features"),

  // Coverage & Diagrams
  getCoverageStats: () =>
    invoke<CoverageStats>("get_coverage_stats"),
  getDiagram: (target: string, diagramType?: string) =>
    invoke<DiagramResult>("get_diagram", { target, diagramType }),
};

// ─── Coverage ────────────────────────────────────────────────────────

export interface CoverageStats {
  totalMethods: number;
  tracedMethods: number;
  deadCodeCandidates: number;
  coveragePct: number;
  deadMethods: DeadMethod[];
}

export interface DeadMethod {
  name: string;
  filePath: string;
  className: string | null;
  nodeId: string;
}

// ─── Diagram ─────────────────────────────────────────────────────────

export interface DiagramResult {
  mermaid: string;
  targetName: string;
  targetLabel: string;
  diagramType: string;
}

// ─── ASP.NET Stats ────────────────────────────────────────────────────

export interface AspNetStats {
  controllers: number;
  actions: number;
  apiEndpoints: number;
  views: number;
  entities: number;
  dbContexts: number;
  areas: number;
}
