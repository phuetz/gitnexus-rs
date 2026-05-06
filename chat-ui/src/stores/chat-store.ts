import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { Message, Session, ToolCall } from '../types/chat';

interface ChatState {
  sessions: Session[];
  currentSessionId: string | null;
  isStreaming: boolean;
  selectedRepo: string | null;
  selectedRepoName: string | null;
  inputDraft: string;
  isSfdPanelOpen: boolean;

  createSession: (title?: string) => string;
  selectSession: (id: string) => void;
  deleteSession: (id: string) => void;
  renameSession: (id: string, title: string) => void;

  appendMessage: (sessionId: string, message: Message) => void;
  updateMessage: (sessionId: string, messageId: string, content: string) => void;
  upsertToolCall: (sessionId: string, messageId: string, toolCall: ToolCall) => void;
  removeMessagesFrom: (sessionId: string, messageId: string) => void;
  setStreaming: (streaming: boolean) => void;
  setSelectedRepo: (repo: string | null, displayName?: string | null) => void;
  setInputDraft: (text: string) => void;
  setSfdPanelOpen: (open: boolean) => void;

  getCurrentSession: () => Session | null;
}

type PersistedChatState = Pick<
  ChatState,
  'sessions' | 'currentSessionId' | 'selectedRepo' | 'selectedRepoName' | 'inputDraft'
>;

const newId = () => crypto.randomUUID();

function persistedChatState(state: ChatState): PersistedChatState {
  return {
    sessions: state.sessions,
    currentSessionId: state.currentSessionId,
    selectedRepo: state.selectedRepo,
    selectedRepoName: state.selectedRepoName,
    inputDraft: state.inputDraft,
  };
}

export function migratePersistedChatState(persistedState: unknown): PersistedChatState {
  const state = (persistedState ?? {}) as Partial<ChatState>;
  return {
    sessions: Array.isArray(state.sessions) ? state.sessions : [],
    currentSessionId: typeof state.currentSessionId === 'string' ? state.currentSessionId : null,
    selectedRepo: typeof state.selectedRepo === 'string' ? state.selectedRepo : null,
    selectedRepoName: typeof state.selectedRepoName === 'string' ? state.selectedRepoName : null,
    inputDraft: typeof state.inputDraft === 'string' ? state.inputDraft : '',
  };
}

export const useChatStore = create<ChatState>()(
  persist(
    (set, get) => ({
      sessions: [],
      currentSessionId: null,
      isStreaming: false,
      selectedRepo: null,
      selectedRepoName: null,
      inputDraft: '',
      isSfdPanelOpen: false,

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

      removeMessagesFrom: (sessionId, messageId) =>
        set((s) => ({
          sessions: s.sessions.map((sess) => {
            if (sess.id !== sessionId) return sess;
            const idx = sess.messages.findIndex((m) => m.id === messageId);
            if (idx === -1) return sess;
            return {
              ...sess,
              messages: sess.messages.slice(0, idx),
              updatedAt: Date.now(),
            };
          }),
        })),

      upsertToolCall: (sessionId, messageId, toolCall) =>
        set((s) => ({
          sessions: s.sessions.map((sess) =>
            sess.id === sessionId
              ? {
                  ...sess,
                  messages: sess.messages.map((m) => {
                    if (m.id !== messageId) return m;
                    const existing = m.toolCalls ?? [];
                    const idx = existing.findIndex((tc) => tc.id === toolCall.id);
                    const next =
                      idx === -1
                        ? [...existing, toolCall]
                        : existing.map((tc, i) => (i === idx ? { ...tc, ...toolCall } : tc));
                    return { ...m, toolCalls: next };
                  }),
                  updatedAt: Date.now(),
                }
              : sess
          ),
        })),

      setStreaming: (streaming) => set({ isStreaming: streaming }),
      setSelectedRepo: (repo, displayName) =>
        set({
          selectedRepo: repo,
          selectedRepoName: repo ? (displayName ?? repo) : null,
        }),
      setInputDraft: (text) => set({ inputDraft: text }),
      setSfdPanelOpen: (open) => set({ isSfdPanelOpen: open }),

      getCurrentSession: () => {
        const { sessions, currentSessionId } = get();
        return sessions.find((s) => s.id === currentSessionId) ?? null;
      },
    }),
    {
      name: 'gitnexus-chat-store',
      version: 3,
      partialize: persistedChatState,
      migrate: migratePersistedChatState,
    }
  )
);
