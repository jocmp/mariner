#!/usr/bin/env node
/**
 * Copy fixtures from the JS mercury-parser repo and record Mercury.parse
 * output for each one, for the Rust parity tests:
 *
 *   crates/mercury-parser/tests/fixtures/<name>.html
 *   crates/mercury-parser/tests/snapshots/<name>.json
 *
 * where <name> is the fixture's path relative to the JS `fixtures/` dir.
 * The URL passed to Mercury is `https://<domain>/`, with the domain taken
 * from the file or directory name (anything after `--` is a variant label).
 *
 * Usage:
 *   TZ=UTC node crates/mercury-parser/scripts/dump-snapshots.mjs [--only <prefix>]
 *
 * MERCURY_PARSER_DIR defaults to ../mercury-parser, next to the mariner
 * checkout. Run `npm run build` there first. TZ=UTC is required: JS parses
 * offset-less dates in local time.
 */
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const CRATE = path.resolve(HERE, '..');
const TESTS_DIR = path.join(CRATE, 'tests');
const MERCURY_DIR =
  process.env.MERCURY_PARSER_DIR || path.resolve(CRATE, '..', '..', '..', 'mercury-parser');

if (process.env.TZ !== 'UTC') {
  console.error('Run with TZ=UTC so offset-less dates parse deterministically.');
  process.exit(1);
}

const args = process.argv.slice(2);
const onlyIndex = args.indexOf('--only');
const only = onlyIndex >= 0 ? args[onlyIndex + 1] : null;

const require = createRequire(import.meta.url);
const Mercury = require(path.join(MERCURY_DIR, 'dist', 'mercury.js'));

const fixturesRoot = path.join(MERCURY_DIR, 'fixtures');
let count = 0;
let errors = 0;

for (const relative of listFixtures(fixturesRoot)) {
  if (only && !relative.startsWith(only)) continue;

  const name = relative.replace(/\.html$/, '');
  const domain = name.split('/')[0].split('--')[0];
  const html = fs.readFileSync(path.join(fixturesRoot, relative), 'utf-8');

  try {
    const result = await Mercury.parse(`https://${domain}/`, {
      html,
      fetchAllPages: false,
    });
    writeFile(path.join(TESTS_DIR, 'fixtures', relative), html);
    writeFile(
      path.join(TESTS_DIR, 'snapshots', `${name}.json`),
      `${JSON.stringify(result, null, 2)}\n`
    );
    count += 1;
  } catch (e) {
    console.error(`[${name}] failed: ${e.message}`);
    errors += 1;
  }
}

console.log(`Dumped ${count} snapshot(s), ${errors} error(s).`);

function listFixtures(root, dir = '') {
  return fs
    .readdirSync(path.join(root, dir), { withFileTypes: true })
    .sort((a, b) => a.name.localeCompare(b.name))
    .flatMap(entry => {
      const relative = path.join(dir, entry.name);
      if (entry.isDirectory()) return listFixtures(root, relative);
      return entry.name.endsWith('.html') ? [relative] : [];
    });
}

function writeFile(file, contents) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, contents, 'utf-8');
}
