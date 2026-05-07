import { useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import {
  Braces,
  FileCode2,
  Folder,
  GitBranch,
  Loader2,
  MessageSquarePlus,
  Search,
  X,
} from 'lucide-react';
import {
  mcpClient,
  type FileTreeNode,
  type GraphNode,
  type GraphPayload,
  type SourceContent,
  type SymbolSearchResult,
} from '../../api/mcp-client';
import { useChatStore } from '../../stores/chat-store';

type WorkspaceTab = 'sources' | 'graph';

interface SourceTarget {
  path: string;
  startLine?: number;
  endLine?: number;
}

interface WorkspacePanelProps {
  onClose: () => void;
}

export function WorkspacePanel({ onClose }: WorkspacePanelProps) {
  const selectedRepo = useChatStore((s) => s.selectedRepo);
  const selectedRepoName = useChatStore((s) => s.selectedRepoName);
  const [tab, setTab] = useState<WorkspaceTab>('sources');
  const [sourceTarget, setSourceTarget] = useState<SourceTarget | null>(null);

  const openSource = useCallback((target: SourceTarget) => {
    setSourceTarget(target);
    setTab('sources');
  }, []);

  return (
    <aside className="flex h-full w-[min(520px,42vw)] min-w-[360px] flex-col border-l border-neutral-900 bg-neutral-950">
      <header className="flex min-h-12 items-center gap-2 border-b border-neutral-900 px-3">
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <Braces className="h-4 w-4 text-violet-300" aria-hidden />
          <div className="min-w-0">
            <div className="truncate text-sm font-medium text-neutral-100">Explorateur</div>
            <div className="truncate text-xs text-neutral-500">{selectedRepoName ?? selectedRepo ?? 'Aucun projet'}</div>
          </div>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="rounded-md border border-neutral-800 p-1.5 text-neutral-400 hover:bg-neutral-900 hover:text-neutral-100"
          aria-label="Fermer l'explorateur"
          title="Fermer"
        >
          <X className="h-4 w-4" aria-hidden />
        </button>
      </header>

      <div className="flex border-b border-neutral-900 px-2 py-2 text-xs">
        <WorkspaceTabButton active={tab === 'sources'} onClick={() => setTab('sources')} icon={<FileCode2 className="h-3.5 w-3.5" />}>
          Sources
        </WorkspaceTabButton>
        <WorkspaceTabButton active={tab === 'graph'} onClick={() => setTab('graph')} icon={<GitBranch className="h-3.5 w-3.5" />}>
          Graphe
        </WorkspaceTabButton>
      </div>

      {!selectedRepo ? (
        <div className="flex flex-1 items-center justify-center p-6 text-center text-sm text-neutral-500">
          Selectionne un projet indexe pour naviguer dans ses sources et son graphe.
        </div>
      ) : tab === 'sources' ? (
        <SourceExplorer repo={selectedRepo} target={sourceTarget} />
      ) : (
        <GraphNavigator repo={selectedRepo} onOpenSource={openSource} />
      )}
    </aside>
  );
}

function WorkspaceTabButton({
  active,
  onClick,
  icon,
  children,
}: {
  active: boolean;
  onClick: () => void;
  icon: ReactNode;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`mr-1 inline-flex items-center gap-1.5 rounded-md border px-2.5 py-1.5 ${
        active
          ? 'border-neutral-700 bg-neutral-900 text-neutral-100'
          : 'border-transparent text-neutral-500 hover:bg-neutral-900 hover:text-neutral-200'
      }`}
    >
      {icon}
      {children}
    </button>
  );
}

function SourceExplorer({ repo, target }: { repo: string; target: SourceTarget | null }) {
  const setInputDraft = useChatStore((s) => s.setInputDraft);
  const [treeState, setTreeState] = useState<{
    repo: string | null;
    files: FileTreeNode[];
    error: string | null;
  }>({ repo: null, files: [], error: null });
  const [filter, setFilter] = useState('');
  const [manualTarget, setManualTarget] = useState<SourceTarget | null>(null);
  const activeTarget = manualTarget ?? target;
  const activePath = activeTarget?.path ?? null;
  const activeStartLine = activeTarget?.startLine;
  const activeEndLine = activeTarget?.endLine;
  const activeTargetKey = activePath
    ? `${repo}:${activePath}:${activeStartLine ?? ''}:${activeEndLine ?? ''}`
    : null;
  const [sourceState, setSourceState] = useState<{
    key: string | null;
    source: SourceContent | null;
    error: string | null;
  }>({ key: null, source: null, error: null });

  useEffect(() => {
    let alive = true;
    void mcpClient
      .fileTree(repo)
      .then((files) => {
        if (alive) setTreeState({ repo, files, error: null });
      })
      .catch((error) => {
        if (alive) {
          setTreeState({
            repo,
            files: [],
            error: error instanceof Error ? error.message : String(error),
          });
        }
      });
    return () => {
      alive = false;
    };
  }, [repo]);

  useEffect(() => {
    if (!activePath || !activeTargetKey) return;
    let alive = true;
    void mcpClient
      .source(repo, activePath, {
        start: activeStartLine,
        end: activeEndLine,
      })
      .then((content) => {
        if (alive) setSourceState({ key: activeTargetKey, source: content, error: null });
      })
      .catch((error) => {
        if (alive) {
          setSourceState({
            key: activeTargetKey,
            source: null,
            error: error instanceof Error ? error.message : String(error),
          });
        }
      });
    return () => {
      alive = false;
    };
  }, [
    activeEndLine,
    activePath,
    activeStartLine,
    activeTargetKey,
    repo,
  ]);

  const openFile = useCallback((path: string, startLine?: number, endLine?: number) => {
    setManualTarget({ path, startLine, endLine });
  }, []);

  const treeLoading = treeState.repo !== repo;
  const treeError = treeLoading ? null : treeState.error;
  const filteredTree = useMemo(() => filterTree(treeState.files, filter), [treeState.files, filter]);
  const sourceLoading = !!activeTargetKey && sourceState.key !== activeTargetKey;
  const source = sourceLoading ? null : sourceState.source;
  const sourceError = sourceLoading ? null : sourceState.error;

  const askAboutSource = () => {
    if (!source) return;
    const range =
      source.startLine && source.endLine
        ? ` lignes ${source.startLine}-${source.endLine}`
        : '';
    setInputDraft(`Explique le fichier ${source.path}${range} et ses liens avec le graphe GitNexus.`);
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="border-b border-neutral-900 p-3">
        <label className="flex items-center gap-2 rounded-md border border-neutral-800 bg-neutral-900/70 px-2 py-1.5 text-xs text-neutral-400">
          <Search className="h-3.5 w-3.5" aria-hidden />
          <input
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
            placeholder="Filtrer les fichiers..."
            className="min-w-0 flex-1 bg-transparent text-neutral-100 outline-none placeholder:text-neutral-600"
          />
        </label>
      </div>
      <div className="grid min-h-0 flex-1 grid-cols-[190px_minmax(0,1fr)]">
        <div className="min-h-0 overflow-auto border-r border-neutral-900 p-2 text-xs">
          {treeLoading ? (
            <LoadingLine label="Chargement des sources..." />
          ) : treeError ? (
            <ErrorText message={treeError} />
          ) : filteredTree.length === 0 ? (
            <div className="p-3 text-neutral-600">Aucun fichier.</div>
          ) : (
            <FileTree nodes={filteredTree} onOpenFile={(path) => void openFile(path)} />
          )}
        </div>
        <div className="min-h-0 overflow-hidden">
          {sourceLoading ? (
            <div className="flex h-full items-center justify-center">
              <LoadingLine label="Lecture du fichier..." />
            </div>
          ) : sourceError ? (
            <div className="p-4">
              <ErrorText message={sourceError} />
            </div>
          ) : source ? (
            <div className="flex h-full flex-col">
              <div className="flex min-h-10 items-center gap-2 border-b border-neutral-900 px-3 text-xs">
                <FileCode2 className="h-3.5 w-3.5 shrink-0 text-violet-300" aria-hidden />
                <span className="min-w-0 flex-1 truncate font-mono text-neutral-200">{source.path}</span>
                <span className="shrink-0 text-neutral-600">
                  {source.totalLines} lignes{source.language ? ` - ${source.language}` : ''}
                </span>
                <button
                  type="button"
                  onClick={askAboutSource}
                  className="rounded-md border border-neutral-800 px-2 py-1 text-neutral-300 hover:bg-neutral-900"
                  title="Envoyer ce contexte au chat"
                >
                  <MessageSquarePlus className="h-3.5 w-3.5" aria-hidden />
                </button>
              </div>
              <SourceCode source={source} />
            </div>
          ) : (
            <div className="flex h-full items-center justify-center p-6 text-center text-sm text-neutral-600">
              Choisis un fichier pour l'afficher ici.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function FileTree({
  nodes,
  onOpenFile,
  depth = 0,
}: {
  nodes: FileTreeNode[];
  onOpenFile: (path: string) => void;
  depth?: number;
}) {
  return (
    <div className="space-y-0.5">
      {nodes.map((node) => (
        <div key={node.path}>
          <button
            type="button"
            onClick={() => {
              if (!node.isDir) onOpenFile(node.path);
            }}
            className={`flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left ${
              node.isDir ? 'text-neutral-500' : 'text-neutral-300 hover:bg-neutral-900 hover:text-neutral-100'
            }`}
            style={{ paddingLeft: `${depth * 10 + 6}px` }}
            disabled={node.isDir}
            title={node.path}
          >
            {node.isDir ? (
              <Folder className="h-3.5 w-3.5 shrink-0 text-amber-300/70" aria-hidden />
            ) : (
              <FileCode2 className="h-3.5 w-3.5 shrink-0 text-neutral-500" aria-hidden />
            )}
            <span className="truncate">{node.name}</span>
          </button>
          {node.isDir && node.children.length > 0 && (
            <FileTree nodes={node.children} onOpenFile={onOpenFile} depth={depth + 1} />
          )}
        </div>
      ))}
    </div>
  );
}

function SourceCode({ source }: { source: SourceContent }) {
  const lines = source.content ? source.content.split('\n') : [];
  const start = source.startLine || 1;

  return (
    <pre className="min-h-0 flex-1 overflow-auto bg-neutral-950 p-0 text-[11px] leading-5 text-neutral-200">
      <code>
        {lines.map((line, index) => (
          <div key={`${source.path}-${start + index}`} className="flex hover:bg-neutral-900/70">
            <span className="w-12 shrink-0 select-none border-r border-neutral-900 pr-3 text-right text-neutral-700">
              {start + index}
            </span>
            <span className="min-w-0 flex-1 whitespace-pre px-3 font-mono">{line || ' '}</span>
          </div>
        ))}
        {source.truncated && (
          <div className="border-t border-neutral-900 px-3 py-2 text-xs text-amber-300">
            Extrait limite aux premieres lignes demandees.
          </div>
        )}
      </code>
    </pre>
  );
}

function GraphNavigator({
  repo,
  onOpenSource,
}: {
  repo: string;
  onOpenSource: (target: SourceTarget) => void;
}) {
  const setInputDraft = useChatStore((s) => s.setInputDraft);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SymbolSearchResult[]>([]);
  const [selected, setSelected] = useState<GraphNode | null>(null);
  const [graph, setGraph] = useState<GraphPayload | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const runSearch = async () => {
    if (!query.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const symbols = await mcpClient.symbols(repo, query.trim(), 25);
      setResults(symbols);
      setGraph(null);
      setSelected(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const openNeighborhood = async (symbol: SymbolSearchResult) => {
    setLoading(true);
    setError(null);
    try {
      const nextGraph = await mcpClient.graphNeighborhood(repo, symbol.nodeId, 2);
      setGraph(nextGraph);
      setSelected(nextGraph.nodes.find((node) => node.id === symbol.nodeId) ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const askAboutNode = (node: GraphNode) => {
    setInputDraft(
      `Explique le noeud ${node.name} (${node.label}) dans ${node.filePath}, ses voisins dans le graphe et les risques de modification.`
    );
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="border-b border-neutral-900 p-3">
        <form
          className="flex gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            void runSearch();
          }}
        >
          <label className="flex min-w-0 flex-1 items-center gap-2 rounded-md border border-neutral-800 bg-neutral-900/70 px-2 py-1.5 text-xs text-neutral-400">
            <Search className="h-3.5 w-3.5" aria-hidden />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Chercher une classe, methode, action..."
              className="min-w-0 flex-1 bg-transparent text-neutral-100 outline-none placeholder:text-neutral-600"
            />
          </label>
          <button
            type="submit"
            className="rounded-md border border-neutral-800 px-3 text-xs text-neutral-200 hover:bg-neutral-900"
          >
            Chercher
          </button>
        </form>
      </div>
      {error && (
        <div className="border-b border-red-950 bg-red-950/20 p-3">
          <ErrorText message={error} />
        </div>
      )}
      <div className="grid min-h-0 flex-1 grid-cols-[190px_minmax(0,1fr)]">
        <div className="min-h-0 overflow-auto border-r border-neutral-900 p-2 text-xs">
          {loading && results.length === 0 ? (
            <LoadingLine label="Recherche..." />
          ) : results.length === 0 ? (
            <div className="p-3 text-neutral-600">Lance une recherche pour ouvrir un voisinage graphe.</div>
          ) : (
            <div className="space-y-1">
              {results.map((symbol) => (
                <button
                  key={symbol.nodeId}
                  type="button"
                  onClick={() => void openNeighborhood(symbol)}
                  className="w-full rounded-md border border-neutral-900 bg-neutral-950 p-2 text-left hover:border-neutral-700 hover:bg-neutral-900"
                >
                  <div className="truncate font-medium text-neutral-100">{symbol.name}</div>
                  <div className="truncate text-[11px] text-neutral-500">
                    {symbol.label} - {symbol.filePath}
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
        <div className="min-h-0 overflow-auto p-3">
          {loading && results.length > 0 && <LoadingLine label="Chargement du voisinage..." />}
          {!graph ? (
            <div className="flex h-full items-center justify-center p-6 text-center text-sm text-neutral-600">
              Selectionne un symbole pour afficher son voisinage.
            </div>
          ) : (
            <div className="space-y-3">
              <div className="rounded-lg border border-neutral-800 bg-neutral-900/40 p-3">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="text-sm font-medium text-neutral-100">
                      {selected?.name ?? 'Voisinage graphe'}
                    </div>
                    <div className="mt-1 text-xs text-neutral-500">
                      {graph.stats.nodeCount} noeuds - {graph.stats.edgeCount} relations
                      {graph.stats.truncated ? ' - tronque' : ''}
                    </div>
                  </div>
                  {selected && (
                    <button
                      type="button"
                      onClick={() => askAboutNode(selected)}
                      className="rounded-md border border-neutral-800 p-1.5 text-neutral-300 hover:bg-neutral-800"
                      title="Demander au chat"
                    >
                      <MessageSquarePlus className="h-3.5 w-3.5" aria-hidden />
                    </button>
                  )}
                </div>
              </div>
              <div className="space-y-2">
                {graph.nodes
                  .slice()
                  .sort((a, b) => (a.depth ?? 0) - (b.depth ?? 0) || a.name.localeCompare(b.name))
                  .map((node) => (
                    <div key={node.id} className="rounded-lg border border-neutral-900 bg-neutral-950 p-3">
                      <div className="flex items-start justify-between gap-3">
                        <div className="min-w-0">
                          <div className="truncate text-sm font-medium text-neutral-100">{node.name}</div>
                          <div className="mt-1 truncate text-xs text-neutral-500">
                            {node.label}
                            {typeof node.depth === 'number' ? ` - distance ${node.depth}` : ''} - {node.filePath}
                          </div>
                        </div>
                        {node.filePath && (
                          <button
                            type="button"
                            onClick={() =>
                              onOpenSource({
                                path: node.filePath,
                                startLine: node.startLine,
                                endLine: node.endLine,
                              })
                            }
                            className="rounded-md border border-neutral-800 px-2 py-1 text-xs text-neutral-300 hover:bg-neutral-900"
                          >
                            Source
                          </button>
                        )}
                      </div>
                    </div>
                  ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function filterTree(nodes: FileTreeNode[], query: string): FileTreeNode[] {
  const q = query.trim().toLowerCase();
  if (!q) return nodes;
  const result: FileTreeNode[] = [];
  for (const node of nodes) {
    const childMatches = node.isDir ? filterTree(node.children, query) : [];
    const selfMatches = node.name.toLowerCase().includes(q) || node.path.toLowerCase().includes(q);
    if (selfMatches || childMatches.length > 0) {
      result.push({
        ...node,
        children: node.isDir && childMatches.length > 0 ? childMatches : node.children,
      });
    }
  }
  return result;
}

function LoadingLine({ label }: { label: string }) {
  return (
    <div className="flex items-center gap-2 p-3 text-xs text-neutral-500">
      <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden />
      {label}
    </div>
  );
}

function ErrorText({ message }: { message: string }) {
  return <div className="whitespace-pre-wrap text-xs text-red-300">{message}</div>;
}
