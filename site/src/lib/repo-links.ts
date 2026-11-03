/**
 * Rewrites relative links in HTML rendered from a repository Markdown file (such as
 * ROADMAP.md) so they point at the file on GitHub instead of at a page that does not exist
 * on this site. Absolute URLs, site-absolute paths, fragments and mailto: links are kept.
 */

import { REPO_URL } from './site.ts';

const HREF = /(<a\b[^>]*?\shref=)(["'])(.*?)\2/gi;

function isRelative(href: string): boolean {
  if (href === '' || href.startsWith('#') || href.startsWith('/')) return false;
  // Any scheme (https:, mailto:, ...) or protocol-relative URL is left alone.
  return !/^[a-z][a-z0-9+.-]*:/i.test(href) && !href.startsWith('//');
}

/**
 * @param html HTML produced from a Markdown file in the repository.
 * @param fromDir Directory of that file relative to the repository root ("" for the root).
 */
export function rewriteRepoLinks(html: string, fromDir = ''): string {
  const base = new URL(`${REPO_URL}/blob/main/${fromDir ? `${fromDir.replace(/^\/+|\/+$/g, '')}/` : ''}`);
  return html.replace(HREF, (match, start: string, quote: string, href: string) => {
    if (!isRelative(href)) return match;
    const target = new URL(href.replaceAll('&amp;', '&'), base);
    // A directory link ("crates/") belongs under /tree/, not /blob/.
    const url = target.pathname.endsWith('/')
      ? target.toString().replace('/blob/main/', '/tree/main/')
      : target.toString();
    return `${start}${quote}${url.replaceAll('&', '&amp;')}${quote}`;
  });
}
