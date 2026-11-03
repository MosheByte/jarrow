/** Formatting helpers shared by the server-rendered pages. */

const COUNT = new Intl.NumberFormat('en-US');

export function formatCount(n: number): string {
  return COUNT.format(n);
}

/** "2026-11-04 02:41 UTC" for an ISO 8601 time; the input unchanged if it does not parse. */
export function formatUtc(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return `${date.toISOString().slice(0, 16).replace('T', ' ')} UTC`;
}
