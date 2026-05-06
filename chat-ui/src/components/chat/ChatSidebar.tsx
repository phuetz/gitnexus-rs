import clsx from 'clsx';
import { Plus, MessageSquare, Trash2, Library } from 'lucide-react';
import { useChatStore } from '../../stores/chat-store';

export function ChatSidebar() {
  const sessions = useChatStore((s) => s.sessions);
  const currentSessionId = useChatStore((s) => s.currentSessionId);
  const createSession = useChatStore((s) => s.createSession);
  const selectSession = useChatStore((s) => s.selectSession);
  const deleteSession = useChatStore((s) => s.deleteSession);

  return (
    <aside className="flex h-full w-72 shrink-0 flex-col border-r border-neutral-900 bg-neutral-950">
      <div className="border-b border-neutral-900 p-3">
        <div className="mb-3 flex items-center gap-2 px-1">
          <div className="flex h-7 w-7 items-center justify-center rounded-md bg-neutral-900 text-neutral-400">
            <Library size={14} aria-hidden="true" />
          </div>
          <div className="min-w-0">
            <div className="text-sm font-medium text-neutral-200">Conversations</div>
            <div className="text-xs text-neutral-600">{sessions.length} session{sessions.length > 1 ? 's' : ''}</div>
          </div>
        </div>
        <button
          type="button"
          onClick={() => createSession()}
          className="flex w-full items-center justify-center gap-2 rounded-md border border-neutral-800 bg-neutral-900 px-3 py-2 text-sm text-neutral-100 transition hover:border-neutral-700 hover:bg-neutral-800"
        >
          <Plus size={14} aria-hidden="true" />
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
                'group flex cursor-pointer items-center gap-2 rounded-md border px-3 py-2 text-sm transition',
                session.id === currentSessionId
                  ? 'border-neutral-700 bg-neutral-800/80 text-neutral-100'
                  : 'border-transparent text-neutral-400 hover:border-neutral-900 hover:bg-neutral-900/80'
              )}
            >
              <MessageSquare size={14} className="shrink-0 opacity-60" aria-hidden="true" />
              <span className="flex-1 truncate">{session.title}</span>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  deleteSession(session.id);
                }}
                aria-label={`Supprimer la conversation "${session.title}"`}
                className="opacity-0 transition group-hover:opacity-100 hover:text-red-400"
              >
                <Trash2 size={14} aria-hidden="true" />
              </button>
            </div>
          ))
        )}
      </div>

      <div className="border-t border-neutral-900 p-3 text-[11px] text-neutral-600">
        <div className="font-medium text-neutral-500">GitNexus Chat</div>
        <div className="mt-0.5">Interface web MCP</div>
      </div>
    </aside>
  );
}
