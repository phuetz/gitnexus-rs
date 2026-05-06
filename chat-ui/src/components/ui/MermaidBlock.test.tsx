import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { MermaidBlock } from './MermaidBlock';

const mermaidMock = vi.hoisted(() => ({
  initialize: vi.fn(),
  render: vi.fn(),
}));

vi.mock('mermaid', () => ({
  default: mermaidMock,
}));

describe('MermaidBlock', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mermaidMock.render.mockResolvedValue({
      svg: '<svg><script>alert(1)</script><g onclick="alert(2)"><text>OK</text></g></svg>',
    });
  });

  it('shows a visible loading state, then renders sanitized SVG', async () => {
    const { container } = render(<MermaidBlock text="flowchart TD\nA --> B" />);

    expect(screen.getByTestId('mermaid-loading')).toBeTruthy();

    await waitFor(() => {
      expect(container.querySelector('svg')).toBeTruthy();
    });

    expect(container.querySelector('script')).toBeNull();
    expect(container.querySelector('[onclick]')).toBeNull();
    expect(container.textContent).toContain('OK');
  });

  it('can reveal and copy the original diagram source', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText },
      configurable: true,
    });

    const source = 'sequenceDiagram\nMVC->>BAL: CreerCourrierMasse';
    const { container } = render(<MermaidBlock text={source} />);

    await waitFor(() => {
      expect(container.querySelector('svg')).toBeTruthy();
    });

    fireEvent.click(screen.getByRole('button', { name: /afficher la source mermaid/i }));
    expect(container.querySelector('pre')?.textContent).toBe(source);

    fireEvent.click(screen.getByRole('button', { name: /copier la source mermaid/i }));
    expect(writeText).toHaveBeenCalledWith(source);
  });

  it('keeps the source visible when Mermaid cannot render the graph', async () => {
    mermaidMock.render.mockRejectedValue(new Error('Parse error'));

    const source = 'flowchart TD\nA -- B';
    const { container } = render(<MermaidBlock text={source} />);

    await waitFor(() => {
      expect(screen.getByText('Rendu Mermaid impossible')).toBeTruthy();
    });

    expect(screen.getByText('Parse error')).toBeTruthy();
    expect(container.querySelector('pre')?.textContent).toBe(source);
  });
});
