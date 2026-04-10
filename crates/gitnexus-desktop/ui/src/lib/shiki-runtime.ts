import { createBundledHighlighter, type HighlighterGeneric } from "@shikijs/core";
import { createJavaScriptRegexEngine } from "@shikijs/engine-javascript";

type UiLang =
  | "rust"
  | "typescript"
  | "javascript"
  | "python"
  | "java"
  | "c"
  | "cpp"
  | "csharp"
  | "go"
  | "php"
  | "ruby"
  | "kotlin"
  | "swift"
  | "toml"
  | "json"
  | "yaml"
  | "markdown"
  | "html"
  | "css"
  | "sql"
  | "bash";

type UiTheme = "github-dark" | "github-dark-default" | "tokyo-night";

const createUiHighlighter = createBundledHighlighter<UiLang, UiTheme>({
  engine: () => createJavaScriptRegexEngine(),
  langs: {
    rust: () => import("@shikijs/langs/rust"),
    typescript: () => import("@shikijs/langs/typescript"),
    javascript: () => import("@shikijs/langs/javascript"),
    python: () => import("@shikijs/langs/python"),
    java: () => import("@shikijs/langs/java"),
    c: () => import("@shikijs/langs/c"),
    cpp: () => import("@shikijs/langs/cpp"),
    csharp: () => import("@shikijs/langs/csharp"),
    go: () => import("@shikijs/langs/go"),
    php: () => import("@shikijs/langs/php"),
    ruby: () => import("@shikijs/langs/ruby"),
    kotlin: () => import("@shikijs/langs/kotlin"),
    swift: () => import("@shikijs/langs/swift"),
    toml: () => import("@shikijs/langs/toml"),
    json: () => import("@shikijs/langs/json"),
    yaml: () => import("@shikijs/langs/yaml"),
    markdown: () => import("@shikijs/langs/markdown"),
    html: () => import("@shikijs/langs/html"),
    css: () => import("@shikijs/langs/css"),
    sql: () => import("@shikijs/langs/sql"),
    bash: () => import("@shikijs/langs/bash"),
  },
  themes: {
    "github-dark": () => import("@shikijs/themes/github-dark"),
    "github-dark-default": () => import("@shikijs/themes/github-dark-default"),
    "tokyo-night": () => import("@shikijs/themes/tokyo-night"),
  },
});

let uiHighlighterPromise: Promise<HighlighterGeneric<UiLang, UiTheme>> | null = null;

export function getUiHighlighter() {
  if (!uiHighlighterPromise) {
    uiHighlighterPromise = createUiHighlighter({
      themes: ["github-dark-default"],
      langs: [],
    });
  }
  return uiHighlighterPromise;
}
