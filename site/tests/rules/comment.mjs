// Builds the pull request comment that sets each screenshot beside its Horizon counterpart.
// The images are raw.githubusercontent.com URLs: Jarrow's from the pr-assets branch, Horizon's
// from docs/design-research/horizon/ on main.
//
//   node comment.mjs --manifest shots/manifest.json --repo jarrow-dev/jarrow \
//     --folder pr/16/<sha> --sha <sha> --label "the merge of <sha> into main" --out body.md

import { readFileSync, writeFileSync } from 'node:fs';
import { parseArgs } from 'node:util';

export const MARKER = '<!-- site-quality:screenshots -->';

const { values } = parseArgs({
  options: {
    manifest: { type: 'string' },
    repo: { type: 'string' },
    folder: { type: 'string' },
    sha: { type: 'string', default: '' },
    label: { type: 'string', default: '' },
    run: { type: 'string', default: '' },
    out: { type: 'string', default: 'comment.md' },
  },
});

const manifest = JSON.parse(readFileSync(values.manifest, 'utf8'));
const raw = (branch, path) => `https://raw.githubusercontent.com/${values.repo}/${branch}/${path}`;
const ours = (file) => raw('pr-assets', `${values.folder}/${file}`);
const theirs = (slug, suffix) => raw('main', `docs/design-research/horizon/${slug}-${suffix}.png`);
const img = (src, width, alt) => `<img src="${src}" width="${width}" alt="${alt}">`;

const lines = [
  MARKER,
  '### Screenshots beside Horizon',
  '',
  `Jarrow ${values.label || `at \`${values.sha.slice(0, 7)}\``}, built and served with \`wrangler pages dev\`, on the left; the Horizon page the [design research](https://github.com/${values.repo}/blob/main/docs/design-research/horizon.md) pairs it with on the right. First screens are captured with motion on, full pages under \`prefers-reduced-motion: reduce\`. All files: [\`${values.folder}/\`](https://github.com/${values.repo}/tree/pr-assets/${values.folder})${values.run ? ` · [run](${values.run})` : ''}.`,
  '',
];
for (const page of manifest.pages) {
  const f = page.files;
  lines.push(
    `#### \`${page.route}\` beside Horizon's \`${page.horizon}\`${page.status && page.status !== 200 ? ` (HTTP ${page.status})` : ''}`,
    '',
    '| Jarrow, 1440 x 900 | Horizon, 1440 x 900 |',
    '| --- | --- |',
    `| ${img(ours(f['1440-first']), 440, `Jarrow ${page.route} at 1440`)} | ${img(theirs(page.horizon, '1440-first'), 440, `Horizon ${page.horizon} at 1440`)} |`,
    '',
    '| Jarrow, 390 x 844 | Horizon, 390 x 844 |',
    '| --- | --- |',
    `| ${img(ours(f['390-first']), 220, `Jarrow ${page.route} at 390`)} | ${img(theirs(page.horizon, '390-first'), 220, `Horizon ${page.horizon} at 390`)} |`,
    '',
    `Full pages: Jarrow [1440](${ours(f['1440-full'])}) and [390](${ours(f['390-full'])}); Horizon [1440](${theirs(page.horizon, '1440-full')}) and [390](${theirs(page.horizon, '390-full')}).`,
    '',
  );
}
if (manifest.notes?.length) lines.push('Pages not captured:', '', ...manifest.notes.map((n) => `- ${n}`), '');
lines.push('The rules every page is held to: [`docs/design/thirty-rules.md`](https://github.com/' + values.repo + '/blob/main/docs/design/thirty-rules.md).');
writeFileSync(values.out, `${lines.join('\n')}\n`);
console.log(`comment: ${manifest.pages.length} pages, ${values.out}`);
