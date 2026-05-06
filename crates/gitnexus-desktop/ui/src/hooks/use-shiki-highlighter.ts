import { useCallback, useEffect, useRef, useState } from "react";

export type TokenLine = { tokens: { content: string; color?: string; fontStyle?: number }[] };

type Highlighter = {
  codeToHtml(code: string, opts: { lang: string; theme: string }): string;
  codeToTokens(code: string, opts: { lang: string; theme: string }): { tokens: TokenLine[][] };
};

// Singleton promise — loaded once, shared across all instances
let highlighterPromise: Promise<Highlighter> | null = null;

export const LANG_MAP: Record<string, string> = {
  rs: "rust", rust: "rust",
  ts: "typescript", tsx: "typescript", typescript: "typescript",
  js: "javascript", jsx: "javascript", javascript: "javascript",
  cs: "csharp", csharp: "csharp",
  py: "python", python: "python",
  go: "go", json: "json",
  yaml: "yaml", yml: "yaml",
  md: "markdown", markdown: "markdown",
  toml: "toml", sql: "sql",
  html: "html", css: "css",
  bash: "bash", sh: "bash", shell: "bash",
};

function getHighlighterSingleton(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = import("shiki").then(({ createHighlighter }) =>
      createHighlighter({
        langs: [...new Set(Object.values(LANG_MAP))],
        themes: ["tokyo-night"],
      })
    ).catch(() => {
      // Reset so next call retries
      highlighterPromise = null;
      throw new Error("Shiki load failed");
    }) as unknown as Promise<Highlighter>;
  }
  return highlighterPromise;
}

export function useShikiHighlighter() {
  const [ready, setReady] = useState(false);
  const highlighterRef = useRef<Highlighter | null>(null);

  useEffect(() => {
    let cancelled = false;
    getHighlighterSingleton()
      .then((h) => { if (!cancelled) { highlighterRef.current = h; setReady(true); } })
      .catch(() => { /* plain text fallback */ });
    return () => { cancelled = true; };
  }, []);

  // Stable reference via useCallback — avoids infinite re-render in useEffect deps
  const tokenize = useCallback((code: string, langHint: string): TokenLine[] | null => {
    if (!highlighterRef.current) return null;
    const lang = LANG_MAP[langHint.toLowerCase()] ?? "text";
    try {
      const result = highlighterRef.current.codeToTokens(code, { lang, theme: "tokyo-night" });
      // Shiki v4 returns { tokens: Token[][] } where each entry is a line
      const raw = result?.tokens;
      if (!Array.isArray(raw)) return null;
      // Normalize: each line may be Token[] or { tokens: Token[] }
      return raw.map((line: unknown) => {
        if (Array.isArray(line)) {
          return { tokens: line as { content: string; color?: string; fontStyle?: number }[] };
        }
        return line as TokenLine;
      });
    } catch {
      return null;
    }
  }, []); // stable — highlighterRef.current changes via mutation, not state

  return { tokenize, ready };
}
