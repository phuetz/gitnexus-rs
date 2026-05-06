import { useState } from 'react';
import { Activity, RefreshCw, ShieldCheck, TriangleAlert, X } from 'lucide-react';
import clsx from 'clsx';
import { mcpClient, type DiagnosticsInfo } from '../../api/mcp-client';
import { useChatStore } from '../../stores/chat-store';
import { formatExportTimestamp } from '../../utils/dates';

type DiagnosticsStatus = 'idle' | 'loading' | 'ready' | 'error';

export function SystemDiagnostics() {
  const selectedRepo = useChatStore((s) => s.selectedRepo);
  const selectedRepoName = useChatStore((s) => s.selectedRepoName);
  const [open, setOpen] = useState(false);
  const [status, setStatus] = useState<DiagnosticsStatus>('idle');
  const [diagnostics, setDiagnostics] = useState<DiagnosticsInfo | null>(null);
  const [error, setError] = useState('');

  const refresh = async () => {
    setStatus('loading');
    setError('');
    try {
      const next = await mcpClient.diagnostics();
      setDiagnostics(next);
      setStatus('ready');
    } catch (e) {
      setDiagnostics(null);
      setStatus('error');
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const toggle = () => {
    const nextOpen = !open;
    setOpen(nextOpen);
    if (nextOpen && status === 'idle') {
      void refresh();
    }
  };

  return (
    <div className="relative">
      <button
        type="button"
        onClick={toggle}
        className={clsx(
          'flex items-center gap-1.5 rounded-md border px-2 py-1 text-xs transition hover:bg-neutral-900',
          status === 'error'
            ? 'border-red-900/70 bg-red-950/20 text-red-300'
            : 'border-neutral-800 bg-neutral-900/60 text-neutral-300'
        )}
        aria-label="Ouvrir le diagnostic système"
        aria-expanded={open}
        title="Diagnostic système"
      >
        {status === 'loading' ? (
          <RefreshCw className="h-3.5 w-3.5 animate-spin" aria-hidden />
        ) : (
          <Activity className="h-3.5 w-3.5" aria-hidden />
        )}
        <span className="hidden xl:inline">Diagnostic</span>
      </button>

      {open && (
        <div
          role="dialog"
          aria-label="Diagnostic système GitNexus"
          className="absolute right-0 top-full z-50 mt-2 w-[min(92vw,30rem)] rounded-lg border border-neutral-800 bg-neutral-950 p-4 text-xs text-neutral-300 shadow-xl"
        >
          <div className="mb-3 flex items-start justify-between gap-3">
            <div>
              <div className="font-medium text-neutral-100">Diagnostic GitNexus</div>
              <div className="mt-1 text-neutral-500">{statusLabel(status, diagnostics)}</div>
            </div>
            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => void refresh()}
                className="rounded-md border border-neutral-800 p-1.5 text-neutral-400 hover:bg-neutral-900 hover:text-neutral-100"
                aria-label="Rafraîchir le diagnostic"
                title="Rafraîchir"
              >
                <RefreshCw className={clsx('h-3.5 w-3.5', status === 'loading' && 'animate-spin')} />
              </button>
              <button
                type="button"
                onClick={() => setOpen(false)}
                className="rounded-md border border-neutral-800 p-1.5 text-neutral-400 hover:bg-neutral-900 hover:text-neutral-100"
                aria-label="Fermer le diagnostic"
                title="Fermer"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
          </div>

          {status === 'error' ? (
            <div className="rounded-md border border-red-900/60 bg-red-950/20 p-3 text-red-200">
              <div className="mb-1 flex items-center gap-2 font-medium">
                <TriangleAlert className="h-4 w-4" aria-hidden />
                Diagnostic indisponible
              </div>
              <div className="leading-relaxed text-red-200/80">{error}</div>
            </div>
          ) : (
            <DiagnosticsBody
              diagnostics={diagnostics}
              selectedRepo={selectedRepo}
              selectedRepoName={selectedRepoName}
            />
          )}
        </div>
      )}
    </div>
  );
}

function DiagnosticsBody({
  diagnostics,
  selectedRepo,
  selectedRepoName,
}: {
  diagnostics: DiagnosticsInfo | null;
  selectedRepo: string | null;
  selectedRepoName: string | null;
}) {
  const frontend = window.location.host || 'navigateur local';
  const backend = mcpClient.baseUrl || 'proxy Vite / même origine';
  const llm = diagnostics?.llm;
  const oauth = diagnostics?.auth?.chatgptOAuth;

  return (
    <div className="space-y-3">
      <div className="grid gap-2 sm:grid-cols-2">
        <DiagnosticItem label="Frontend" value={frontend} />
        <DiagnosticItem label="Backend" value={backend} />
        <DiagnosticItem
          label="Service"
          value={diagnostics ? `${diagnostics.service} ${diagnostics.version}` : 'lecture...'}
        />
        <DiagnosticItem
          label="Projet actif"
          value={selectedRepoName ?? selectedRepo ?? 'aucun projet sélectionné'}
        />
      </div>

      <div className="grid gap-2 sm:grid-cols-3">
        <BooleanItem
          label="Auth HTTP"
          value={diagnostics ? diagnostics.httpAuthRequired : null}
          trueText="requise"
          falseText="non requise"
        />
        <BooleanItem
          label="Chemins repo"
          value={diagnostics ? diagnostics.repoPathsExposed : null}
          trueText="exposés"
          falseText="masqués"
        />
        <DiagnosticItem
          label="Repos indexés"
          value={diagnostics ? String(diagnostics.repos.count) : '...'}
        />
      </div>

      <div className="rounded-md border border-neutral-800 bg-neutral-900/40 p-3">
        <div className="mb-2 flex items-center gap-2 font-medium text-neutral-100">
          <ShieldCheck className="h-4 w-4 text-emerald-300" aria-hidden />
          LLM
        </div>
        <div className="grid gap-2 sm:grid-cols-2">
          <DiagnosticItem
            label="Provider"
            value={llm?.configured ? (llm.provider ?? 'inconnu') : 'non configuré'}
          />
          <DiagnosticItem label="Modèle" value={llm?.model ?? 'non défini'} />
          <DiagnosticItem label="Raisonnement" value={llm?.reasoningEffort ?? 'défaut'} />
          <DiagnosticItem
            label="Max tokens"
            value={llm?.maxTokens ? String(llm.maxTokens) : 'défaut'}
          />
          <DiagnosticItem label="OAuth ChatGPT" value={oauthStatusLabel(oauth?.status)} />
          <DiagnosticItem
            label="Dernier refresh"
            value={formatOAuthRefresh(oauth?.lastRefresh)}
          />
        </div>
      </div>
    </div>
  );
}

function DiagnosticItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-md border border-neutral-800 bg-neutral-900/40 px-3 py-2">
      <div className="text-[11px] uppercase text-neutral-500">{label}</div>
      <div className="mt-1 truncate font-mono text-neutral-200" title={value}>
        {value}
      </div>
    </div>
  );
}

function BooleanItem({
  label,
  value,
  trueText,
  falseText,
}: {
  label: string;
  value: boolean | null;
  trueText: string;
  falseText: string;
}) {
  const text = value === null ? '...' : value ? trueText : falseText;
  const tone =
    value === null
      ? 'text-neutral-400'
      : value
        ? 'text-amber-300'
        : 'text-emerald-300';
  return (
    <div className="rounded-md border border-neutral-800 bg-neutral-900/40 px-3 py-2">
      <div className="text-[11px] uppercase text-neutral-500">{label}</div>
      <div className={clsx('mt-1 font-medium', tone)}>{text}</div>
    </div>
  );
}

function statusLabel(status: DiagnosticsStatus, diagnostics: DiagnosticsInfo | null): string {
  if (status === 'loading') return 'Lecture en cours...';
  if (status === 'error') return 'Erreur de diagnostic';
  if (!diagnostics) return 'Pas encore rafraîchi';
  return `Généré ${formatExportTimestamp(diagnostics.generatedAtUnixMs)}`;
}

function oauthStatusLabel(status: string | undefined): string {
  switch (status) {
    case 'logged_in':
      return 'connecté';
    case 'missing':
      return 'non détecté';
    case 'incomplete':
      return 'incomplet';
    case 'invalid':
      return 'fichier invalide';
    case 'unreadable':
      return 'illisible';
    default:
      return status ?? 'inconnu';
  }
}

function formatOAuthRefresh(raw: string | null | undefined): string {
  if (!raw) return 'non défini';
  const timestamp = new Date(raw).getTime();
  if (Number.isNaN(timestamp)) return raw;
  return formatExportTimestamp(timestamp);
}
