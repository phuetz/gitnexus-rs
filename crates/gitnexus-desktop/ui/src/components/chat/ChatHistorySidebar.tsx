import { useState } from "react";
import { Plus, MessageSquare, Trash2, Pencil } from "lucide-react";
import { useChatSessionStore } from "../../stores/chat-session-store";
import { useAppStore } from "../../stores/app-store";
import { useI18n } from "../../hooks/use-i18n";

export function ChatHistorySidebar() {
  const { t } = useI18n();
  const activeRepo = useAppStore((s) => s.activeRepo) || "global";
  const {
    activeSessionId,
    createSession,
    deleteSession,
    setActiveSession,
    renameSession,
    getSessionsForRepo,
  } = useChatSessionStore();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");

  const repoSessions = getSessionsForRepo(activeRepo);

  const handleNewChat = () => {
    createSession(activeRepo, t("chat.newChat"));
  };

  return (
    <div
      className="flex flex-col h-full w-full"
      style={{
        background: "var(--bg-0)",
        borderRight: "1px solid var(--surface-border)",
      }}
    >
      <div
        className="shrink-0 flex items-center justify-between px-4 py-3"
        style={{ borderBottom: "1px solid var(--surface-border)" }}
      >
        <h3
          style={{
            fontFamily: "var(--font-display)",
            fontSize: 14,
            fontWeight: 600,
            color: "var(--text-1)",
          }}
        >
          {t("chat.recentChats")}
        </h3>
        <button
          onClick={handleNewChat}
          className="p-1 rounded-lg transition-colors flex items-center justify-center"
          style={{ background: "var(--accent)", color: "white", width: 24, height: 24 }}
          title={t("chat.newChat")}
          aria-label="Create new chat"
        >
          <Plus size={14} />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto py-2 px-2" style={{ gap: 4, display: "flex", flexDirection: "column" }}>
        {repoSessions.length === 0 ? (
          <div className="text-center px-4 py-8" style={{ color: "var(--text-3)", fontSize: 12 }}>
            {t("chat.noRecentChats")}
          </div>
        ) : (
          repoSessions.map((session) => {
            const isActive = session.id === activeSessionId;
            return (
              <div
                key={session.id}
                className="group relative flex items-center rounded-lg px-3 py-2 cursor-pointer transition-colors"
                style={{
                  background: isActive ? "var(--bg-2)" : "transparent",
                  color: isActive ? "var(--text-0)" : "var(--text-2)",
                }}
                onClick={() => setActiveSession(session.id)}
              >
                <MessageSquare size={14} className="shrink-0 mr-3" />
                {editingId === session.id ? (
                  <input
                    autoFocus
                    className="flex-1 text-[13px] bg-transparent outline-none border-b"
                    style={{ color: "var(--text-0)", borderColor: "var(--accent)" }}
                    value={editValue}
                    onChange={(e) => setEditValue(e.target.value)}
                    onBlur={() => {
                      if (editValue.trim()) renameSession(session.id, editValue.trim());
                      setEditingId(null);
                    }}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") { e.currentTarget.blur(); }
                      if (e.key === "Escape") { setEditingId(null); }
                    }}
                    onClick={(e) => e.stopPropagation()}
                  />
                ) : (
                  <div
                    className="flex-1 truncate text-[13px]"
                    style={{ fontWeight: isActive ? 500 : 400 }}
                    title={session.title}
                    onDoubleClick={(e) => {
                      e.stopPropagation();
                      setEditingId(session.id);
                      setEditValue(session.title);
                    }}
                  >
                    {session.title}
                  </div>
                )}
                <div className="shrink-0 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
                  <button
                    style={{ color: "var(--text-3)" }}
                    className="p-1"
                    onClick={(e) => {
                      e.stopPropagation();
                      setEditingId(session.id);
                      setEditValue(session.title);
                    }}
                    title={t("chat.renameChat")}
                  >
                    <Pencil size={11} />
                  </button>
                  <button
                    style={{ color: "var(--red-400)" }}
                    className="p-1"
                    onClick={(e) => {
                      e.stopPropagation();
                      deleteSession(session.id);
                    }}
                    title={t("chat.deleteChat")}
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
