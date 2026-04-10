/**
 * Lazy-loaded Shiki highlighter singleton.
 * Returns tokenized lines for a given code + language so we can render
 * them with our own line-number gutter and hover styles.
 */
import { useState, useEffect } from "react";
import type { ThemedToken } from "@shikijs/core";
import { getUiHighlighter } from "../lib/shiki-runtime";

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

const loadedLangs = new Set<string>();

async function ensureLanguageLoaded(
  highlighter: Awaited<ReturnType<typeof getUiHighlighter>>,
  lang: string
): Promise<boolean> {
  if (loadedLangs.has(lang)) return true;
  try {
    await highlighter.loadLanguage(lang as never);
    loadedLangs.add(lang);
    return true;
  } catch {
    return false;
  }
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

  // Handle no-code or unknown-language cases via render-time state adjustment.
  // (Synchronous setState inside an effect is forbidden by react-hooks/set-state-in-effect.)
  const lang = LANG_MAP[language || ""];
  const [prevKey, setPrevKey] = useState<string>(`${code ?? ""}|${lang ?? ""}`);
  const currentKey = `${code ?? ""}|${lang ?? ""}`;
  if (currentKey !== prevKey) {
    setPrevKey(currentKey);
    if (!code || !lang) {
      // No code, or language not supported by Shiki — fall back to plain text.
      setTokens(null);
      setReady(true);
    }
  }

  // Async highlighter load for actual code with a supported language.
  useEffect(() => {
    if (!code) return;
    if (!lang) return;

    let cancelled = false;

    getUiHighlighter()
      .then(async (hl) => {
        if (cancelled) return;
        const langOk = await ensureLanguageLoaded(hl, lang);
        if (cancelled) return;
        // codeToTokensBase returns an array of lines, each line is an array of tokens
        const result = hl.codeToTokensBase(code, {
          lang: langOk ? (lang as never) : ("text" as never),
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
  }, [code, language, lang]);

  return { tokens, ready };
}
