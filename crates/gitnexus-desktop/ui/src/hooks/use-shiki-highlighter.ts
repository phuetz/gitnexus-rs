import { useEffect, useRef, useState } from "react";

export type TokenLine = { tokens: { content: string; color?: string; fontStyle?: number }[] };

type Highlighter = {
  codeToHtml(code: string, opts: { lang: string; theme: string }): string;
  codeToTokens(code: string, opts: { lang: string; theme: string }): { tokens: TokenLine[] };
};

// Singleton promise — Shiki is loaded once and shared across all component instances
let highlighterPromise: Promise<Highlighter> | null = null;

const LANG_MAP: Record<string, string> = {
  rs: "rust",
  rust: "rust",
  ts: "typescript",
  tsx: "typescript",
  typescript: "typescript",
  js: "javascript",
  jsx: "javascript",
  javascript: "javascript",
  cs: "csharp",
  csharp: "csharp",
  py: "python",
  python: "python",
  go: "go",
  json: "json",
  yaml: "yaml",
  yml: "yaml",
  md: "markdown",
  markdown: "markdown",
  toml: "toml",
  sql: "sql",
  html: "html",
  css: "css",
  bash: "bash",
  sh: "bash",
  shell: "bash",
};

function getHighlighterSingleton(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = import("shiki").then(({ createHighlighter }) =>
      createHighlighter({
        langs: Object.values(LANG_MAP).filter((v, i, a) => a.indexOf(v) === i),
        themes: ["tokyo-night"],
      })
    );
  }
  return highlighterPromise;
}

/**
 * Returns a highlight function once Shiki has loaded.
 * During loading, returns null — callers should show plain text fallback.
 */
export function useShikiHighlighter() {
  const [ready, setReady] = useState(false);
  const highlighterRef = useRef<Highlighter | null>(null);

  useEffect(() => {
    let cancelled = false;
    getHighlighterSingleton()
      .then((h) => {
        if (!cancelled) {
          highlighterRef.current = h;
          setReady(true);
        }
      })
      .catch(() => {
        // Shiki unavailable — plain text fallback stays
      });
    return () => { cancelled = true; };
  }, []);

  const tokenize = (code: string, langHint: string): TokenLine[] | null => {
    if (!ready || !highlighterRef.current) return null;
    const lang = LANG_MAP[langHint.toLowerCase()] ?? "text";
    try {
      return highlighterRef.current.codeToTokens(code, { lang, theme: "tokyo-night" }).tokens;
    } catch {
      return null;
    }
  };

  return { tokenize, ready };
}
