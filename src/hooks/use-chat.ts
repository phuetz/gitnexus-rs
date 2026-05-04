import { useCallback, useRef } from 'react';
import { useChatStore } from '../stores/chat-store';
import { mcpClient, type ChatHistoryMessage } from '../api/mcp-client';
import type { Message } from '../types/chat';

const newId = () => crypto.randomUUID();

export function useChat() {
  const currentSessionId = useChatStore((s) => s.currentSessionId);
  const selectedRepo = useChatStore((s) => s.selectedRepo);
  const createSession = useChatStore((s) => s.createSession);
  const appendMessage = useChatStore((s) => s.appendMessage);
  const updateMessage = useChatStore((s) => s.updateMessage);
  const setStreaming = useChatStore((s) => s.setStreaming);
  const isStreaming = useChatStore((s) => s.isStreaming);
  const getCurrentSession = useChatStore((s) => s.getCurrentSession);

  const abortRef = useRef<AbortController | null>(null);

  const cancel = useCallback(() => {
    abortRef.current?.abort();
    abortRef.current = null;
    setStreaming(false);
  }, [setStreaming]);

  const sendMessage = useCallback(
    async (content: string) => {
      const trimmed = content.trim();
      if (!trimmed || isStreaming) return;

      if (!selectedRepo) {
        const errMsg: Message = {
          id: newId(),
          role: 'assistant',
          content:
            '_Aucun projet sélectionné._ Choisis un repo dans la barre du haut avant de poser ta question.',
          createdAt: Date.now(),
        };
        let sid = currentSessionId;
        if (!sid) sid = createSession('Nouvelle conversation');
        appendMessage(sid, errMsg);
        return;
      }

      let sessionId = currentSessionId;
      if (!sessionId) sessionId = createSession(trimmed.slice(0, 60));

      const previous = getCurrentSession();
      const history: ChatHistoryMessage[] = (previous?.messages ?? [])
        .filter((m) => m.role === 'user' || m.role === 'assistant')
        .map((m) => ({ role: m.role as 'user' | 'assistant', content: m.content }));

      const userMessage: Message = {
        id: newId(),
        role: 'user',
        content: trimmed,
        createdAt: Date.now(),
      };
      appendMessage(sessionId, userMessage);

      const assistantId = newId();
      const assistantMessage: Message = {
        id: assistantId,
        role: 'assistant',
        content: '',
        createdAt: Date.now(),
      };
      appendMessage(sessionId, assistantMessage);

      const ctrl = new AbortController();
      abortRef.current = ctrl;
      setStreaming(true);

      let acc = '';
      try {
        await mcpClient.chatStream(
          selectedRepo,
          trimmed,
          history,
          (delta) => {
            acc += delta;
            updateMessage(sessionId!, assistantId, acc);
          },
          ctrl.signal
        );
        if (!acc) {
          updateMessage(sessionId, assistantId, '_Réponse vide reçue du serveur._');
        }
      } catch (err) {
        const aborted = err instanceof DOMException && err.name === 'AbortError';
        const reason = aborted
          ? '_Requête annulée._'
          : `_Erreur :_ ${err instanceof Error ? err.message : String(err)}\n\nVérifie que le serveur tourne : \`gitnexus serve --port 3000\` (ou ajuste \`VITE_MCP_URL\` dans \`.env.local\`).`;
        updateMessage(sessionId, assistantId, acc ? `${acc}\n\n${reason}` : reason);
      } finally {
        abortRef.current = null;
        setStreaming(false);
      }
    },
    [
      appendMessage,
      createSession,
      currentSessionId,
      getCurrentSession,
      isStreaming,
      selectedRepo,
      setStreaming,
      updateMessage,
    ]
  );

  return { sendMessage, cancel, isStreaming };
}
