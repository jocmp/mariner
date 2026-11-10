// Builds tests/js_date_corpus.json: `new Date(string).toISOString()` (or
// null) for fuzzed and hand-picked strings, under TZ=UTC.
// Usage: TZ=UTC node crates/mercury-parser/scripts/date-corpus/build_js_date_corpus.js fixtures_log.json > crates/mercury-parser/tests/dates/js_date_corpus.json
const fs = require('fs');
const { dateStrings } = require('./fuzz');

const log = process.argv[2] ? JSON.parse(fs.readFileSync(process.argv[2], 'utf8')) : [];
const inputs = [
  ...new Set([
    ...log.map((r) => r.input),
    ...dateStrings(3, 6000),
    '-000000-01-01', '+002020-01-01', '-000001-01-01T00:00:00Z', '+275760-09-13T00:00:00.000Z',
    '+275760-09-13T00:00:00.001Z', '-271821-04-20T00:00:00.000Z', '-271821-04-19T23:59:59.999Z',
    'Jan 5 2020 10:00 +596523:00', 'Jan 5 2020 10:00 +596524:00', 'Jan\u00005 2020',
    'Jan 5 2020 10:00', 'Jan 5 2020﻿10:00', 'Jan 5 2020　10:00',
    'Jan 5 2020 10:00:00.0001234567890', '2020-01-05T10:00:00.0001234567890Z',
    '((Jan 5 2020', 'Jan 5 2020 (a) (b', 'Jan 5 2020 10:00 GMT+', 'Jan 5 2020 10:00 GMT+12345',
    'Jan 5 2020 10:00 GMT+1:', 'Jan 5 2020 10:00 GMT+01:30', 'Jan 5 2020 10:00 +01:30:45',
    '2020-01-05T10:00:00+24:00', '2020-01-05T10:00:00+23:59', '2020-01-05T10:00:00+2359',
    'Jan 5 2020 10:00:60', 'Jan 5 2020 10:60', 'Jan 5 2020 12:00 pm', 'Jan 5 2020 0:00 pm',
    'Jan 5 2020 24:00:00.000', 'Jan 5 2020 24:00:01', '2020-01-05T24:00:00.001',
    '1/2/3 4:5:6', '1 2 3', 'Jan 1 2 3', 'May 1 32', '32 May 1', '2020 32 May',
  ]),
];
const out = inputs.map((input) => {
  const d = new Date(input);
  return { input, expected: Number.isNaN(d.getTime()) ? null : d.toISOString() };
});
process.stderr.write(`${out.length} cases\n`);
process.stdout.write(`[\n${out.map((c) => JSON.stringify(c)).join(',\n')}\n]\n`);
