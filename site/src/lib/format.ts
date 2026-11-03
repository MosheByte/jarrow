/** Formatting helpers shared by the server-rendered pages. */

const COUNT = new Intl.NumberFormat('en-US');

export function formatCount(n: number): string {
  return COUNT.format(n);
}

/** "1.20.1, 1.21.1 and 1.21.11": a plain English list, without a serial comma. */
export function formatList(items: readonly string[]): string {
  if (items.length <= 1) return items.join('');
  return `${items.slice(0, -1).join(', ')} and ${items[items.length - 1]}`;
}

/** "2026-11-04 02:41 UTC" for an ISO 8601 time; the input unchanged if it does not parse. */
export function formatUtc(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return `${date.toISOString().slice(0, 16).replace('T', ' ')} UTC`;
}
