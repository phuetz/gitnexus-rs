import { useState } from 'react';
import { useBackendStatus, type BackendStatus as Status } from '../../hooks/use-backend-status';

/**
 * Header badge that shows whether the MCP backend is reachable. Click
 * the badge to expand a tooltip with the full message + version + a
 * one-shot retry button.
 *
 * Three visual states:
 *   - online   : green dot, service@version label
 *   - checking : amber dot pulsing, "Vérification…"
 *   - offline  : red dot, "Hors ligne" + click for details
 *
 * Designed to live in `ChatPanel`'s header next to `ProjectSelector`.
 *
 * No accessibility shortcuts beyond the tooltip — this is read-only
 * status, not a primary action. The `aria-label` describes the current
 * state for screen readers.
 */
export function BackendStatus() {
  const health = useBackendStatus();
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        className="flex items-center gap-1.5 rounded-md border border-neutral-800 bg-neutral-900/60 px-2 py-1 text-xs text-neutral-300 hover:bg-neutral-900"
        aria-label={ariaLabel(health.status, health.message)}
        aria-expanded={expanded}
      >
        <StatusDot status={health.status} />
        <span className="hidden sm:inline">{shortLabel(health)}</span>
      </button>
      {expanded && (
        <div
          role="dialog"
          aria-label="Détails de la connexion au serveur MCP"
          className="absolute right-0 top-full z-50 mt-2 w-80 rounded-lg border border-neutral-800 bg-neutral-950 p-3 text-xs text-neutral-300 shadow-xl"
        >
          <div className="mb-1 flex items-center gap-2 font-medium text-neutral-100">
            <StatusDot status={health.status} />
            {longLabel(health.status)}
          </div>
          <p className="mb-2 leading-relaxed text-neutral-400">{health.message}</p>
          {health.lastSuccessAt > 0 && health.status !== 'online' && (
            <p className="text-neutral-500">
              Dernière connexion réussie : {new Date(health.lastSuccessAt).toLocaleTimeString()}
            </p>
          )}
          <div className="mt-2 flex justify-end">
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

function StatusDot({ status }: { status: Status }) {
  const className =
    status === 'online'
      ? 'h-2 w-2 rounded-full bg-emerald-500'
      : status === 'checking'
        ? 'h-2 w-2 rounded-full bg-amber-400 animate-pulse'
        : 'h-2 w-2 rounded-full bg-red-500';
  return <span className={className} aria-hidden="true" />;
}

function shortLabel(health: ReturnType<typeof useBackendStatus>): string {
  if (health.status === 'online') {
    return health.service ? `${health.service}` : 'Connecté';
  }
  if (health.status === 'checking') return 'Vérification…';
  return 'Hors ligne';
}

function longLabel(status: Status): string {
  if (status === 'online') return 'Serveur connecté';
  if (status === 'checking') return 'Vérification en cours';
  return 'Serveur injoignable';
}

function ariaLabel(status: Status, message: string): string {
  if (status === 'online') return `Serveur connecté. ${message}. Cliquer pour les détails.`;
  if (status === 'checking') return 'Vérification de la connexion au serveur en cours.';
  return `Serveur injoignable. ${message}. Cliquer pour les détails.`;
}
