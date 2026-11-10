// Runs Mercury over every fixture and records each cleanDatePublished call.
// Usage: TZ=UTC node crates/mercury-parser/scripts/date-corpus/collect_fixtures.js > fixtures_log.json
const fs = require('fs');
const path = require('path');
const { loadMercury, MERCURY } = require('./load');

async function main() {
  const Mercury = loadMercury();
  const dir = path.join(MERCURY, 'fixtures');
  const jobs = [];
  for (const entry of fs.readdirSync(dir).sort()) {
    const full = path.join(dir, entry);
    if (fs.statSync(full).isDirectory()) {
      for (const f of fs.readdirSync(full).sort()) {
        if (f.endsWith('.html')) jobs.push([entry, path.join(full, f)]);
      }
    } else if (entry.endsWith('.html')) {
      jobs.push([entry.replace(/\.html$/, '').split('--')[0], full]);
    }
  }
  const log = [];
  globalThis.__dateLog = log;
  for (const [domain, file] of jobs) {
    const html = fs.readFileSync(file, 'utf8');
    try {
      await Mercury.parse(`https://${domain}/`, { html, fallback: true, fetchAllPages: false });
    } catch (e) {
      process.stderr.write(`error ${file}: ${e.message}\n`);
    }
  }
  process.stderr.write(`${jobs.length} fixtures, ${log.length} calls\n`);
  process.stdout.write(JSON.stringify(log));
}

main();
