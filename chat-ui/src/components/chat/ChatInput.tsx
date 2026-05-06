import { useEffect, useRef, type KeyboardEvent } from 'react';
import { Database, Send, Square } from 'lucide-react';
import { useChat } from '../../hooks/use-chat';
import { useChatStore } from '../../stores/chat-store';

const MIN_HEIGHT = 44;
const MAX_HEIGHT = 200;

export function ChatInput() {
  const { sendMessage, cancel, isStreaming } = useChat();
  const selectedRepo = useChatStore((s) => s.selectedRepo);
  const selectedRepoName = useChatStore((s) => s.selectedRepoName);
  const value = useChatStore((s) => s.inputDraft);
  const setValue = useChatStore((s) => s.setInputDraft);
  const taRef = useRef<HTMLTextAreaElement>(null);
  const repoLabel = selectedRepoName ?? selectedRepo;

  useEffect(() => {
    const ta = taRef.current;
    if (!ta) return;
    ta.style.height = 'auto';
    ta.style.height = `${Math.min(MAX_HEIGHT, Math.max(MIN_HEIGHT, ta.scrollHeight))}px`;
  }, [value]);

  const submit = () => {
    if (!value.trim() || isStreaming) return;
    void sendMessage(value);
    setValue('');
    // Re-focus textarea après envoi (productivité clavier).
    requestAnimationFrame(() => taRef.current?.focus());
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  };

  return (
    <div className="border-t border-neutral-900 bg-neutral-950 p-4">
      <div className="mx-auto max-w-4xl">
        <div className="mb-2 flex items-center justify-between gap-3 text-xs text-neutral-500">
          <span className="inline-flex min-w-0 items-center gap-1.5 truncate">
            <Database size={12} aria-hidden="true" />
            <span className="truncate">{repoLabel ?? 'Aucun projet sélectionné'}</span>
          </span>
          <span className={selectedRepo ? 'text-emerald-400' : 'text-neutral-600'}>
            {selectedRepo ? 'Prêt' : 'Projet requis'}
          </span>
        </div>
        <div className="flex items-end gap-2 rounded-lg border border-neutral-800 bg-neutral-900/70 p-2 shadow-[0_1px_0_rgba(255,255,255,0.03)] focus-within:border-neutral-700">
        <textarea
          ref={taRef}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={
            selectedRepo
              ? `Pose ta question sur ${repoLabel}…`
              : 'Sélectionne un projet en haut à droite avant de poser ta question…'
          }
          aria-label="Message à envoyer au chat"
          aria-busy={isStreaming}
          className="max-h-[200px] flex-1 resize-none bg-transparent px-2 py-2 text-sm leading-6 text-neutral-100 outline-none placeholder:text-neutral-600 disabled:cursor-not-allowed disabled:text-neutral-600"
          style={{ minHeight: MIN_HEIGHT }}
          disabled={isStreaming || !selectedRepo}
        />
        {isStreaming ? (
          <button
            type="button"
            onClick={cancel}
            aria-label="Annuler la requête en cours"
            className="flex h-10 w-10 items-center justify-center rounded-md border border-red-500/30 bg-red-600/90 text-white transition hover:bg-red-500"
            title="Annuler"
          >
            <Square size={14} fill="currentColor" aria-hidden="true" />
          </button>
        ) : (
          <button
            type="button"
            onClick={submit}
            disabled={!value.trim() || !selectedRepo}
            aria-label="Envoyer le message"
            className="flex h-10 w-10 items-center justify-center rounded-md border border-transparent bg-purple-600 text-white transition hover:bg-purple-500 disabled:cursor-not-allowed disabled:border-neutral-800 disabled:bg-neutral-900 disabled:text-neutral-600"
            title="Envoyer (Entrée)"
          >
            <Send size={16} aria-hidden="true" />
          </button>
        )}
      </div>
      </div>
    </div>
  );
}
