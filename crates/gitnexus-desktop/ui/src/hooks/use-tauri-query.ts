import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { commands } from "../lib/tauri-commands";
import type { GraphFilter } from "../lib/tauri-commands";

export function useRepos() {
  return useQuery({
    queryKey: ["repos"],
    queryFn: () => commands.listRepos(),
  });
}

export function useOpenRepo() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => commands.openRepo(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["graph"] });
      queryClient.invalidateQueries({ queryKey: ["file-tree"] });
    },
  });
}

export function useGraphData(filter: GraphFilter, enabled: boolean = true) {
  return useQuery({
    queryKey: ["graph", filter],
    queryFn: () => commands.getGraphData(filter),
    enabled,
    staleTime: Infinity,
  });
}

export function useSubgraph(centerNodeId: string | null, depth?: number) {
  return useQuery({
    queryKey: ["subgraph", centerNodeId, depth],
    queryFn: () => commands.getSubgraph(centerNodeId!, depth),
    enabled: !!centerNodeId,
    staleTime: Infinity,
  });
}

export function useSearchSymbols(query: string, enabled: boolean = true) {
  return useQuery({
    queryKey: ["search", query],
    queryFn: () => commands.searchSymbols(query),
    enabled: enabled && query.length > 0,
    staleTime: 30_000,
  });
}

export function useSymbolContext(nodeId: string | null) {
  return useQuery({
    queryKey: ["context", nodeId],
    queryFn: () => commands.getSymbolContext(nodeId!),
    enabled: !!nodeId,
    staleTime: Infinity,
  });
}

export function useImpactAnalysis(
  targetId: string | null,
  direction?: string,
  maxDepth?: number
) {
  return useQuery({
    queryKey: ["impact", targetId, direction, maxDepth],
    queryFn: () => commands.getImpactAnalysis(targetId!, direction, maxDepth),
    enabled: !!targetId,
    staleTime: Infinity,
  });
}

export function useFileTree(enabled: boolean = true) {
  return useQuery({
    queryKey: ["file-tree"],
    queryFn: () => commands.getFileTree(),
    enabled,
    staleTime: Infinity,
  });
}

export function useFileContent(
  filePath: string | null,
  startLine?: number,
  endLine?: number
) {
  return useQuery({
    queryKey: ["file-content", filePath, startLine, endLine],
    queryFn: () => commands.readFileContent(filePath!, startLine, endLine),
    enabled: !!filePath,
    staleTime: Infinity,
  });
}
