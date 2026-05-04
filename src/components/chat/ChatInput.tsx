import { useState, type KeyboardEvent } from 'react';
import { Send, Square } from 'lucide-react';
import { useChat } from '../../hooks/use-chat';
import { useChatStore } from '../../stores/chat-store';

export function ChatInput() {
  const [value, setValue] = useState('');
  const { sendMessage, cancel, isStreaming } = useChat();
  const selectedRepo = useChatStore((s) => s.selectedRepo);

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
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={onKeyDown}
          rows={1}
          placeholder={
            selectedRepo
              ? `Pose ta question sur ${selectedRepo}… (Shift+Entrée = newline)`
              : 'Sélectionne un projet en haut à droite avant de poser ta question…'
          }
          className="max-h-48 flex-1 resize-none bg-transparent px-2 py-2 text-sm text-neutral-100 outline-none placeholder:text-neutral-600"
          disabled={isStreaming}
        />
        {isStreaming ? (
          <button
            onClick={cancel}
            className="flex h-9 w-9 items-center justify-center rounded-lg bg-red-600 text-white transition hover:bg-red-500"
            title="Annuler"
          >
            <Square size={14} fill="currentColor" />
          </button>
        ) : (
          <button
            onClick={submit}
            disabled={!value.trim() || !selectedRepo}
            className="flex h-9 w-9 items-center justify-center rounded-lg bg-purple-600 text-white transition hover:bg-purple-500 disabled:cursor-not-allowed disabled:bg-neutral-800 disabled:text-neutral-600"
            title="Envoyer (Entrée)"
          >
            <Send size={16} />
          </button>
        )}
      </div>
      <p className="mx-auto mt-2 max-w-3xl text-center text-[11px] text-neutral-600">
        V1 · backend gitnexus-mcp via SSE · {selectedRepo ?? 'aucun projet'}
      </p>
    </div>
  );
}
