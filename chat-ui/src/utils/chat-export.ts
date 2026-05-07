import type { LlmConfigInfo } from '../api/mcp-client';
import type { Message, Session } from '../types/chat';
import { formatExportTimestamp } from './dates';
import { looksLikeMermaid, normalizeCodeFenceLanguage } from './markdown';

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
    lines.push(sanitizeExportText(message.content).trim());
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

  const transcriptHtml = renderedTranscript
    ? transcriptHtmlFromRenderedElement(renderedTranscript)
    : fallbackTranscriptHtml(session);
  popup.document.open();
  popup.document.write(printableHtml(session, metadata, transcriptHtml));
  popup.document.close();
  popup.focus();
  popup.setTimeout(() => popup.print(), 650);
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
  const maxTokens =
    typeof llm.maxTokens === 'number' && Number.isFinite(llm.maxTokens)
      ? `, max ${llm.maxTokens} tokens`
      : '';
  return `${provider} / ${model}${effort}${maxTokens}`;
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
    .map((message) => {
      const toolSummary = formatToolCalls(message);
      return `
        <section class="print-message print-message-${escapeHtml(message.role)}">
          <h2>${escapeHtml(messageLabel(message))}</h2>
          ${toolSummary ? `<p class="print-tools">Outils: ${escapeHtml(toolSummary)}</p>` : ''}
          ${fallbackContentHtml(message.content)}
        </section>`;
    })
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
    * {
      box-sizing: border-box;
      -webkit-print-color-adjust: exact;
      print-color-adjust: exact;
    }
    html {
      color-scheme: light;
    }
    body {
      margin: 0;
      background: #fff;
      color: #111827;
      font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
      line-height: 1.5;
    }
    body > header,
    body > main {
      margin: 0 auto;
      max-width: 980px;
      padding: 0 28px;
    }
    header {
      border-bottom: 1px solid #d1d5db;
      margin-bottom: 22px;
      padding-bottom: 16px;
      padding-top: 28px;
    }
    h1 {
      font-size: 24px;
      margin: 0 0 8px;
    }
    h2 {
      break-after: avoid;
      font-size: 14px;
      margin: 0 0 8px;
      color: #374151;
    }
    h3, h4 {
      break-after: avoid;
      color: #1f2937;
      margin: 14px 0 6px;
    }
    p {
      margin: 0 0 10px;
    }
    .meta {
      color: #4b5563;
      font-size: 12px;
    }
    .print-help {
      margin: 18px auto 0;
      max-width: 980px;
      padding: 10px 28px;
      color: #475569;
      font-size: 12px;
    }
    button, [role="button"], [aria-label*="Copier"], [aria-label*="Régénérer"] {
      display: none !important;
    }
    a {
      color: #1d4ed8;
      text-decoration: none;
    }
    a[href^="http"]::after {
      color: #64748b;
      content: " (" attr(href) ")";
      font-size: 10px;
      overflow-wrap: anywhere;
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
      word-break: break-word;
    }
    code {
      color: #111827;
      font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
      font-size: 11px;
    }
    blockquote {
      border-left: 3px solid #cbd5e1;
      color: #475569;
      margin: 12px 0;
      padding-left: 12px;
    }
    table {
      border-collapse: collapse;
      font-size: 11px;
      margin: 12px 0;
      width: 100%;
    }
    th, td {
      border: 1px solid #d1d5db;
      padding: 6px 8px;
      text-align: left;
      vertical-align: top;
    }
    th {
      background: #f1f5f9;
      font-weight: 700;
    }
    tr {
      break-inside: avoid;
    }
    details {
      border: 1px solid #e5e7eb;
      border-radius: 6px;
      margin: 12px 0;
      padding: 8px 10px;
    }
    summary {
      font-weight: 700;
    }
    figure {
      margin: 12px 0;
    }
    figcaption {
      color: #475569;
      font-size: 11px;
      font-weight: 700;
      margin-bottom: 6px;
      text-transform: uppercase;
    }
    .chat-transcript > *,
    .print-message {
      margin-bottom: 18px;
    }
    .print-message {
      border-left: 3px solid #e5e7eb;
      padding-left: 12px;
    }
    .print-message-user {
      border-left-color: #8b5cf6;
    }
    .print-message-assistant {
      border-left-color: #10b981;
    }
    .print-tools {
      color: #4b5563;
      font-size: 12px;
      font-style: italic;
      margin: 0 0 8px;
    }
    .print-code-block,
    .print-diagram,
    [data-testid="mermaid-block"] {
      break-inside: avoid;
      page-break-inside: avoid;
    }
    .print-diagram,
    [data-testid="mermaid-block"] {
      background: #fff;
      border: 1px solid #cbd5e1;
      border-radius: 8px;
      margin: 14px 0;
      overflow: hidden;
      padding: 10px;
    }
    [data-testid="mermaid-block"] > div:first-child,
    [data-testid="mermaid-block"] [data-testid="mermaid-loading"] {
      display: none !important;
    }
    [data-testid="mermaid-block"] svg {
      background: #fff;
      color: #111827;
      display: block;
      height: auto;
      margin: 0 auto;
      max-width: 100%;
    }
    [data-print-mermaid-source] {
      display: none !important;
    }
    [data-print-mermaid-source][data-print-visible="true"] {
      background: #fff7ed;
      border: 1px solid #fed7aa;
      color: #7c2d12;
      display: block !important;
      margin: 8px 0 0;
    }
    .print-source-ref {
      border: 1px solid #ddd6fe;
      border-radius: 3px;
      color: #5b21b6;
      font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
      padding: 0 3px;
    }
    @page {
      margin: 18mm;
    }
    @media print {
      body > header,
      body > main {
        max-width: none;
        padding: 0;
      }
      header {
        padding-top: 0;
      }
      .print-help {
        display: none;
      }
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
  <div class="print-help">Aperçu PDF GitNexus. Utilise "Enregistrer au format PDF" dans la fenêtre d'impression.</div>
  <main class="chat-transcript">${transcriptHtml}</main>
</body>
</html>`;
}

function exportableMessages(session: Session): Message[] {
  return session.messages.filter((message) => message.content.trim());
}

function transcriptHtmlFromRenderedElement(element: HTMLElement): string {
  const clone = element.cloneNode(true) as HTMLElement;
  sanitizeTextNodes(clone);
  replacePrintableButtons(clone);
  prepareMermaidFallbacks(clone);
  clone.querySelectorAll('[role="dialog"], [data-export-skip]').forEach((node) => node.remove());
  return clone.innerHTML;
}

function sanitizeTextNodes(node: Node) {
  if (node.nodeType === Node.TEXT_NODE) {
    node.nodeValue = sanitizeExportText(node.nodeValue ?? '');
    return;
  }

  node.childNodes.forEach((child) => sanitizeTextNodes(child));
}

function replacePrintableButtons(root: HTMLElement) {
  root.querySelectorAll('button').forEach((button) => {
    const text = sanitizeExportText(button.textContent ?? '').trim();
    if (text && !button.querySelector('svg')) {
      const span = button.ownerDocument.createElement('span');
      span.className = 'print-source-ref';
      span.textContent = text;
      button.replaceWith(span);
      return;
    }
    button.remove();
  });
}

function prepareMermaidFallbacks(root: HTMLElement) {
  root.querySelectorAll<HTMLElement>('[data-testid="mermaid-block"]').forEach((block) => {
    const hasSvg = !!block.querySelector('svg');
    const source = block.querySelector<HTMLElement>('[data-print-mermaid-source]');
    if (!hasSvg && source) {
      source.dataset.printVisible = 'true';
    }
  });
}

function fallbackContentHtml(content: string): string {
  return parseFallbackBlocks(sanitizeExportText(content.trim()))
    .map((block) => {
      if (block.kind === 'code') {
        return fallbackCodeHtml(block.text, block.language);
      }
      return fallbackTextHtml(block.text);
    })
    .join('\n');
}

type FallbackBlock =
  | { kind: 'text'; text: string }
  | { kind: 'code'; text: string; language: string | undefined };

function parseFallbackBlocks(content: string): FallbackBlock[] {
  const blocks: FallbackBlock[] = [];
  const textBuffer: string[] = [];
  const codeBuffer: string[] = [];
  let language: string | undefined;
  let inCode = false;

  const flushText = () => {
    const text = textBuffer.join('\n').trim();
    if (text) blocks.push({ kind: 'text', text });
    textBuffer.length = 0;
  };

  const flushCode = () => {
    blocks.push({ kind: 'code', language, text: codeBuffer.join('\n').replace(/\n$/, '') });
    codeBuffer.length = 0;
    language = undefined;
    inCode = false;
  };

  for (const line of content.split('\n')) {
    const opening = /^```([^\s`]*)?.*$/.exec(line.trim());
    if (opening && !inCode) {
      flushText();
      language = normalizeCodeFenceLanguage(opening[1]);
      inCode = true;
      continue;
    }

    if (line.trim() === '```' && inCode) {
      flushCode();
      continue;
    }

    if (inCode) {
      codeBuffer.push(line);
    } else {
      textBuffer.push(line);
    }
  }

  if (inCode) {
    flushCode();
  }
  flushText();
  return blocks;
}

function fallbackTextHtml(text: string): string {
  return text
    .split(/\n{2,}/)
    .map((chunk) => {
      const trimmed = chunk.trim();
      const heading = /^(#{1,4})\s+(.+)$/.exec(trimmed);
      if (heading) {
        const level = Math.min(4, heading[1].length + 2);
        return `<h${level}>${escapeHtml(heading[2])}</h${level}>`;
      }
      return `<p>${trimmed.split('\n').map(escapeHtml).join('<br />')}</p>`;
    })
    .join('\n');
}

function fallbackCodeHtml(code: string, language: string | undefined): string {
  const isMermaid = language === 'mermaid' || looksLikeMermaid(code);
  const classes = isMermaid ? 'print-diagram print-diagram-mermaid' : 'print-code-block';
  const label = isMermaid ? 'Diagramme Mermaid (source)' : `Code${language ? ` ${language}` : ''}`;
  return `<figure class="${classes}">
    <figcaption>${escapeHtml(label)}</figcaption>
    <pre><code>${escapeHtml(code)}</code></pre>
  </figure>`;
}

function sanitizeExportText(value: string): string {
  return value
    .replace(/\r\n?/g, '\n')
    .replace(/\u200B/g, '')
    .replace(/\u200C/g, '')
    .replace(/\u200D/g, '')
    .replace(/\uFE0F/g, '');
}

function escapeHtml(value: string): string {
  return sanitizeExportText(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
