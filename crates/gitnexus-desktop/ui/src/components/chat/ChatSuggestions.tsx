import { MessageSquare, Code2, Network, Skull, Zap, Flame, Share2, GitBranch, Cpu, Search } from "lucide-react";
import { useI18n } from "../../hooks/use-i18n";
import { StaggerContainer, StaggerItem } from "../shared/motion";
import { useQuery } from "@tanstack/react-query";
import { commands } from "../../lib/tauri-commands";
import { useAppStore } from "../../stores/app-store";

// Slash command shortcuts shown as quick chips
export const SLASH_COMMANDS = [
  { cmd: "/expliquer ", label: "/expliquer", hint: "Expliquer un module", icon: "📖" },
  { cmd: "/algorithme ", label: "/algorithme", hint: "Décrire l'algorithme", icon: "⚙️" },
  { cmd: "/impact ", label: "/impact", hint: "Analyser le blast radius", icon: "💥" },
  { cmd: "/architecture ", label: "/architecture", hint: "Vue d'ensemble", icon: "🏗️" },
  { cmd: "/diagramme ", label: "/diagramme", hint: "Générer un diagramme", icon: "📊" },
];

const SUGGESTIONS = [
  {
    icon: Zap,
    textKey: "chat.suggestion.entryPoints",
    fallback: "What are the main entry points of the application?",
    color: "var(--amber)",
  },
  {
    icon: Code2,
    textKey: "chat.suggestion.complex",
    fallback: "Show me the most complex methods",
    color: "var(--accent)",
  },
  {
    icon: Network,
    textKey: "chat.suggestion.architecture",
    fallback: "Explain the overall architecture",
    color: "var(--green)",
  },
  {
    icon: Skull,
    textKey: "chat.suggestion.deadCode",
    fallback: "Is there any dead code?",
    color: "var(--rose)",
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
          gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))",
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
