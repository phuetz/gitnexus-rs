import { useEffect, useRef } from 'react';
import { BarChart3, Network, Plug, Skull } from 'lucide-react';
import { useChatStore } from '../../stores/chat-store';
import { ChatMessage } from './ChatMessage';

const SUGGESTIONS = [
  { icon: BarChart3, label: 'Hotspots', prompt: 'Quels sont les hotspots du repo (fichiers les plus touchés et risqués) ?' },
  { icon: Network, label: 'Architecture', prompt: 'Donne-moi une vue d\'ensemble de l\'architecture du projet en 5 points clés.' },
  { icon: Plug, label: 'Endpoints', prompt: 'Liste les endpoints HTTP exposés par ce projet et leurs handlers.' },
  { icon: Skull, label: 'Code mort', prompt: 'Identifie le code mort ou les candidats à supprimer.' },
];

export function ChatMessages() {
  const session = useChatStore((s) => s.getCurrentSession());
  const isStreaming = useChatStore((s) => s.isStreaming);
  const selectedRepo = useChatStore((s) => s.selectedRepo);
  const setInputDraft = useChatStore((s) => s.setInputDraft);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: 'smooth' });
  }, [session?.messages.length, isStreaming]);

  if (!session || session.messages.length === 0) {
    return (
      <div className="flex h-full items-center justify-center px-6">
        <div className="w-full max-w-2xl text-center">
          <h2 className="mb-2 text-2xl font-light text-neutral-400">GitNexus Chat</h2>
          <p className="mb-8 text-sm text-neutral-600">
            {selectedRepo
              ? `Pose une question sur ${selectedRepo} ou choisis une suggestion :`
              : 'Sélectionne un projet en haut à droite, puis pose ta question.'}
          </p>
          {selectedRepo && (
            <div className="grid gap-3 sm:grid-cols-2">
              {SUGGESTIONS.map(({ icon: Icon, label, prompt }) => (
                <button
                  key={label}
                  type="button"
                  onClick={() => setInputDraft(prompt)}
                  aria-label={`Suggestion : ${prompt}`}
                  className="group flex items-start gap-3 rounded-xl border border-neutral-800 bg-neutral-900/40 p-4 text-left transition hover:border-neutral-700 hover:bg-neutral-900"
                >
                  <Icon size={18} className="mt-0.5 shrink-0 text-purple-400" aria-hidden="true" />
                  <div className="min-w-0">
                    <div className="text-sm font-medium text-neutral-200">{label}</div>
                    <div className="mt-1 text-xs text-neutral-500 line-clamp-2 group-hover:text-neutral-400">
                      {prompt}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    );
  }

  return (
    <div ref={scrollRef} className="h-full overflow-y-auto" role="log" aria-live="polite" aria-relevant="additions text">
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
