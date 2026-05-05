import clsx from 'clsx';
import { User, Bot } from 'lucide-react';
import type { Message } from '../../types/chat';
import { Markdown } from '../ui/Markdown';

interface Props {
  message: Message;
}

export function ChatMessage({ message }: Props) {
  const isUser = message.role === 'user';

  return (
    <div
      className={clsx(
        'flex gap-3 px-4 py-4',
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
        <div className="mb-1 text-xs font-medium text-neutral-500">
          {isUser ? 'Vous' : 'GitNexus'}
        </div>
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
