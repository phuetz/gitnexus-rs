import { useState } from 'react';
import clsx from 'clsx';
import { User, Bot, Copy, RotateCcw, Check, Loader2, Wrench, X } from 'lucide-react';
import type { Message, ToolCall } from '../../types/chat';
import { Markdown } from '../ui/Markdown';

interface Props {
  message: Message;
  onRegenerate?: (messageId: string) => void;
  canRegenerate?: boolean;
}

export function ChatMessage({ message, onRegenerate, canRegenerate }: Props) {
  const isUser = message.role === 'user';
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(message.content);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard access can be denied (insecure context, focus). Silently
      // ignore — the user can still select & Ctrl+C as a fallback.
    }
  };

  return (
    <div
      className={clsx(
        'group flex gap-3 px-4 py-4',
        isUser ? 'bg-neutral-950/40' : 'bg-transparent'
      )}
    >
      <div
        className={clsx(
          'flex h-8 w-8 shrink-0 items-center justify-center rounded-md',
          isUser ? 'bg-purple-600/20 text-purple-300' : 'bg-emerald-600/20 text-emerald-300'
        )}
      >
        {isUser ? <User size={16} /> : <Bot size={16} />}
      </div>
      <div className="min-w-0 flex-1">
        <div className="mb-1 flex items-center gap-2">
          <span className="text-xs font-medium text-neutral-500">
            {isUser ? 'Vous' : 'GitNexus'}
          </span>
          {!isUser && message.content && (
            <div className="flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
              <button
                type="button"
                onClick={handleCopy}
                className="rounded p-1 text-neutral-500 hover:bg-neutral-900 hover:text-neutral-200"
                aria-label="Copier la réponse"
                title={copied ? 'Copié !' : 'Copier'}
              >
                {copied ? <Check size={12} /> : <Copy size={12} />}
              </button>
              {onRegenerate && canRegenerate && (
                <button
                  type="button"
                  onClick={() => onRegenerate(message.id)}
                  className="rounded p-1 text-neutral-500 hover:bg-neutral-900 hover:text-neutral-200"
                  aria-label="Régénérer la réponse"
                  title="Régénérer"
                >
                  <RotateCcw size={12} />
                </button>
              )}
            </div>
          )}
        </div>
        {!isUser && message.toolCalls && message.toolCalls.length > 0 && (
          <div className="mb-2 flex flex-wrap gap-1.5" aria-label="Outils invoqués par l'agent">
            {message.toolCalls.map((tc) => (
              <ToolCallBadge key={tc.id} toolCall={tc} />
            ))}
          </div>
        )}
        {isUser ? (
          <div className="whitespace-pre-wrap text-sm text-neutral-200">
            {message.content}
          </div>
        ) : (
          <Markdown>{message.content}</Markdown>
        )}
      </div>
    </div>
  );
}

function ToolCallBadge({ toolCall }: { toolCall: ToolCall }) {
  const { name, status } = toolCall;
  const tone =
    status === 'running'
      ? 'border-amber-800/60 bg-amber-950/30 text-amber-300'
      : status === 'done'
        ? 'border-emerald-800/60 bg-emerald-950/30 text-emerald-300'
        : status === 'error'
          ? 'border-red-800/60 bg-red-950/30 text-red-300'
          : 'border-neutral-800 bg-neutral-900 text-neutral-400';
  const Icon =
    status === 'running' ? Loader2 : status === 'error' ? X : status === 'done' ? Check : Wrench;
  return (
    <span
      className={clsx(
        'inline-flex items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-medium',
        tone
      )}
      title={`${name} — ${status}`}
    >
      <Icon size={11} className={status === 'running' ? 'animate-spin' : ''} aria-hidden />
      <code className="font-mono">{name}</code>
    </span>
  );
}
