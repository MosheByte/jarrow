// Prints a Markdown table of the thirty rules from the reports the checks wrote
// (RULES_OUT/static.json and RULES_OUT/rendered.json): one row per rule, with its result and
// the first violation. The rules job appends it to the job summary.
//
//   node rules-summary.mjs [--dir out]

import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { parseArgs } from 'node:util';

import { RULES } from './lib/rules.mjs';

const { values } = parseArgs({ options: { dir: { type: 'string', default: process.env.RULES_OUT ?? 'out' } } });
const read = (name) => {
  const file = join(values.dir, name);
  return existsSync(file) ? JSON.parse(readFileSync(file, 'utf8')) : null;
};
const statics = read('static.json')?.report ?? {};
const rendered = read('rendered.json');
const renderedReport = rendered?.report ?? {};

const cell = (s) => s.replace(/\|/g, '\\|').replace(/\s+/g, ' ').slice(0, 160);
const rows = [];
let failing = 0;
for (const [n, rule] of Object.entries(RULES)) {
  const results = rule.checks.map((check) => {
    const [kind, id] = check.split(' ');
    if (check === 'lighthouse') return { check, kind: 'lighthouse' };
    const r = kind === 'rendered' ? renderedReport[id] : statics[id];
    return { check, kind, r };
  });
  const violations = results.flatMap((x) => x.r?.violations ?? []);
  const ran = results.filter((x) => x.r && !x.r.skipped);
  let result;
  if (!rule.checks.length) result = 'review';
  else if (violations.length) result = `fails (${violations.length})`;
  else if (results.some((x) => x.kind !== 'lighthouse' && !x.r)) result = 'not run';
  else if (!ran.length && results.every((x) => x.kind !== 'lighthouse')) result = 'nothing to check';
  else result = 'passes';
  if (violations.length) failing++;
  const review = rule.review ? (rule.checks.length ? `; review: ${rule.review}` : `: ${rule.review}`) : '';
  rows.push(`| ${n} | ${cell(rule.title)} | ${rule.checks.map((c) => `\`${c}\``).join(', ') || '-'} | ${result}${review} | ${violations.length ? cell(violations[0]) : ''} |`);
}

const md = [
  '### The thirty rules, automated checks',
  '',
  `${rendered ? `${rendered.routes.length} pages at 1440 x 900 and 390 x 844. ` : ''}${failing} rules with violations. Full lists: the \`rules\` artifact (static.json, rendered.json) and the test log.`,
  '',
  '| Rule | What | Checks | Result | First violation |',
  '| ---: | --- | --- | --- | --- |',
  ...rows,
  '',
].join('\n');
console.log(md);
writeFileSync(join(values.dir, 'summary.md'), `${md}\n`);
if (process.env.GITHUB_STEP_SUMMARY) writeFileSync(process.env.GITHUB_STEP_SUMMARY, `${md}\n`, { flag: 'a' });
