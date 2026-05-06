import { Cpu, RefreshCw } from 'lucide-react';
import clsx from 'clsx';
import type { LlmConfigState } from '../../hooks/use-llm-config';

interface Props {
  llm: LlmConfigState;
}

export function LlmStatus({ llm }: Props) {
  const { status, config, message, refresh } = llm;
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

  return (
    <button
      type="button"
      onClick={() => void refresh()}
      className={clsx(
        'flex max-w-[280px] items-center gap-1.5 rounded-md border px-2 py-1 text-xs transition hover:bg-neutral-900',
        status === 'ready'
          ? 'border-neutral-800 bg-neutral-900/60 text-neutral-300'
          : status === 'checking'
            ? 'border-amber-900/70 bg-amber-950/20 text-amber-300'
            : 'border-red-900/70 bg-red-950/20 text-red-300'
      )}
      aria-label={`Configuration LLM : ${detail || label}. Cliquer pour rafraîchir.`}
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
  );
}
