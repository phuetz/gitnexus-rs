import type { LlmConfigInfo } from '../api/mcp-client';
import type { Message, Session } from '../types/chat';
import { formatExportTimestamp } from './dates';

interface ExportMetadata {
  repo: string | null;
  llm: LlmConfigInfo | null;
}

export function conversationToMarkdown(session: Session, metadata: ExportMetadata): string {
  const messages = exportableMessages(session);
  const lines: string[] = [
    `# ${session.title || 'Conversation GitNexus'}`,
    '',
    `- Projet: ${metadata.repo ?? 'non sélectionné'}`,
    `- LLM: ${formatLlmLabel(metadata.llm)}`,
    `- Conversation créée: ${formatExportTimestamp(session.createdAt) || 'inconnue'}`,
    `- Dernière activité: ${formatExportTimestamp(session.updatedAt) || 'inconnue'}`,
    `- Messages exportés: ${messages.length}`,
    `- Export: ${formatExportTimestamp(Date.now())}`,
    '',
  ];

  for (const message of messages) {
    lines.push(`## ${messageLabel(message)}`);
    lines.push('');
    const toolSummary = formatToolCalls(message);
    if (toolSummary) {
      lines.push(`_Outils: ${toolSummary}_`);
      lines.push('');
    }
    lines.push(message.content.trim());
    lines.push('');
  }

  return lines.join('\n').trimEnd() + '\n';
}

export function exportMarkdown(session: Session, metadata: ExportMetadata) {
  downloadTextFile(exportFilename(session, metadata.repo, 'md'), conversationToMarkdown(session, metadata));
}

export function exportPdf(session: Session, metadata: ExportMetadata, renderedTranscript?: HTMLElement | null) {
  const popup = window.open('', '_blank', 'width=980,height=760');
  if (!popup) {
    throw new Error('Le navigateur a bloqué la fenêtre d’export PDF.');
  }

  const transcriptHtml = renderedTranscript?.innerHTML || fallbackTranscriptHtml(session);
  popup.document.open();
  popup.document.write(printableHtml(session, metadata, transcriptHtml));
  popup.document.close();
  popup.focus();
  popup.setTimeout(() => popup.print(), 350);
}

export function exportFilename(session: Session, repo: string | null, extension: 'md' | 'pdf'): string {
  const base = [repo, session.title || 'conversation']
    .filter(Boolean)
    .join('-')
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 90);
  const stamp = new Date().toISOString().replace(/[-:]/g, '').replace(/\..+$/, '').replace('T', '-');
  return `gitnexus-${base || 'conversation'}-${stamp}.${extension}`;
}

function messageLabel(message: Message): string {
  const who =
    message.role === 'user' ? 'Vous' : message.role === 'assistant' ? 'GitNexus' : 'Système';
  const timestamp = formatExportTimestamp(message.createdAt);
  return timestamp ? `${who} - ${timestamp}` : who;
}

function formatLlmLabel(llm: LlmConfigInfo | null): string {
  if (!llm?.configured) return 'non configuré';
  const provider = llm.provider ?? 'provider inconnu';
  const model = llm.model ?? 'modèle inconnu';
  const effort = llm.reasoningEffort ? `, raisonnement ${llm.reasoningEffort}` : '';
  return `${provider} / ${model}${effort}`;
}

function formatToolCalls(message: Message): string {
  const calls = message.toolCalls ?? [];
  if (calls.length === 0) return '';
  return calls.map((call) => `${call.name} (${call.status})`).join(', ');
}

function downloadTextFile(filename: string, content: string) {
  const blob = new Blob([content], { type: 'text/markdown;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

function fallbackTranscriptHtml(session: Session): string {
  return exportableMessages(session)
    .map(
      (message) => `
        <section class="print-message print-message-${escapeHtml(message.role)}">
          <h2>${escapeHtml(messageLabel(message))}</h2>
          <pre>${escapeHtml(message.content.trim())}</pre>
        </section>`
    )
    .join('\n');
}

function printableHtml(session: Session, metadata: ExportMetadata, transcriptHtml: string): string {
  const messages = exportableMessages(session);
  return `<!doctype html>
<html lang="fr">
<head>
  <meta charset="utf-8" />
  <title>${escapeHtml(session.title || 'Conversation GitNexus')}</title>
  <style>
    body {
      margin: 32px;
      background: #fff;
      color: #111827;
      font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
      line-height: 1.5;
    }
    header {
      border-bottom: 1px solid #d1d5db;
      margin-bottom: 20px;
      padding-bottom: 14px;
    }
    h1 {
      font-size: 22px;
      margin: 0 0 8px;
    }
    h2 {
      font-size: 14px;
      margin: 18px 0 8px;
      color: #374151;
    }
    .meta {
      color: #4b5563;
      font-size: 12px;
    }
    button, [aria-label*="Copier"], [aria-label*="Régénérer"] {
      display: none !important;
    }
    svg {
      max-width: 100%;
      height: auto;
    }
    pre {
      background: #f3f4f6;
      border: 1px solid #e5e7eb;
      border-radius: 6px;
      color: #111827;
      overflow-wrap: anywhere;
      padding: 10px;
      white-space: pre-wrap;
    }
    code {
      color: #111827;
      font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    }
    .chat-transcript > * {
      break-inside: avoid;
      margin-bottom: 16px;
    }
    @page {
      margin: 18mm;
    }
  </style>
</head>
<body>
  <header>
    <h1>${escapeHtml(session.title || 'Conversation GitNexus')}</h1>
    <div class="meta">Projet : ${escapeHtml(metadata.repo ?? 'non sélectionné')}</div>
    <div class="meta">LLM : ${escapeHtml(formatLlmLabel(metadata.llm))}</div>
    <div class="meta">Conversation créée : ${escapeHtml(formatExportTimestamp(session.createdAt) || 'inconnue')}</div>
    <div class="meta">Dernière activité : ${escapeHtml(formatExportTimestamp(session.updatedAt) || 'inconnue')}</div>
    <div class="meta">Messages exportés : ${messages.length}</div>
    <div class="meta">Export : ${escapeHtml(formatExportTimestamp(Date.now()))}</div>
  </header>
  <main class="chat-transcript">${transcriptHtml}</main>
</body>
</html>`;
}

function exportableMessages(session: Session): Message[] {
  return session.messages.filter((message) => message.content.trim());
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
