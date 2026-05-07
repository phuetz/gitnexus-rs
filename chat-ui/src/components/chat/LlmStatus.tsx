import { useMemo, useState } from 'react';
import { Check, Copy, Cpu, RefreshCw, SlidersHorizontal } from 'lucide-react';
import clsx from 'clsx';
import type { LlmConfigState } from '../../hooks/use-llm-config';
import { copyTextToClipboard } from '../../utils/clipboard';

interface Props {
  llm: LlmConfigState;
}

const REASONING_PRESETS = [
  { value: 'low', label: 'Low', hint: 'rapide' },
  { value: 'medium', label: 'Medium', hint: 'équilibré' },
  { value: 'high', label: 'High', hint: 'approfondi' },
  { value: 'xhigh', label: 'XHigh', hint: 'maximum' },
] as const;

export function LlmStatus({ llm }: Props) {
  const { status, config, message, refresh } = llm;
  const [expanded, setExpanded] = useState(false);
  const [copiedPreset, setCopiedPreset] = useState<string | null>(null);
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

  const activeReasoning = config?.reasoningEffort?.toLowerCase();
  const previewCommand = useMemo(() => buildReasoningCommand(config, 'xhigh'), [config]);

  const handleCopyCommand = async (reasoning: string) => {
    const ok = await copyTextToClipboard(buildReasoningCommand(config, reasoning));
    if (!ok) return;
    setCopiedPreset(reasoning);
    window.setTimeout(() => setCopiedPreset(null), 1500);
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
            <div className="mb-2 flex items-center justify-between gap-2">
              <div className="text-[11px] uppercase tracking-wide text-neutral-500">
                Niveau de réflexion
              </div>
              <code className="truncate font-mono text-[10px] text-neutral-500">
                {previewCommand}
              </code>
            </div>
            <div className="grid grid-cols-2 gap-1.5">
              {REASONING_PRESETS.map((preset) => {
                const active = activeReasoning === preset.value;
                const copied = copiedPreset === preset.value;
                return (
                  <button
                    key={preset.value}
                    type="button"
                    onClick={() => void handleCopyCommand(preset.value)}
                    className={clsx(
                      'flex items-center justify-between gap-2 rounded-md border px-2 py-1.5 text-left transition',
                      active
                        ? 'border-purple-700 bg-purple-950/40 text-purple-100'
                        : 'border-neutral-800 bg-neutral-950/60 text-neutral-300 hover:bg-neutral-900'
                    )}
                    aria-label={`Copier la commande de configuration LLM en ${preset.value}`}
                    title={`Copier la commande ${preset.value}`}
                  >
                    <span className="min-w-0">
                      <span className="block text-xs font-medium">{preset.label}</span>
                      <span className="block text-[10px] text-neutral-500">{preset.hint}</span>
                    </span>
                    {copied ? (
                      <Check className="h-3.5 w-3.5 shrink-0 text-emerald-300" aria-hidden />
                    ) : (
                      <Copy className="h-3.5 w-3.5 shrink-0 text-neutral-500" aria-hidden />
                    )}
                  </button>
                );
              })}
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

function buildReasoningCommand(config: LlmConfigState['config'], reasoning: string): string {
  const model = config?.model?.trim() || 'gpt-5.5';
  const maxTokens = config?.maxTokens || 8192;
  return `.\\config-chatgpt.cmd -Model ${model} -Reasoning ${reasoning} -MaxTokens ${maxTokens}`;
}
