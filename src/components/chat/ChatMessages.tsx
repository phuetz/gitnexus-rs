import { useEffect, useRef } from 'react';
import { useChatStore } from '../../stores/chat-store';
import { ChatMessage } from './ChatMessage';

export function ChatMessages() {
  const session = useChatStore((s) => s.getCurrentSession());
  const isStreaming = useChatStore((s) => s.isStreaming);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: 'smooth' });
  }, [session?.messages.length, isStreaming]);

  if (!session || session.messages.length === 0) {
    return (
      <div className="flex h-full items-center justify-center text-neutral-600">
        <div className="text-center">
          <h2 className="mb-2 text-2xl font-light text-neutral-400">GitNexus Chat</h2>
          <p className="text-sm">Pose une question sur ton code pour démarrer.</p>
        </div>
      </div>
    );
  }

  return (
    <div ref={scrollRef} className="h-full overflow-y-auto">
      <div className="mx-auto max-w-3xl divide-y divide-neutral-900">
        {session.messages.map((m) => (
          <ChatMessage key={m.id} message={m} />
        ))}
        {isStreaming && (
          <div className="flex gap-3 px-4 py-4">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-emerald-600/20 text-emerald-300">
              <div className="h-2 w-2 animate-pulse rounded-full bg-current" />
            </div>
            <div className="text-sm text-neutral-500">GitNexus réfléchit…</div>
          </div>
        )}
      </div>
    </div>
  );
}
