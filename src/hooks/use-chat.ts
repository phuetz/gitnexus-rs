import { useCallback } from 'react';
import { useChatStore } from '../stores/chat-store';
import { mcpClient } from '../api/mcp-client';
import type { Message } from '../types/chat';

const newId = () => crypto.randomUUID();

export function useChat() {
  const currentSessionId = useChatStore((s) => s.currentSessionId);
  const createSession = useChatStore((s) => s.createSession);
  const appendMessage = useChatStore((s) => s.appendMessage);
  const setStreaming = useChatStore((s) => s.setStreaming);
  const isStreaming = useChatStore((s) => s.isStreaming);

  const sendMessage = useCallback(
    async (content: string) => {
      const trimmed = content.trim();
      if (!trimmed || isStreaming) return;

      let sessionId = currentSessionId;
      if (!sessionId) {
        sessionId = createSession(trimmed.slice(0, 60));
      }

      const userMessage: Message = {
        id: newId(),
        role: 'user',
        content: trimmed,
        createdAt: Date.now(),
      };
      appendMessage(sessionId, userMessage);

      setStreaming(true);
      try {
        const reply = await mcpClient.chat(trimmed);
        const assistantMessage: Message = {
          id: newId(),
          role: 'assistant',
          content: reply,
          createdAt: Date.now(),
        };
        appendMessage(sessionId, assistantMessage);
      } catch (err) {
        const errMessage: Message = {
          id: newId(),
          role: 'assistant',
          content: `_Erreur :_ ${err instanceof Error ? err.message : String(err)}`,
          createdAt: Date.now(),
        };
        appendMessage(sessionId, errMessage);
      } finally {
        setStreaming(false);
      }
    },
    [appendMessage, createSession, currentSessionId, isStreaming, setStreaming]
  );

  return { sendMessage, isStreaming };
}
