import clsx from 'clsx';
import { Plus, MessageSquare, Trash2 } from 'lucide-react';
import { useChatStore } from '../../stores/chat-store';

export function ChatSidebar() {
  const sessions = useChatStore((s) => s.sessions);
  const currentSessionId = useChatStore((s) => s.currentSessionId);
  const createSession = useChatStore((s) => s.createSession);
  const selectSession = useChatStore((s) => s.selectSession);
  const deleteSession = useChatStore((s) => s.deleteSession);

  return (
    <aside className="flex h-full w-64 shrink-0 flex-col border-r border-neutral-900 bg-neutral-950/40">
      <div className="border-b border-neutral-900 p-3">
        <button
          onClick={() => createSession()}
          className="flex w-full items-center justify-center gap-2 rounded-lg border border-neutral-800 bg-neutral-900/60 px-3 py-2 text-sm text-neutral-200 transition hover:bg-neutral-800"
        >
          <Plus size={14} />
          Nouvelle conversation
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        {sessions.length === 0 ? (
          <p className="px-3 py-4 text-xs text-neutral-600">
            Aucune conversation. Crée-en une pour démarrer.
          </p>
        ) : (
          sessions.map((session) => (
            <div
              key={session.id}
              onClick={() => selectSession(session.id)}
              className={clsx(
                'group flex cursor-pointer items-center gap-2 rounded-lg px-3 py-2 text-sm transition',
                session.id === currentSessionId
                  ? 'bg-neutral-800/80 text-neutral-100'
                  : 'text-neutral-400 hover:bg-neutral-900/80'
              )}
            >
              <MessageSquare size={14} className="shrink-0 opacity-60" />
              <span className="flex-1 truncate">{session.title}</span>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  deleteSession(session.id);
                }}
                className="opacity-0 transition group-hover:opacity-100 hover:text-red-400"
              >
                <Trash2 size={14} />
              </button>
            </div>
          ))
        )}
      </div>

      <div className="border-t border-neutral-900 p-3 text-[11px] text-neutral-600">
        <div className="font-medium text-neutral-500">GitNexus Chat</div>
        <div>v0.0.1 · MIT</div>
      </div>
    </aside>
  );
}
