// Reads the Lighthouse CI reports (lhci upload --target=filesystem) for one preset, prints a
// Markdown table of the representative runs, and checks the two bars `lhci assert` cannot
// express: zero third-party requests on / and, on mobile, the transfer of / against the budget.
//
//   node lighthouse-summary.mjs --preset mobile --dir out/lighthouse/mobile --config ../../lighthouserc.json
//
// Exits 1 when any bar in the config's assertMatrix, the transfer budget or the third-party
// rule is missed, so the table and the job's result never disagree. Writes summary.json beside
// the reports.

import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { parseArgs } from 'node:util';

import { SPEC } from './lib/spec.mjs';

const { values } = parseArgs({
  options: {
    preset: { type: 'string', default: 'mobile' },
    dir: { type: 'string' },
    config: { type: 'string' },
  },
});
const dir = values.dir ?? join('out', 'lighthouse', values.preset);
const manifestFile = join(dir, 'manifest.json');
if (!existsSync(manifestFile)) {
  console.error(`lighthouse-summary: ${manifestFile} not found; run lhci collect and lhci upload first.`);
  process.exit(1);
}
const manifest = JSON.parse(readFileSync(manifestFile, 'utf8'));
const config = values.config ? JSON.parse(readFileSync(values.config, 'utf8')) : null;
const matrix = config?.ci?.assert?.assertMatrix ?? [];

function thresholdsFor(url) {
  const out = {};
  for (const { matchingUrlPattern, assertions } of matrix) {
    if (!new RegExp(matchingUrlPattern).test(url)) continue;
    for (const [key, [, opts]] of Object.entries(assertions)) out[key] = opts;
  }
  return out;
}

const rows = [];
let failed = false;
let version = '';
for (const entry of manifest.filter((e) => e.isRepresentativeRun)) {
  const lhr = JSON.parse(readFileSync(entry.jsonPath.startsWith('/') || /^[A-Z]:/i.test(entry.jsonPath) ? entry.jsonPath : join(dir, entry.jsonPath), 'utf8'));
  version = lhr.lighthouseVersion ?? version;
  const url = new URL(lhr.finalDisplayedUrl ?? lhr.finalUrl ?? entry.url);
  const path = url.pathname;
  const score = (id) => (lhr.categories[id]?.score ?? null);
  const audit = (id) => lhr.audits[id]?.numericValue ?? null;
  const requests = lhr.audits['network-requests']?.details?.items ?? [];
  const thirdParty = requests.filter((r) => /^https?:/.test(r.url) && new URL(r.url).origin !== url.origin).map((r) => r.url);
  const transfer = audit('total-byte-weight');
  const t = thresholdsFor(entry.url);
  const misses = [];
  const check = (key, value, test, label) => {
    if (t[key] === undefined || value === null) return;
    if (!test(value, t[key])) misses.push(label);
  };
  check('categories:performance', score('performance'), (v, o) => v >= o.minScore, `performance ${Math.round(score('performance') * 100)} < ${Math.round((t['categories:performance']?.minScore ?? 0) * 100)}`);
  check('categories:accessibility', score('accessibility'), (v, o) => v >= o.minScore, `accessibility ${Math.round(score('accessibility') * 100)} < ${Math.round((t['categories:accessibility']?.minScore ?? 0) * 100)}`);
  check('cumulative-layout-shift', audit('cumulative-layout-shift'), (v, o) => v <= o.maxNumericValue, `CLS ${audit('cumulative-layout-shift')?.toFixed(4)} > ${t['cumulative-layout-shift']?.maxNumericValue}`);
  check('total-byte-weight', transfer, (v, o) => v <= o.maxNumericValue, `transfer ${(transfer / 1024).toFixed(1)} KiB > ${((t['total-byte-weight']?.maxNumericValue ?? 0) / 1024).toFixed(0)} KiB`);
  if (path === '/' && thirdParty.length) misses.push(`${thirdParty.length} third-party requests on /`);
  if (path === '/' && values.preset === 'mobile' && transfer !== null && transfer > SPEC.lighthouse.homeTransferKiB * 1024 && t['total-byte-weight'] === undefined) {
    misses.push(`transfer ${(transfer / 1024).toFixed(1)} KiB > ${SPEC.lighthouse.homeTransferKiB} KiB`);
  }
  if (misses.length) failed = true;
  rows.push({
    path,
    performance: score('performance'),
    accessibility: score('accessibility'),
    bestPractices: score('best-practices'),
    seo: score('seo'),
    fcp: audit('first-contentful-paint'),
    lcp: audit('largest-contentful-paint'),
    tbt: audit('total-blocking-time'),
    cls: audit('cumulative-layout-shift'),
    transferKiB: transfer === null ? null : Math.round((transfer / 1024) * 10) / 10,
    requests: requests.length,
    thirdParty,
    misses,
    report: entry.htmlPath,
  });
}

const pct = (v) => (v === null ? '-' : String(Math.round(v * 100)));
const ms = (v) => (v === null ? '-' : v >= 1000 ? `${(v / 1000).toFixed(1)} s` : `${Math.round(v)} ms`);
const bars = values.preset === 'mobile' ? `performance ${Math.round(SPEC.lighthouse.mobilePerformance * 100)} or more, accessibility 100, CLS 0, transfer of / ${SPEC.lighthouse.homeTransferKiB} KiB or less, no third-party request on /` : `performance ${Math.round(SPEC.lighthouse.desktopPerformance * 100)} or more, accessibility 100, CLS 0, no third-party request on /`;
const lines = [
  `### Lighthouse, ${values.preset}`,
  '',
  `Bars: ${bars}. Representative (median) run of ${manifest.filter((e) => e.url === manifest[0]?.url).length} per page, Lighthouse ${version}.`,
  '',
  '| Page | Performance | Accessibility | Best practices | SEO | FCP | LCP | TBT | CLS | Transfer | Requests | Third-party | Result |',
  '| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |',
  ...rows.map((r) => `| \`${r.path}\` | ${pct(r.performance)} | ${pct(r.accessibility)} | ${pct(r.bestPractices)} | ${pct(r.seo)} | ${ms(r.fcp)} | ${ms(r.lcp)} | ${ms(r.tbt)} | ${r.cls === null ? '-' : r.cls.toFixed(4)} | ${r.transferKiB ?? '-'} KiB | ${r.requests} | ${r.thirdParty.length} | ${r.misses.length ? `missed: ${r.misses.join('; ')}` : 'meets the bars'} |`),
  '',
];
for (const r of rows.filter((r) => r.thirdParty.length)) lines.push(`Third-party requests on \`${r.path}\`: ${r.thirdParty.slice(0, 10).map((u) => `\`${u}\``).join(', ')}`, '');
const md = lines.join('\n');
console.log(md);
writeFileSync(join(dir, 'summary.json'), `${JSON.stringify({ preset: values.preset, rows }, null, 2)}\n`);
writeFileSync(join(dir, 'summary.md'), `${md}\n`);
if (process.env.GITHUB_STEP_SUMMARY) writeFileSync(process.env.GITHUB_STEP_SUMMARY, `${md}\n`, { flag: 'a' });
process.exit(failed ? 1 : 0);
