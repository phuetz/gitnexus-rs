import { useCallback, useRef } from 'react';
import { useChatStore } from '../stores/chat-store';
import {
  mcpClient,
  ChatStreamError,
  type ChatHistoryMessage,
  type ToolCallStreamEvent,
} from '../api/mcp-client';
import type { Message } from '../types/chat';

const newId = () => crypto.randomUUID();
const DEFAULT_SESSION_TITLE = 'Nouvelle conversation';

function titleFromMessage(content: string): string {
  return content.replace(/\s+/g, ' ').trim().slice(0, 80) || DEFAULT_SESSION_TITLE;
}

export function useChat() {
  const currentSessionId = useChatStore((s) => s.currentSessionId);
  const selectedRepo = useChatStore((s) => s.selectedRepo);
  const createSession = useChatStore((s) => s.createSession);
  const renameSession = useChatStore((s) => s.renameSession);
  const appendMessage = useChatStore((s) => s.appendMessage);
  const updateMessage = useChatStore((s) => s.updateMessage);
  const upsertToolCall = useChatStore((s) => s.upsertToolCall);
  const setStreaming = useChatStore((s) => s.setStreaming);
  const isStreaming = useChatStore((s) => s.isStreaming);
  const getCurrentSession = useChatStore((s) => s.getCurrentSession);

  const abortRef = useRef<AbortController | null>(null);

  const cancel = useCallback(() => {
    abortRef.current?.abort();
    abortRef.current = null;
    setStreaming(false);
  }, [setStreaming]);

  const removeMessagesFrom = useChatStore((s) => s.removeMessagesFrom);

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
        if (!sid) sid = createSession(DEFAULT_SESSION_TITLE);
        appendMessage(sid, errMsg);
        return;
      }

      let sessionId = currentSessionId;
      if (!sessionId) sessionId = createSession(titleFromMessage(trimmed));

      const previous = getCurrentSession();
      if (
        previous &&
        previous.id === sessionId &&
        previous.messages.length === 0 &&
        previous.title === DEFAULT_SESSION_TITLE
      ) {
        renameSession(sessionId, titleFromMessage(trimmed));
      }
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
      const onToolCall = (event: ToolCallStreamEvent) => {
        if (event.phase === 'start') {
          let parsedArgs: Record<string, unknown>;
          try {
            parsedArgs = JSON.parse(event.args) as Record<string, unknown>;
          } catch {
            parsedArgs = { raw: event.args };
          }
          upsertToolCall(sessionId!, assistantId, {
            id: event.id,
            name: event.name,
            args: parsedArgs,
            status: 'running',
          });
        } else {
          upsertToolCall(sessionId!, assistantId, {
            id: event.id,
            name: event.name,
            args: {},
            status: event.success ? 'done' : 'error',
          });
        }
      };
      try {
        await mcpClient.chatStream(
          selectedRepo,
          trimmed,
          history,
          (delta) => {
            acc += delta;
            updateMessage(sessionId!, assistantId, acc);
          },
          ctrl.signal,
          onToolCall
        );
        if (!acc) {
          updateMessage(sessionId, assistantId, '_Réponse vide reçue du serveur._');
        }
      } catch (err) {
        const aborted = err instanceof DOMException && err.name === 'AbortError';
        const isStreamErr = err instanceof ChatStreamError;
        const msg = err instanceof Error ? err.message : String(err);
        const reason = aborted
          ? '> ⚠️ _Requête annulée._'
          : isStreamErr
            ? `> ❌ **Erreur serveur** : ${msg}`
            : `> ❌ **Erreur** : ${msg}\n>\n> Vérifie le backend avec \`.\\gitnexus.cmd doctor\`, puis relance le chat avec \`.\\gitnexus.cmd chat -RestartBackend\` si le port/proxy est bloqué.`;
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
      renameSession,
      selectedRepo,
      setStreaming,
      updateMessage,
      upsertToolCall,
    ]
  );

  /**
   * Drop the assistant message with `assistantMessageId` (and anything after
   * it), then re-fire the user message that prompted it. Conversation history
   * up to that point is preserved.
   */
  const regenerate = useCallback(
    async (assistantMessageId: string) => {
      if (isStreaming) return;
      const session = getCurrentSession();
      if (!session) return;
      const idx = session.messages.findIndex((m) => m.id === assistantMessageId);
      if (idx === -1 || idx === 0) return;
      const previousUser = [...session.messages.slice(0, idx)]
        .reverse()
        .find((m) => m.role === 'user');
      if (!previousUser) return;
      removeMessagesFrom(session.id, previousUser.id);
      await sendMessage(previousUser.content);
    },
    [getCurrentSession, isStreaming, removeMessagesFrom, sendMessage]
  );

  return { sendMessage, regenerate, cancel, isStreaming };
}
