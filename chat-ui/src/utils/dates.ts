/**
 * Parse `indexedAt` from gitnexus-mcp /api/repos response.
 * The server returns two heterogeneous formats:
 *   - Unix epoch seconds with optional `Z` trailing: "1774507049Z"
 *   - ISO 8601: "2026-04-17T05:58:45Z"
 *
 * Note: `new Date("123")` returns year 122 (valid Date but obviously wrong),
 * so we test for purely-numeric input first and treat it as Unix epoch.
 */
export function parseIndexedAt(raw: string | undefined | null): Date | null {
  if (!raw) return null;
  const stripped = raw.replace(/Z$/, '');
  if (/^\d+$/.test(stripped)) {
    const unix = parseInt(stripped, 10);
    if (unix > 1_000_000_000) return new Date(unix * 1000);
    return null;
  }
  const iso = new Date(raw);
  if (!isNaN(iso.getTime())) return iso;
  return null;
}
