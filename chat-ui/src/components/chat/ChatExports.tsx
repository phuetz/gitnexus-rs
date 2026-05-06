import { useEffect, useRef, useState } from 'react';
import { Check, Copy, FileDown, Printer } from 'lucide-react';
import type { LlmConfigState } from '../../hooks/use-llm-config';
import { useChatStore } from '../../stores/chat-store';
import { conversationToMarkdown, exportMarkdown, exportPdf } from '../../utils/chat-export';
import { copyTextToClipboard } from '../../utils/clipboard';

interface Props {
  llm: LlmConfigState;
}

export function ChatExports({ llm }: Props) {
  const session = useChatStore((s) => s.getCurrentSession());
  const selectedRepo = useChatStore((s) => s.selectedRepo);
  const selectedRepoName = useChatStore((s) => s.selectedRepoName);
  const hasContent = !!session?.messages.some((m) => m.content.trim());
  const [copied, setCopied] = useState(false);
  const copiedResetTimer = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (copiedResetTimer.current !== null) {
        window.clearTimeout(copiedResetTimer.current);
      }
    };
  }, []);

  const exportMetadata = {
    repo: selectedRepoName ?? selectedRepo,
    llm: llm.config,
  };

  const handleCopyMarkdown = async () => {
    if (!session || !hasContent) return;
    const ok = await copyTextToClipboard(conversationToMarkdown(session, exportMetadata));
    if (ok) {
      setCopied(true);
      if (copiedResetTimer.current !== null) {
        window.clearTimeout(copiedResetTimer.current);
      }
      copiedResetTimer.current = window.setTimeout(() => setCopied(false), 1600);
    } else {
      window.alert('Impossible de copier la conversation dans ce navigateur.');
    }
  };

  const handleMarkdown = () => {
    if (!session || !hasContent) return;
    exportMarkdown(session, exportMetadata);
  };

  const handlePdf = () => {
    if (!session || !hasContent) return;
    try {
      exportPdf(session, exportMetadata, document.getElementById('gitnexus-chat-export-source'));
    } catch (e) {
      window.alert(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="flex items-center gap-1" aria-label="Exports de la conversation">
      <button
        type="button"
        onClick={() => void handleCopyMarkdown()}
        disabled={!hasContent}
        className="rounded-md border border-neutral-800 bg-neutral-900/60 p-1.5 text-neutral-300 transition hover:bg-neutral-900 disabled:cursor-not-allowed disabled:opacity-40"
        aria-label={copied ? 'Conversation Markdown copiée' : 'Copier la conversation en Markdown'}
        title={copied ? 'Copié' : 'Copier en Markdown'}
      >
        {copied ? (
          <Check className="h-3.5 w-3.5 text-emerald-400" aria-hidden />
        ) : (
          <Copy className="h-3.5 w-3.5" aria-hidden />
        )}
      </button>
      <button
        type="button"
        onClick={handleMarkdown}
        disabled={!hasContent}
        className="rounded-md border border-neutral-800 bg-neutral-900/60 p-1.5 text-neutral-300 transition hover:bg-neutral-900 disabled:cursor-not-allowed disabled:opacity-40"
        aria-label="Exporter la conversation en Markdown"
        title="Exporter en Markdown"
      >
        <FileDown className="h-3.5 w-3.5" aria-hidden />
      </button>
      <button
        type="button"
        onClick={handlePdf}
        disabled={!hasContent}
        className="rounded-md border border-neutral-800 bg-neutral-900/60 p-1.5 text-neutral-300 transition hover:bg-neutral-900 disabled:cursor-not-allowed disabled:opacity-40"
        aria-label="Exporter la conversation en PDF"
        title="Exporter en PDF"
      >
        <Printer className="h-3.5 w-3.5" aria-hidden />
      </button>
    </div>
  );
}
