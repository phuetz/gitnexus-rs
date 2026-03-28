/**
 * Pre-configured Sonner toaster themed for Obsidian Observatory.
 */

import { Toaster as SonnerToaster } from "sonner";

export function Toaster() {
  return (
    <SonnerToaster
      theme="dark"
      position="bottom-right"
      toastOptions={{
        style: {
          background: "var(--bg-2)",
          border: "1px solid var(--surface-border)",
          color: "var(--text-0)",
          fontFamily: "var(--font-body)",
          fontSize: "13px",
          boxShadow: "var(--shadow-lg)",
        },
      }}
      gap={8}
      visibleToasts={4}
    />
  );
}
