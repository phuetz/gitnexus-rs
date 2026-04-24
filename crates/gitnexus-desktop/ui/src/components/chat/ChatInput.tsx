import { forwardRef, useRef, useEffect, useCallback, useState } from "react";
import { Settings2, Send, Loader2, Microscope, Square } from "lucide-react";
import { useI18n } from "../../hooks/use-i18n";

const SLASH_COMPLETIONS = [
  { cmd: "/expliquer ",   hint: "Expliquer un module ou symbole" },
  { cmd: "/algorithme ",  hint: "Décrire l'algorithme étape par étape" },
  { cmd: "/impact ",      hint: "Analyser le blast radius" },
  { cmd: "/architecture ", hint: "Vue d'ensemble de l'architecture" },
  { cmd: "/diagramme ",   hint: "Générer un diagramme Mermaid" },
  { cmd: "/explain ",     hint: "Explain a module or symbol" },
];

interface ChatInputProps {
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  onKeyDown: (e: React.KeyboardEvent) => void;
  isPending: boolean;
  onOpenSettings?: () => void;
  deepResearch: boolean;
  hasFilters: boolean;
  /** Called when the user clicks the Stop button during streaming */
  onCancel?: () => void;
  /** True while tokens are actively streaming (shows Stop button) */
  isStreaming?: boolean;
}

export const ChatInput = forwardRef<HTMLTextAreaElement, ChatInputProps>(
  ({ value, onChange, onSend, onKeyDown, isPending, onOpenSettings, deepResearch, hasFilters, onCancel, isStreaming }, ref) => {
    const { t } = useI18n();
    const internalRef = useRef<HTMLTextAreaElement | null>(null);
    const [slashMatches, setSlashMatches] = useState<typeof SLASH_COMPLETIONS>([]);
    const [selectedIdx, setSelectedIdx] = useState(0);
    const placeholder = deepResearch
      ? t("chat.placeholder.deepResearch")
      : hasFilters
      ? t("chat.placeholder.filtered")
      : t("chat.placeholder.default");

    // Auto-resize textarea on input change (min 56px to prevent shrink)
    useEffect(() => {
      const el = internalRef.current;
      if (!el) return;
      el.style.height = "56px";
      el.style.height = `${Math.max(56, Math.min(el.scrollHeight, 200))}px`;
    }, [value]);

    // Merge forwarded ref with internal ref
    const setRefs = useCallback(
      (node: HTMLTextAreaElement | null) => {
        internalRef.current = node;
        if (typeof ref === "function") {
          ref(node);
        } else if (ref) {
          (ref as React.MutableRefObject<HTMLTextAreaElement | null>).current = node;
        }
      },
      [ref],
    );

    return (
      <div
        className="flex-shrink-0 px-4 py-3"
        style={{ borderTop: "1px solid var(--surface-border)" }}
      >
        <div
          className="chat-input-container relative flex items-end gap-2 rounded-2xl px-4 py-3 transition-all"
          style={{
            background: "var(--bg-2)",
            border: deepResearch
              ? "2px solid var(--purple)"
              : "1px solid var(--surface-border)",
            boxShadow: "0 2px 12px rgba(0,0,0,0.15)",
          }}
        >
          {/* Deep research indicator */}
          {deepResearch && (
            <Microscope
              size={16}
              className="mb-1 flex-shrink-0"
              style={{ color: "var(--purple)" }}
            />
          )}

          {/* Slash command completion popup */}
          {slashMatches.length > 0 && (
            <div
              className="absolute bottom-full left-0 mb-1 w-64 rounded-lg overflow-hidden shadow-lg z-50"
              style={{ background: "var(--bg-2)", border: "1px solid var(--surface-border)" }}
            >
              {slashMatches.map((m, i) => (
                <button
                  key={m.cmd}
                  className="w-full text-left px-3 py-2 text-[13px] flex justify-between items-center"
                  style={{
                    background: i === selectedIdx ? "var(--accent)" : "transparent",
                    color: i === selectedIdx ? "#fff" : "var(--text-0)",
                  }}
                  onMouseDown={(e) => { e.preventDefault(); onChange(m.cmd); setSlashMatches([]); internalRef.current?.focus(); }}
                >
                  <span className="font-mono font-semibold">{m.cmd.trim()}</span>
                  <span className="text-[11px] opacity-70 ml-2">{m.hint}</span>
                </button>
              ))}
            </div>
          )}
          <textarea
            ref={setRefs}
            value={value}
            onChange={(e) => {
              const v = e.target.value;
              onChange(v);
              // Slash completion: show when value starts with /
              if (v.startsWith("/") && !v.includes(" ")) {
                const q = v.toLowerCase();
                setSlashMatches(SLASH_COMPLETIONS.filter(c => c.cmd.startsWith(q)));
                setSelectedIdx(0);
              } else {
                setSlashMatches([]);
              }
            }}
            onKeyDown={(e) => {
              // Navigate and confirm slash completions
              if (slashMatches.length > 0) {
                if (e.key === "ArrowDown") { e.preventDefault(); setSelectedIdx(i => Math.min(i + 1, slashMatches.length - 1)); return; }
                if (e.key === "ArrowUp")   { e.preventDefault(); setSelectedIdx(i => Math.max(i - 1, 0)); return; }
                if (e.key === "Tab" || e.key === "Enter") {
                  e.preventDefault();
                  onChange(slashMatches[selectedIdx].cmd);
                  setSlashMatches([]);
                  return;
                }
                if (e.key === "Escape") { setSlashMatches([]); return; }
              }
              onKeyDown(e);
            }}
            placeholder={placeholder}
            aria-label={t("chat.inputLabel") || "Ask a question about the code"}
            rows={1}
            className="flex-1 bg-transparent resize-none text-[14px] outline-none min-h-[40px] max-h-[200px] leading-relaxed"
            onInput={(e) => { const el = e.currentTarget; el.style.height = "auto"; el.style.height = `${Math.min(el.scrollHeight, 200)}px`; }}
            style={{
              color: "var(--text-0)",
              fontFamily: "var(--font-body)",
            }}
          />
          <div className="flex items-center gap-1">
            {onOpenSettings && (
              <button
                onClick={onOpenSettings}
                className="p-1.5 rounded-lg transition-colors"
                style={{ color: "var(--text-3)" }}
                aria-label={t("chat.settings") || "Chat Settings"}
              >
                <Settings2 size={14} />
              </button>
            )}
            {isStreaming && onCancel ? (
              <button
                onClick={onCancel}
                aria-label="Stop streaming"
                title="Stop"
                className="p-2 rounded-xl transition-all"
                style={{
                  background: "rgba(239,68,68,0.15)",
                  border: "1px solid rgba(239,68,68,0.4)",
                  color: "rgb(239,68,68)",
                  minWidth: 36, minHeight: 36,
                  display: "flex", alignItems: "center", justifyContent: "center",
                }}
              >
                <Square size={14} />
              </button>
            ) : (
              <button
                onClick={onSend}
                disabled={!value.trim() || isPending}
                aria-label={isPending ? "Sending..." : "Send message"}
                className="p-2 rounded-xl transition-all"
                style={{
                  background: value.trim() && !isPending
                    ? deepResearch ? "var(--purple)" : "var(--accent)"
                    : "var(--bg-3)",
                  color: value.trim() && !isPending ? "#fff" : "var(--text-3)",
                  minWidth: 36, minHeight: 36,
                  display: "flex", alignItems: "center", justifyContent: "center",
                }}
              >
                {isPending ? <Loader2 size={16} className="animate-spin" /> : <Send size={16} />}
              </button>
            )}
          </div>
        </div>
        <p className="mt-1.5 text-[11px] text-center" style={{ color: "var(--text-3)" }}>
          {deepResearch
            ? t("chat.deepResearchHint")
            : t("chat.inputHint")}
        </p>
      </div>
    );
  }
);

ChatInput.displayName = "ChatInput";
