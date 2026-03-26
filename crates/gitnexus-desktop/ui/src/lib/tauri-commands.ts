/**
 * Type-safe wrappers for Tauri IPC commands.
 * These types mirror the Rust response types in src/types.rs.
 */
import { invoke } from "@tauri-apps/api/core";

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

// ─── Commands ────────────────────────────────────────────────────────────

export const commands = {
  // Repos
  listRepos: () => invoke<RepoInfo[]>("list_repos"),
  openRepo: (name: string) => invoke<RepoInfo>("open_repo", { name }),
  getActiveRepo: () => invoke<string | null>("get_active_repo"),
  analyzeRepo: (path: string) => invoke<string>("analyze_repo", { path }),
  generateDocs: (what: string, path: string) =>
    invoke<string>("generate_docs", { what, path }),

  // Graph
  getGraphData: (filter: GraphFilter) =>
    invoke<GraphPayload>("get_graph_data", { filter }),
  getSubgraph: (centerNodeId: string, depth?: number) =>
    invoke<GraphPayload>("get_subgraph", { centerNodeId, depth }),
  getNeighbors: (nodeId: string, direction?: string) =>
    invoke<GraphPayload>("get_neighbors", { nodeId, direction }),

  // Search
  searchSymbols: (query: string, limit?: number) =>
    invoke<SearchResult[]>("search_symbols", { query, limit }),
  searchAutocomplete: (prefix: string, limit?: number) =>
    invoke<SearchResult[]>("search_autocomplete", { prefix, limit }),

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
};
