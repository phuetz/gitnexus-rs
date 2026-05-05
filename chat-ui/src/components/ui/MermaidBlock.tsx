import { useEffect, useId, useState } from 'react';
import DOMPurify from 'dompurify';

/**
 * Renders a Mermaid diagram from raw text.
 *
 * Why a lazy dynamic import:
 *   `mermaid` ships ~500 KB minified. Most chat replies are plain prose, so
 *   loading the lib up-front would inflate every page load for no reason.
 *   We import it on first render and the chunk gets cached for the rest of
 *   the session.
 *
 * Why an `useId`-derived render target:
 *   `mermaid.render` accepts a target id and writes the SVG output as a
 *   string. The id has to start with a letter, hence the `m-` prefix; it
 *   has to be stable across re-renders so React's commit phase doesn't lose
 *   the reference to the element we just decorated.
 *
 * Defense in depth on the SVG output:
 *   - Mermaid is configured with `securityLevel: 'strict'`, which sanitizes
 *     anything the user smuggled via diagram labels (Mermaid's own
 *     DOMPurify pass).
 *   - We additionally run DOMPurify on the rendered SVG before injecting
 *     it. Belt and braces — if a future Mermaid version regresses on
 *     sanitization, our pass still strips `<script>` / `on*` handlers.
 *
 * Failure modes are explicit: a malformed graph keeps the original ```mermaid
 * source visible inside a `<pre>` so the user can copy-paste it elsewhere
 * (mermaid live editor, etc.) instead of staring at a blank box.
 */
interface Props {
  text: string;
}

export function MermaidBlock({ text }: Props) {
  const id = useId();
  const svgId = `m-${id.replace(/:/g, '')}`;
  const [svg, setSvg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setError(null);
    setSvg(null);

    void (async () => {
      try {
        const { default: mermaid } = await import('mermaid');
        mermaid.initialize({
          startOnLoad: false,
          theme: 'dark',
          fontFamily: 'ui-sans-serif, system-ui, -apple-system, sans-serif',
          securityLevel: 'strict',
        });
        const { svg: rendered } = await mermaid.render(svgId, text.trim());
        if (cancelled) return;
        const purified = DOMPurify.sanitize(rendered, {
          USE_PROFILES: { svg: true, svgFilters: true },
          ADD_TAGS: ['foreignObject'],
        });
        setSvg(purified);
      } catch (e) {
        if (cancelled) return;
        setError(e instanceof Error ? e.message : String(e));
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [text, svgId]);

  if (error) {
    return (
      <div className="my-3 rounded-md border border-red-900 bg-red-950/30 p-3 text-xs">
        <p className="mb-2 font-medium text-red-300">Mermaid rendering failed</p>
        <p className="mb-2 text-red-400/80">{error}</p>
        <pre className="overflow-x-auto rounded bg-neutral-900 p-2 text-neutral-300">
          <code>{text}</code>
        </pre>
      </div>
    );
  }

  return (
    <div
      className="my-3 flex justify-center overflow-x-auto rounded-md border border-neutral-800 bg-neutral-950/60 p-4"
      data-testid="mermaid-block"
      // eslint-disable-next-line react/no-danger -- DOMPurify-sanitized SVG from Mermaid
      dangerouslySetInnerHTML={svg ? { __html: svg } : undefined}
    />
  );
}
