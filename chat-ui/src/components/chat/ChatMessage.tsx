import { useState } from 'react';
import clsx from 'clsx';
import { User, Bot, Copy, RotateCcw, Check, Loader2, Wrench, X } from 'lucide-react';
import type { Message, ToolCall } from '../../types/chat';
import type { LlmConfigInfo } from '../../api/mcp-client';
import { Markdown } from '../ui/Markdown';
import { formatMessageTimestamp } from '../../utils/dates';
import { copyTextToClipboard } from '../../utils/clipboard';

interface Props {
  message: Message;
  llm?: LlmConfigInfo | null;
  onRegenerate?: (messageId: string) => void;
  canRegenerate?: boolean;
}

export function ChatMessage({ message, llm, onRegenerate, canRegenerate }: Props) {
  const isUser = message.role === 'user';
  const [copied, setCopied] = useState(false);
  const timestamp = formatMessageTimestamp(message.createdAt);
  const llmLabel = isUser ? null : formatLlmBadge(llm);

  const handleCopy = async () => {
    const ok = await copyTextToClipboard(message.content);
    if (ok) {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }
  };

  return (
    <div
      className={clsx(
        'group flex',
        isUser ? 'justify-end' : 'justify-start'
      )}
    >
      <div className={clsx('flex max-w-[88%] gap-3', isUser && 'flex-row-reverse')}>
        <div
          className={clsx(
            'flex h-8 w-8 shrink-0 items-center justify-center rounded-md border',
            isUser
              ? 'border-purple-800/50 bg-purple-600/15 text-purple-300'
              : 'border-emerald-800/50 bg-emerald-600/15 text-emerald-300'
          )}
        >
          {isUser ? <User size={16} /> : <Bot size={16} />}
        </div>
        <div
          className={clsx(
            'min-w-0 flex-1 rounded-lg border px-4 py-3',
            isUser
              ? 'border-purple-900/50 bg-purple-950/30'
              : 'border-neutral-900 bg-neutral-900/35'
          )}
        >
          <div className="mb-2 flex items-center gap-2">
            <span className="text-xs font-medium text-neutral-500">
              {isUser ? 'Vous' : 'GitNexus'}
            </span>
            {timestamp && (
              <time
                dateTime={new Date(message.createdAt).toISOString()}
                className="text-[11px] text-neutral-600"
                title={timestamp}
              >
                {timestamp}
              </time>
            )}
            {llmLabel && (
              <span
                className="max-w-[16rem] truncate rounded bg-neutral-900 px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-neutral-500"
                title={`LLM actif : ${llmLabel}`}
              >
                {llmLabel}
              </span>
            )}
            {message.content && (
              <div className="ml-auto flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
                <button
                  type="button"
                  onClick={handleCopy}
                  className="rounded p-1 text-neutral-500 hover:bg-neutral-800 hover:text-neutral-200"
                  aria-label={isUser ? 'Copier le message' : 'Copier la réponse'}
                  title={copied ? 'Copié !' : 'Copier'}
                >
                  {copied ? <Check size={12} /> : <Copy size={12} />}
                </button>
                {!isUser && onRegenerate && canRegenerate && (
                  <button
                    type="button"
                    onClick={() => onRegenerate(message.id)}
                    className="rounded p-1 text-neutral-500 hover:bg-neutral-800 hover:text-neutral-200"
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
            <div className="mb-3 flex flex-wrap gap-1.5" aria-label="Outils invoqués par l'agent">
              {message.toolCalls.map((tc) => (
                <ToolCallBadge key={tc.id} toolCall={tc} />
              ))}
            </div>
          )}
          {isUser ? (
            <div className="whitespace-pre-wrap text-sm leading-6 text-neutral-100">
              {message.content}
            </div>
          ) : (
            <Markdown>{message.content}</Markdown>
          )}
        </div>
      </div>
    </div>
  );
}

function formatLlmBadge(llm: LlmConfigInfo | null | undefined): string | null {
  if (!llm?.configured) return null;
  const parts = [llm.provider, llm.model, llm.reasoningEffort].filter(
    (part): part is string => typeof part === 'string' && part.trim().length > 0
  );
  return parts.length > 0 ? parts.join(' · ') : null;
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
