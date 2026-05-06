import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { Markdown } from './Markdown';
import { normalizeBareMermaid, normalizeCodeFenceLanguage } from '../../utils/markdown';

vi.mock('./MermaidBlock', () => ({
  MermaidBlock: ({ text }: { text: string }) => (
    <div data-testid="mermaid-block">{text}</div>
  ),
}));

describe('normalizeBareMermaid', () => {
  it('wraps a bare flowchart block so Mermaid can render it', () => {
    const normalized = normalizeBareMermaid(`Flux global
flowchart TD
    A[Controller] --> B[Service]
    B --> C[PDF]
Étapes détaillées
Le texte continue.`);

    expect(normalized).toContain('```mermaid\nflowchart TD');
    expect(normalized).toContain('    A[Controller] --> B[Service]');
    expect(normalized).toContain('```\nÉtapes détaillées');
  });

  it('does not double-wrap an already fenced Mermaid block', () => {
    const markdown = `\`\`\`mermaid
flowchart TD
    A --> B
\`\`\``;

    expect(normalizeBareMermaid(markdown)).toBe(markdown);
  });
});

describe('normalizeCodeFenceLanguage', () => {
  it('maps common LLM language aliases to highlighter names', () => {
    expect(normalizeCodeFenceLanguage('cs')).toBe('csharp');
    expect(normalizeCodeFenceLanguage('c#')).toBe('csharp');
    expect(normalizeCodeFenceLanguage('ts')).toBe('typescript');
    expect(normalizeCodeFenceLanguage('js')).toBe('javascript');
    expect(normalizeCodeFenceLanguage('ps1')).toBe('powershell');
    expect(normalizeCodeFenceLanguage('shell')).toBe('bash');
  });

  it('keeps unknown language tags instead of dropping highlighting', () => {
    expect(normalizeCodeFenceLanguage('rust')).toBe('rust');
    expect(normalizeCodeFenceLanguage(undefined)).toBeUndefined();
  });
});

describe('Markdown Mermaid rendering', () => {
  it('routes bare Mermaid flowcharts to MermaidBlock', () => {
    render(
      <Markdown>{`Flux global
flowchart TD
    A[Controller] --> B[Service]
Étapes détaillées`}</Markdown>
    );

    expect(screen.getByTestId('mermaid-block').textContent).toContain('flowchart TD');
    expect(screen.getByText('Étapes détaillées')).toBeTruthy();
  });

  it('treats common Mermaid typo aliases as diagrams', () => {
    const markdown = [
      '```maimaid',
      'sequenceDiagram',
      '    MVC->>BAL: CreerCourrierMasse',
      '```',
    ].join('\n');

    render(<Markdown>{markdown}</Markdown>);

    expect(screen.getByTestId('mermaid-block').textContent).toContain('sequenceDiagram');
  });

  it('keeps code readable while the syntax highlighter chunk loads', () => {
    render(<Markdown>{'```ts\nconst value: number = 1;\n```'}</Markdown>);

    expect(screen.getByText(/const value: number = 1/)).toBeTruthy();
  });
});
