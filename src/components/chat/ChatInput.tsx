import { useEffect, useRef, type KeyboardEvent } from 'react';
import { Send, Square } from 'lucide-react';
import { useChat } from '../../hooks/use-chat';
import { useChatStore } from '../../stores/chat-store';

const MIN_HEIGHT = 44;
const MAX_HEIGHT = 200;

export function ChatInput() {
  const { sendMessage, cancel, isStreaming } = useChat();
  const selectedRepo = useChatStore((s) => s.selectedRepo);
  const value = useChatStore((s) => s.inputDraft);
  const setValue = useChatStore((s) => s.setInputDraft);
  const taRef = useRef<HTMLTextAreaElement>(null);

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
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  };

  return (
    <div className="border-t border-neutral-900 bg-neutral-950/60 p-4">
      <div className="mx-auto flex max-w-3xl items-end gap-2 rounded-xl border border-neutral-800 bg-neutral-900/60 p-2 focus-within:border-neutral-700">
        <textarea
          ref={taRef}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={
            selectedRepo
              ? `Pose ta question sur ${selectedRepo}… (Shift+Entrée = newline)`
              : 'Sélectionne un projet en haut à droite avant de poser ta question…'
          }
          aria-label="Message à envoyer au chat"
          aria-busy={isStreaming}
          className="max-h-[200px] flex-1 resize-none bg-transparent px-2 py-2 text-sm text-neutral-100 outline-none placeholder:text-neutral-600"
          style={{ minHeight: MIN_HEIGHT }}
          disabled={isStreaming}
        />
        {isStreaming ? (
          <button
            type="button"
            onClick={cancel}
            aria-label="Annuler la requête en cours"
            className="flex h-9 w-9 items-center justify-center rounded-lg bg-red-600 text-white transition hover:bg-red-500"
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
            className="flex h-9 w-9 items-center justify-center rounded-lg bg-purple-600 text-white transition hover:bg-purple-500 disabled:cursor-not-allowed disabled:bg-neutral-800 disabled:text-neutral-600"
            title="Envoyer (Entrée)"
          >
            <Send size={16} aria-hidden="true" />
          </button>
        )}
      </div>
      <p className="mx-auto mt-2 max-w-3xl text-center text-[11px] text-neutral-600">
        V1 · backend gitnexus-mcp via SSE · {selectedRepo ?? 'aucun projet'}
      </p>
    </div>
  );
}
