/**
 * Lazy-loaded Shiki highlighter singleton.
 * Returns tokenized lines for a given code + language so we can render
 * them with our own line-number gutter and hover styles.
 */
import { useState, useEffect } from "react";
import type { HighlighterCore, ThemedToken } from "shiki";

/** Map from our backend language names to Shiki grammar ids */
const LANG_MAP: Record<string, string> = {
  rust: "rust",
  typescript: "typescript",
  javascript: "javascript",
  python: "python",
  java: "java",
  c: "c",
  cpp: "cpp",
  csharp: "csharp",
  go: "go",
  php: "php",
  ruby: "ruby",
  kotlin: "kotlin",
  swift: "swift",
  toml: "toml",
  json: "json",
  yaml: "yaml",
  markdown: "markdown",
  html: "html",
  css: "css",
  sql: "sql",
  bash: "bash",
  shell: "bash",
};

// Singleton highlighter — created once, reused everywhere
let highlighterPromise: Promise<HighlighterCore> | null = null;

function getHighlighter(): Promise<HighlighterCore> {
  if (!highlighterPromise) {
    highlighterPromise = import("shiki").then((shiki) =>
      shiki.createHighlighter({
        themes: ["github-dark-default"],
        langs: [
          "rust",
          "typescript",
          "javascript",
          "python",
          "java",
          "c",
          "cpp",
          "csharp",
          "go",
          "php",
          "ruby",
          "kotlin",
          "swift",
          "toml",
          "json",
          "yaml",
          "markdown",
          "html",
          "css",
          "sql",
          "bash",
        ],
      })
    );
  }
  return highlighterPromise;
}

export type TokenizedLine = ThemedToken[];

/**
 * Hook that returns tokenized lines for syntax highlighting.
 * Falls back to plain text if the language isn't supported or shiki hasn't loaded yet.
 */
export function useShikiTokens(
  code: string | undefined,
  language: string | undefined
): { tokens: TokenizedLine[] | null; ready: boolean } {
  const [tokens, setTokens] = useState<TokenizedLine[] | null>(null);
  const [ready, setReady] = useState(false);

  // Handle no-code case via render-time state adjustment (avoids setState in effect)
  const [prevCode, setPrevCode] = useState(code);
  if (code !== prevCode) {
    setPrevCode(code);
    if (!code) {
      setTokens(null);
      setReady(true);
    }
  }

  // Async highlighter load for actual code
  useEffect(() => {
    if (!code) return;

    let cancelled = false;
    const lang = LANG_MAP[language || ""] || "text";

    getHighlighter()
      .then((hl) => {
        if (cancelled) return;
        // codeToTokensBase returns an array of lines, each line is an array of tokens
        const result = hl.codeToTokensBase(code, {
          lang: lang as never,
          theme: "github-dark-default",
        });
        setTokens(result);
        setReady(true);
      })
      .catch((err) => {
        if (cancelled) return;
        console.warn("Shiki tokenization failed, falling back to plain text:", err);
        setTokens(null);
        setReady(true);
      });

    return () => {
      cancelled = true;
    };
  }, [code, language]);

  return { tokens, ready };
}
