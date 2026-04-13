import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { ChatSource as ChatSourceType, ResearchPlan, QueryComplexity } from "../lib/tauri-commands";

export interface Message {
  id: string;
  role: "user" | "assistant";
  content: string;
  sources?: ChatSourceType[];
  model?: string | null;
  plan?: ResearchPlan;
  complexity?: QueryComplexity;
  timestamp: number;
}

export interface ChatSession {
  id: string;
  repo: string;
  title: string;
  updatedAt: number;
  messages: Message[];
}

interface ChatSessionState {
  sessions: ChatSession[];
  activeSessionId: string | null;
  
  createSession: (repo: string, initialTitle?: string) => string;
  deleteSession: (id: string) => void;
  setActiveSession: (id: string | null) => void;
  
  // Gets the active session for the current repo
  getActiveSession: (repo: string) => ChatSession | null;
  
  // Updates messages in the active session
  addMessage: (repo: string, message: Message) => void;
  updateMessage: (repo: string, messageId: string, update: Partial<Message>) => void;
  setMessages: (repo: string, messages: Message[]) => void;
  clearSessionMessages: (repo: string) => void;
  
  getSessionsForRepo: (repo: string) => ChatSession[];
}

export const useChatSessionStore = create<ChatSessionState>()(
  persist(
    (set, get) => ({
      sessions: [],
      activeSessionId: null,

      createSession: (repo, initialTitle = "New Chat") => {
        const newSession: ChatSession = {
          id: `chat-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
          repo,
          title: initialTitle,
          updatedAt: Date.now(),
          messages: [],
        };
        set((state) => ({
          sessions: [newSession, ...state.sessions],
          activeSessionId: newSession.id,
        }));
        return newSession.id;
      },

      deleteSession: (id) =>
        set((state) => ({
          sessions: state.sessions.filter((s) => s.id !== id),
          activeSessionId: state.activeSessionId === id ? null : state.activeSessionId,
        })),

      setActiveSession: (id) => set({ activeSessionId: id }),

      getActiveSession: (repo) => {
        const { sessions, activeSessionId } = get();
        if (activeSessionId) {
          const session = sessions.find((s) => s.id === activeSessionId && s.repo === repo);
          if (session) return session;
        }
        // If no active session matches the repo, find the most recent one for this repo
        const repoSessions = sessions.filter((s) => s.repo === repo).sort((a, b) => b.updatedAt - a.updatedAt);
        if (repoSessions.length > 0) {
          set({ activeSessionId: repoSessions[0].id });
          return repoSessions[0];
        }
        return null;
      },

      getSessionsForRepo: (repo) => {
        return get().sessions.filter((s) => s.repo === repo).sort((a, b) => b.updatedAt - a.updatedAt);
      },

      addMessage: (repo, message) => {
        set((state) => {
          let sessionId = state.activeSessionId;
          let sessions = [...state.sessions];
          
          // Auto-create session if none active
          if (!sessionId || !sessions.find(s => s.id === sessionId && s.repo === repo)) {
             const newSession: ChatSession = {
               id: `chat-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
               repo,
               title: message.role === 'user' ? message.content.slice(0, 30) + "..." : "New Chat",
               updatedAt: Date.now(),
               messages: [],
             };
             sessions = [newSession, ...sessions];
             sessionId = newSession.id;
          }

          return {
            activeSessionId: sessionId,
            sessions: sessions.map(s => {
              if (s.id === sessionId) {
                const newTitle = s.messages.length === 0 && message.role === 'user' 
                   ? message.content.slice(0, 30) + "..." 
                   : s.title;
                return {
                  ...s,
                  title: newTitle,
                  updatedAt: Date.now(),
                  messages: [...s.messages, message]
                };
              }
              return s;
            })
          };
        });
      },

      updateMessage: (repo, messageId, update) => {
        set((state) => {
          const sessionId = state.activeSessionId;
          if (!sessionId) return state;
          
          return {
            sessions: state.sessions.map(s => {
              if (s.id === sessionId && s.repo === repo) {
                return {
                  ...s,
                  updatedAt: Date.now(),
                  messages: s.messages.map(m => m.id === messageId ? { ...m, ...update } : m)
                };
              }
              return s;
            })
          };
        });
      },

      setMessages: (repo, messages) => {
        set((state) => {
          const sessionId = state.activeSessionId;
          if (!sessionId) return state;
          
          return {
            sessions: state.sessions.map(s => {
              if (s.id === sessionId && s.repo === repo) {
                return {
                  ...s,
                  updatedAt: Date.now(),
                  messages
                };
              }
              return s;
            })
          };
        });
      },

      clearSessionMessages: (repo) => {
        set((state) => {
          const sessionId = state.activeSessionId;
          if (!sessionId) return state;
          
          return {
            sessions: state.sessions.map(s => {
              if (s.id === sessionId && s.repo === repo) {
                return {
                  ...s,
                  updatedAt: Date.now(),
                  messages: []
                };
              }
              return s;
            })
          };
        });
      },
    }),
    {
      name: "gitnexus-chat-sessions",
    }
  )
);
