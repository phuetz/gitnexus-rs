import { MessageSquare, Code2, Network, Skull, Zap, Flame, Share2 } from "lucide-react";
import { useI18n } from "../../hooks/use-i18n";
import { StaggerContainer, StaggerItem } from "../shared/motion";
import { useQuery } from "@tanstack/react-query";
import { commands } from "../../lib/tauri-commands";
import { useAppStore } from "../../stores/app-store";

const SUGGESTIONS = [
  {
    icon: Zap,
    textKey: "chat.suggestion.entryPoints",
    fallback: "What are the main entry points of the application?",
    color: "#e0af68",
  },
  {
    icon: Code2,
    textKey: "chat.suggestion.complex",
    fallback: "Show me the most complex methods",
    color: "#7aa2f7",
  },
  {
    icon: Network,
    textKey: "chat.suggestion.architecture",
    fallback: "Explain the overall architecture",
    color: "#9ece6a",
  },
  {
    icon: Skull,
    textKey: "chat.suggestion.deadCode",
    fallback: "Is there any dead code?",
    color: "#f7768e",
  },
];

interface Props {
  onSelect: (question: string) => void;
}

export function ChatSuggestions({ onSelect }: Props) {
  const { t } = useI18n();
  const activeRepo = useAppStore((s) => s.activeRepo);

  const { data: hotspots } = useQuery({
    queryKey: ["git-hotspots", activeRepo],
    queryFn: () => commands.getHotspots(90),
    enabled: !!activeRepo,
  });

  const dynamicSuggestions = [
    ...SUGGESTIONS,
    {
      icon: Share2,
      textKey: "chat.suggestion.diagram",
      fallback: "Generate a Mermaid diagram of the architecture",
      color: "#bb9af7",
    },
    ...(hotspots && hotspots.length > 5 ? [{
      icon: Flame,
      textKey: "chat.suggestion.hotspots",
      fallback: "What are the main hotspots (most changed files) in the code?",
      color: "#ff9e64",
    }] : [])
  ].slice(0, 4); // Keep at most 4 suggestions to fit grid

  return (
    <div
      className="flex flex-col items-center justify-center h-full"
      style={{
        padding: "40px 24px",
        background: "radial-gradient(ellipse at 50% 40%, rgba(106,161,248,0.04) 0%, transparent 60%)",
      }}
    >
      <div
        style={{
          width: 56,
          height: 56,
          borderRadius: 14,
          background: "var(--accent-subtle)",
          border: "1px solid var(--accent-border)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          marginBottom: 16,
        }}
      >
        <MessageSquare size={26} style={{ color: "var(--accent)" }} />
      </div>
      <h3
        style={{
          fontFamily: "var(--font-display)",
          fontSize: 18,
          fontWeight: 600,
          color: "var(--text-1)",
          marginBottom: 6,
        }}
      >
        {t("chat.welcomeTitle") || "Ask about your code"}
      </h3>
      <p
        style={{
          fontSize: 13,
          color: "var(--text-3)",
          marginBottom: 28,
          textAlign: "center",
          maxWidth: 360,
        }}
      >
        {t("chat.welcomeDesc") || "Ask questions about architecture, dependencies, or code quality."}
      </p>

      <StaggerContainer
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: 10,
          maxWidth: 500,
          width: "100%",
        }}
      >
        {dynamicSuggestions.map((s) => {
          const text = t(s.textKey) || s.fallback;
          return (
            <StaggerItem key={s.textKey}>
              <button
                onClick={() => onSelect(text)}
                className="flex items-start gap-3 w-full rounded-xl transition-all"
                style={{
                  padding: "14px 16px",
                  background: "var(--surface)",
                  border: "1px solid var(--surface-border)",
                  cursor: "pointer",
                  textAlign: "left",
                  color: "inherit",
                  fontFamily: "inherit",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "var(--surface-hover)";
                  e.currentTarget.style.borderColor = "var(--surface-border-hover)";
                  e.currentTarget.style.transform = "translateY(-1px)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "var(--surface)";
                  e.currentTarget.style.borderColor = "var(--surface-border)";
                  e.currentTarget.style.transform = "translateY(0)";
                }}
              >
                <s.icon size={18} style={{ color: s.color, flexShrink: 0, marginTop: 1 }} />
                <span style={{ fontSize: 12, color: "var(--text-2)", lineHeight: 1.4 }}>
                  {text}
                </span>
              </button>
            </StaggerItem>
          );
        })}
      </StaggerContainer>
    </div>
  );
}
