import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ChevronDown, FolderOpen, RefreshCw, AlertCircle, Search, Copy, Check } from 'lucide-react';
import clsx from 'clsx';
import { mcpClient, type RepoInfo } from '../../api/mcp-client';
import { useChatStore } from '../../stores/chat-store';
import { parseIndexedAt } from '../../utils/dates';
import { copyTextToClipboard } from '../../utils/clipboard';

function repoSelectionValue(repo: RepoInfo): string {
  return repo.id || repo.name;
}

function repoMatchesSelection(repo: RepoInfo, selection: string | null): boolean {
  if (!selection) return false;
  return repoSelectionValue(repo) === selection || repo.name === selection;
}

function shortRepoId(repo: RepoInfo): string | null {
  if (!repo.id) return null;
  return repo.id.replace(/^repo_/, '').slice(0, 8);
}

function countRepoNames(repos: RepoInfo[]): Record<string, number> {
  return repos.reduce<Record<string, number>>((acc, repo) => {
    acc[repo.name] = (acc[repo.name] ?? 0) + 1;
    return acc;
  }, {});
}

function repoDisplayName(repo: RepoInfo, duplicateNames: Record<string, number>): string {
  const idSuffix = duplicateNames[repo.name] > 1 ? shortRepoId(repo) : null;
  return `${repo.name}${idSuffix ? ` · ${idSuffix}` : ''}`;
}

export function ProjectSelector() {
  const selectedRepo = useChatStore((s) => s.selectedRepo);
  const setSelectedRepo = useChatStore((s) => s.setSelectedRepo);
  const [repos, setRepos] = useState<RepoInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [copiedError, setCopiedError] = useState(false);
  const copiedErrorTimer = useRef<number | null>(null);

  const fetchRepos = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await mcpClient.listRepos();
      setRepos(list);
      const names = countRepoNames(list);
      const currentRepo = useChatStore.getState().selectedRepo;
      const matchedRepo = currentRepo
        ? list.find((repo) => repoMatchesSelection(repo, currentRepo))
        : undefined;
      if (matchedRepo && repoSelectionValue(matchedRepo) !== currentRepo) {
        setSelectedRepo(repoSelectionValue(matchedRepo), repoDisplayName(matchedRepo, names));
      } else if (list.length > 0 && !matchedRepo) {
        setSelectedRepo(repoSelectionValue(list[0]), repoDisplayName(list[0], names));
      }
      if (list.length === 0) {
        setSelectedRepo(null);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [setSelectedRepo]);

  useEffect(() => {
    // Initial fetch on mount — sync setState here is intentional (boot data load).
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void fetchRepos();
  }, [fetchRepos]);

  useEffect(() => {
    return () => {
      if (copiedErrorTimer.current !== null) {
        window.clearTimeout(copiedErrorTimer.current);
      }
    };
  }, []);

  const copyErrorDiagnostic = useCallback(async () => {
    if (!error) return;
    const ok = await copyTextToClipboard(
      [
        'GitNexus - erreur de liste des projets',
        '',
        error,
        '',
        'Commandes utiles:',
        '.\\gitnexus.cmd doctor',
        '.\\gitnexus.cmd chat -RestartBackend',
      ].join('\n')
    );
    if (!ok) return;
    setCopiedError(true);
    if (copiedErrorTimer.current !== null) {
      window.clearTimeout(copiedErrorTimer.current);
    }
    copiedErrorTimer.current = window.setTimeout(() => {
      setCopiedError(false);
      copiedErrorTimer.current = null;
    }, 1600);
  }, [error]);

  const selectedRepoInfo = repos.find((repo) => repoMatchesSelection(repo, selectedRepo));
  const duplicateNames = countRepoNames(repos);
  const normalizedQuery = query.trim().toLowerCase();
  const filteredRepos = useMemo(() => {
    if (!normalizedQuery) return repos;
    return repos.filter((repo) =>
      [repo.name, repo.id ?? '', repo.path ?? ''].join('\n').toLowerCase().includes(normalizedQuery)
    );
  }, [normalizedQuery, repos]);
  const label = selectedRepoInfo
    ? repoDisplayName(selectedRepoInfo, duplicateNames)
    : selectedRepo ?? (loading ? 'Chargement…' : 'Aucun projet');

  return (
    <div
      className="relative"
      onKeyDown={(e) => {
        if (e.key === 'Escape' && open) setOpen(false);
      }}
    >
      <button
        type="button"
        onClick={() => {
          const next = !open;
          setOpen(next);
          if (next && error) void fetchRepos();
        }}
        aria-label="Sélectionner le projet à interroger"
        aria-haspopup="listbox"
        aria-expanded={open}
        className={clsx(
          'flex items-center gap-2 rounded-lg border border-neutral-800 bg-neutral-900/60 px-3 py-1.5 text-sm transition hover:bg-neutral-800',
          error ? 'text-amber-300' : 'text-neutral-200'
        )}
      >
        {error ? <AlertCircle size={14} aria-hidden="true" /> : <FolderOpen size={14} className="text-neutral-500" aria-hidden="true" />}
        <span className="max-w-[240px] truncate">{label}</span>
        <ChevronDown size={14} className="opacity-60" aria-hidden="true" />
      </button>

      {open && (
        <div className="absolute left-0 top-full z-10 mt-1 max-h-80 w-72 overflow-y-auto rounded-lg border border-neutral-800 bg-neutral-900 shadow-xl">
          <div className="flex items-center justify-between border-b border-neutral-800 px-3 py-2">
            <div className="min-w-0">
              <div className="text-xs font-medium uppercase tracking-wider text-neutral-500">
                Projets indexés
              </div>
              {repos.length > 0 && (
                <div className="text-[11px] text-neutral-600">
                  {normalizedQuery ? `${filteredRepos.length}/${repos.length}` : repos.length}{' '}
                  projet(s)
                </div>
              )}
            </div>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                void fetchRepos();
              }}
              aria-label="Rafraîchir la liste des projets"
              className="text-neutral-500 transition hover:text-neutral-300"
              title="Rafraîchir"
            >
              <RefreshCw size={12} className={loading ? 'animate-spin' : ''} aria-hidden="true" />
            </button>
          </div>

          {!error && repos.length > 0 && (
            <label className="m-2 flex items-center gap-2 rounded-md border border-neutral-800 bg-neutral-950/60 px-2 py-1.5 text-xs text-neutral-500 focus-within:border-neutral-700">
              <Search size={13} aria-hidden="true" />
              <input
                type="search"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Rechercher..."
                aria-label="Rechercher un projet"
                className="min-w-0 flex-1 bg-transparent text-neutral-200 outline-none placeholder:text-neutral-600"
              />
            </label>
          )}

          {error && (
            <div className="px-3 py-3 text-xs text-amber-300">
              <div className="font-medium">Erreur</div>
              <div className="mt-1 break-words text-amber-300/80">{error}</div>
              <div className="mt-2 text-neutral-500">
                Lance <code className="rounded bg-neutral-800 px-1">.\gitnexus.cmd doctor</code> pour vérifier ports, backend et proxy Vite.
              </div>
              <button
                type="button"
                onClick={() => void copyErrorDiagnostic()}
                className="mt-3 inline-flex items-center gap-1.5 rounded-md border border-amber-900/60 bg-amber-950/20 px-2 py-1 text-[11px] font-medium text-amber-200 transition hover:bg-amber-950/35"
                aria-label={copiedError ? 'Diagnostic copié' : 'Copier le diagnostic'}
                title={copiedError ? 'Diagnostic copié' : 'Copier le diagnostic'}
              >
                {copiedError ? <Check size={12} aria-hidden="true" /> : <Copy size={12} aria-hidden="true" />}
                {copiedError ? 'Copié' : 'Copier diagnostic'}
              </button>
            </div>
          )}

          {!error && repos.length === 0 && !loading && (
            <div className="px-3 py-3 text-xs text-neutral-500">
              Aucun projet indexé. Lance{' '}
              <code className="rounded bg-neutral-800 px-1">.\gitnexus.cmd analyze -Repo &lt;path&gt;</code>{' '}
              depuis ce dépôt.
            </div>
          )}

          {!error && filteredRepos.length === 0 && repos.length > 0 && (
            <div className="px-3 py-3 text-xs text-neutral-500">
              Aucun projet ne correspond à cette recherche.
            </div>
          )}

          {filteredRepos.map((repo, index) => {
            const idSuffix = duplicateNames[repo.name] > 1 ? shortRepoId(repo) : null;
            return (
              <button
                key={`${repoSelectionValue(repo)}-${repo.indexedAt ?? 'unknown'}-${index}`}
                onClick={() => {
                  setSelectedRepo(repoSelectionValue(repo), repoDisplayName(repo, duplicateNames));
                  setOpen(false);
                }}
                className={clsx(
                  'flex w-full flex-col items-start gap-0.5 px-3 py-2 text-left text-sm transition',
                  repoMatchesSelection(repo, selectedRepo)
                    ? 'bg-purple-600/20 text-purple-200'
                    : 'text-neutral-300 hover:bg-neutral-800'
                )}
              >
                <div className="flex w-full items-center justify-between">
                  <span className="truncate font-medium">
                    {repo.name}
                    {idSuffix && (
                      <span className="ml-1 font-mono text-[10px] font-normal text-neutral-500">
                        {idSuffix}
                      </span>
                    )}
                  </span>
                  {(() => {
                    const d = parseIndexedAt(repo.indexedAt);
                    return d ? (
                      <span className="ml-2 shrink-0 text-[10px] text-neutral-500">
                        {d.toLocaleDateString()}
                      </span>
                    ) : null;
                  })()}
                </div>
                {repo.path && (
                  <span className="truncate text-[11px] text-neutral-500">{repo.path}</span>
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
