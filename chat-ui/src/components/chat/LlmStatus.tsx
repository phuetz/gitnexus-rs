import { useMemo, useState } from 'react';
import { Check, Copy, Cpu, RefreshCw, SlidersHorizontal } from 'lucide-react';
import clsx from 'clsx';
import type { LlmConfigState } from '../../hooks/use-llm-config';
import { copyTextToClipboard } from '../../utils/clipboard';

interface Props {
  llm: LlmConfigState;
}

export function LlmStatus({ llm }: Props) {
  const { status, config, message, refresh } = llm;
  const [expanded, setExpanded] = useState(false);
  const [copied, setCopied] = useState(false);
  const label =
    status === 'ready'
      ? `${config?.provider ?? 'LLM'} · ${config?.model ?? 'modèle ?'}`
      : status === 'missing'
        ? 'LLM non configuré'
        : status === 'checking'
          ? 'LLM...'
          : 'LLM indisponible';

  const detail = [
    message,
    config?.reasoningEffort ? `Raisonnement: ${config.reasoningEffort}` : null,
    config?.maxTokens ? `Max tokens: ${config.maxTokens}` : null,
  ]
    .filter(Boolean)
    .join('\n');

  const reasoningCommand = useMemo(() => buildReasoningCommand(config), [config]);

  const handleCopyCommand = async () => {
    const ok = await copyTextToClipboard(reasoningCommand);
    if (!ok) return;
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setExpanded((value) => !value)}
        className={clsx(
          'flex max-w-[280px] items-center gap-1.5 rounded-md border px-2 py-1 text-xs transition hover:bg-neutral-900',
          status === 'ready'
            ? 'border-neutral-800 bg-neutral-900/60 text-neutral-300'
            : status === 'checking'
              ? 'border-amber-900/70 bg-amber-950/20 text-amber-300'
              : 'border-red-900/70 bg-red-950/20 text-red-300'
        )}
        aria-label={`Configuration LLM : ${detail || label}. Cliquer pour les détails.`}
        aria-expanded={expanded}
        title={detail || label}
      >
        {status === 'checking' ? (
          <RefreshCw className="h-3.5 w-3.5 animate-spin" aria-hidden />
        ) : (
          <Cpu className="h-3.5 w-3.5" aria-hidden />
        )}
        <span className="hidden truncate lg:inline">{label}</span>
        {config?.reasoningEffort && (
          <span className="hidden rounded bg-neutral-800 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-neutral-400 xl:inline">
            {config.reasoningEffort}
          </span>
        )}
      </button>

      {expanded && (
        <div
          role="dialog"
          aria-label="Détails de la configuration LLM"
          className="absolute right-0 top-full z-50 mt-2 w-96 rounded-lg border border-neutral-800 bg-neutral-950 p-3 text-xs text-neutral-300 shadow-xl"
        >
          <div className="mb-3 flex items-center gap-2 font-medium text-neutral-100">
            <SlidersHorizontal className="h-3.5 w-3.5 text-purple-300" aria-hidden />
            Configuration LLM
          </div>

          <dl className="grid grid-cols-[110px_1fr] gap-x-3 gap-y-1.5">
            <dt className="text-neutral-500">État</dt>
            <dd>{statusLabel(status)}</dd>
            <dt className="text-neutral-500">Fournisseur</dt>
            <dd>{config?.provider ?? 'non configuré'}</dd>
            <dt className="text-neutral-500">Modèle</dt>
            <dd>{config?.model ?? 'modèle ?'}</dd>
            <dt className="text-neutral-500">Réflexion</dt>
            <dd className="uppercase">{config?.reasoningEffort ?? 'non renseigné'}</dd>
            <dt className="text-neutral-500">Max tokens</dt>
            <dd>{config?.maxTokens ?? 'non renseigné'}</dd>
          </dl>

          <div className="mt-3 rounded-md border border-neutral-800 bg-neutral-900/70 p-2">
            <div className="mb-1 text-[11px] uppercase tracking-wide text-neutral-500">
              Commande réflexion xhigh
            </div>
            <div className="flex items-center gap-2">
              <code className="min-w-0 flex-1 truncate font-mono text-[11px] text-neutral-200">
                {reasoningCommand}
              </code>
              <button
                type="button"
                onClick={() => void handleCopyCommand()}
                className="rounded border border-neutral-800 p-1 text-neutral-400 hover:bg-neutral-800 hover:text-neutral-100"
                aria-label="Copier la commande de configuration LLM"
                title={copied ? 'Copié' : 'Copier'}
              >
                {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
              </button>
            </div>
          </div>

          <div className="mt-3 flex justify-end gap-2">
            <button
              type="button"
              onClick={() => void refresh()}
              className="inline-flex items-center gap-1 rounded border border-neutral-800 px-2 py-1 text-xs hover:bg-neutral-900"
            >
              <RefreshCw className="h-3 w-3" aria-hidden />
              Rafraîchir
            </button>
            <button
              type="button"
              onClick={() => setExpanded(false)}
              className="rounded border border-neutral-800 px-2 py-1 text-xs hover:bg-neutral-900"
            >
              Fermer
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

function statusLabel(status: LlmConfigState['status']): string {
  if (status === 'ready') return 'Prêt';
  if (status === 'checking') return 'Vérification';
  if (status === 'missing') return 'Non configuré';
  return 'Indisponible';
}

function buildReasoningCommand(config: LlmConfigState['config']): string {
  const model = config?.model?.trim() || 'gpt-5.5';
  const maxTokens = config?.maxTokens || 8192;
  return `.\\config-chatgpt.cmd -Model ${model} -Reasoning xhigh -MaxTokens ${maxTokens}`;
}
