import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { Message, Session } from '../types/chat';

interface ChatState {
  sessions: Session[];
  currentSessionId: string | null;
  isStreaming: boolean;
  selectedRepo: string | null;
  inputDraft: string;

  createSession: (title?: string) => string;
  selectSession: (id: string) => void;
  deleteSession: (id: string) => void;
  renameSession: (id: string, title: string) => void;

  appendMessage: (sessionId: string, message: Message) => void;
  updateMessage: (sessionId: string, messageId: string, content: string) => void;
  setStreaming: (streaming: boolean) => void;
  setSelectedRepo: (repo: string | null) => void;
  setInputDraft: (text: string) => void;

  getCurrentSession: () => Session | null;
}

const newId = () => crypto.randomUUID();

export const useChatStore = create<ChatState>()(
  persist(
    (set, get) => ({
      sessions: [],
      currentSessionId: null,
      isStreaming: false,
      selectedRepo: null,
      inputDraft: '',

      createSession: (title = 'Nouvelle conversation') => {
        const id = newId();
        const now = Date.now();
        const session: Session = {
          id,
          title,
          createdAt: now,
          updatedAt: now,
          messages: [],
        };
        set((s) => ({
          sessions: [session, ...s.sessions],
          currentSessionId: id,
        }));
        return id;
      },

      selectSession: (id) => set({ currentSessionId: id }),

      deleteSession: (id) =>
        set((s) => {
          const sessions = s.sessions.filter((sess) => sess.id !== id);
          const currentSessionId =
            s.currentSessionId === id ? (sessions[0]?.id ?? null) : s.currentSessionId;
          return { sessions, currentSessionId };
        }),

      renameSession: (id, title) =>
        set((s) => ({
          sessions: s.sessions.map((sess) =>
            sess.id === id ? { ...sess, title, updatedAt: Date.now() } : sess
          ),
        })),

      appendMessage: (sessionId, message) =>
        set((s) => ({
          sessions: s.sessions.map((sess) =>
            sess.id === sessionId
              ? {
                  ...sess,
                  messages: [...sess.messages, message],
                  updatedAt: Date.now(),
                }
              : sess
          ),
        })),

      updateMessage: (sessionId, messageId, content) =>
        set((s) => ({
          sessions: s.sessions.map((sess) =>
            sess.id === sessionId
              ? {
                  ...sess,
                  messages: sess.messages.map((m) =>
                    m.id === messageId ? { ...m, content } : m
                  ),
                  updatedAt: Date.now(),
                }
              : sess
          ),
        })),

      setStreaming: (streaming) => set({ isStreaming: streaming }),
      setSelectedRepo: (repo) => set({ selectedRepo: repo }),
      setInputDraft: (text) => set({ inputDraft: text }),

      getCurrentSession: () => {
        const { sessions, currentSessionId } = get();
        return sessions.find((s) => s.id === currentSessionId) ?? null;
      },
    }),
    {
      name: 'gitnexus-chat-store',
      version: 2,
    }
  )
);
