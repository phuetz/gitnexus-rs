/**
 * Tauri runtime detection utilities.
 * Allows the app to degrade gracefully when opened in a regular browser
 * (e.g. during UI development without the Tauri shell).
 */

import { MOCK_RESPONSES } from "./mock-data";

/** Returns true when running inside the Tauri webview (IPC available) */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Safe invoke wrapper — calls the real Tauri invoke when available,
 * returns mock data when running in the browser (for UI dev).
 */
export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    // Small delay to simulate network latency
    await new Promise((r) => setTimeout(r, 80 + Math.random() * 120));

    if (cmd in MOCK_RESPONSES) {
      const entry = MOCK_RESPONSES[cmd];
      // Support function-based mocks that receive the invocation args
      const value = typeof entry === "function" ? entry(args) : entry;
      // Deep-clone to avoid mutation issues
      return structuredClone(value) as T;
    }
    console.warn(`[GitNexus mock] No mock data for command "${cmd}"`, args);
    return undefined as T;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}
