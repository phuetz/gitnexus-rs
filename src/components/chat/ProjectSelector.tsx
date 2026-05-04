import { useCallback, useEffect, useState } from 'react';
import { ChevronDown, FolderOpen, RefreshCw, AlertCircle } from 'lucide-react';
import clsx from 'clsx';
import { mcpClient, type RepoInfo } from '../../api/mcp-client';
import { useChatStore } from '../../stores/chat-store';

export function ProjectSelector() {
  const selectedRepo = useChatStore((s) => s.selectedRepo);
  const setSelectedRepo = useChatStore((s) => s.setSelectedRepo);
  const [repos, setRepos] = useState<RepoInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState(false);

  const fetchRepos = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await mcpClient.listRepos();
      setRepos(list);
      if (list.length > 0 && !selectedRepo) {
        setSelectedRepo(list[0].name);
      }
      if (list.length === 0) setError('Aucun projet indexé.');
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [selectedRepo, setSelectedRepo]);

  useEffect(() => {
    // Initial fetch on mount — sync setState here is intentional (boot data load).
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void fetchRepos();
  }, [fetchRepos]);

  const label = selectedRepo ?? (loading ? 'Chargement…' : 'Aucun projet');

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((o) => !o)}
        className={clsx(
          'flex items-center gap-2 rounded-lg border border-neutral-800 bg-neutral-900/60 px-3 py-1.5 text-sm transition hover:bg-neutral-800',
          error ? 'text-amber-300' : 'text-neutral-200'
        )}
      >
        {error ? <AlertCircle size={14} /> : <FolderOpen size={14} className="text-neutral-500" />}
        <span className="max-w-[240px] truncate">{label}</span>
        <ChevronDown size={14} className="opacity-60" />
      </button>

      {open && (
        <div className="absolute left-0 top-full z-10 mt-1 max-h-80 w-72 overflow-y-auto rounded-lg border border-neutral-800 bg-neutral-900 shadow-xl">
          <div className="flex items-center justify-between border-b border-neutral-800 px-3 py-2">
            <span className="text-xs font-medium uppercase tracking-wider text-neutral-500">
              Projets indexés
            </span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                void fetchRepos();
              }}
              className="text-neutral-500 transition hover:text-neutral-300"
              title="Rafraîchir"
            >
              <RefreshCw size={12} className={loading ? 'animate-spin' : ''} />
            </button>
          </div>

          {error && (
            <div className="px-3 py-3 text-xs text-amber-300">
              <div className="font-medium">Erreur</div>
              <div className="mt-1 break-words text-amber-300/80">{error}</div>
              <div className="mt-2 text-neutral-500">
                Vérifie que <code className="rounded bg-neutral-800 px-1">gitnexus serve --port 3000</code> tourne.
              </div>
            </div>
          )}

          {!error && repos.length === 0 && !loading && (
            <div className="px-3 py-3 text-xs text-neutral-500">
              Aucun projet. Lance <code className="rounded bg-neutral-800 px-1">gitnexus analyze &lt;path&gt;</code> d'abord.
            </div>
          )}

          {repos.map((repo) => (
            <button
              key={repo.name}
              onClick={() => {
                setSelectedRepo(repo.name);
                setOpen(false);
              }}
              className={clsx(
                'flex w-full flex-col items-start gap-0.5 px-3 py-2 text-left text-sm transition',
                repo.name === selectedRepo
                  ? 'bg-purple-600/20 text-purple-200'
                  : 'text-neutral-300 hover:bg-neutral-800'
              )}
            >
              <div className="flex w-full items-center justify-between">
                <span className="truncate font-medium">{repo.name}</span>
                {repo.indexedAt && (
                  <span className="ml-2 shrink-0 text-[10px] text-neutral-500">
                    {new Date(repo.indexedAt).toLocaleDateString()}
                  </span>
                )}
              </div>
              {repo.path && (
                <span className="truncate text-[11px] text-neutral-500">{repo.path}</span>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
