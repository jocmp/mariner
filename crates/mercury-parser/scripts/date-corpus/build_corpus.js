// Builds tests/date_corpus.json from the fixture log plus hand-written cases,
// evaluating the real JS cleanDatePublished with a frozen "now".
// Usage: TZ=UTC node crates/mercury-parser/scripts/date-corpus/build_corpus.js fixtures_log.json > crates/mercury-parser/tests/dates/date_corpus.json
const fs = require('fs');
const path = require('path');
const { loadMercury, MERCURY } = require('./load');
const { dateStrings, formatCases, rng, pick } = require('./fuzz');

const NOWS = [
  Date.UTC(2024, 6, 15, 12, 34, 56, 789), // summer (DST in US/EU)
  Date.UTC(2025, 0, 31, 23, 59, 59, 999), // winter, end of month
];

const RealDate = Date;
let frozenNow = NOWS[0];
class FakeDate extends RealDate {
  constructor(...args) {
    if (args.length === 0) super(frozenNow);
    else super(...args);
  }
  static now() {
    return frozenNow;
  }
}
globalThis.Date = FakeDate;

loadMercury();
const clean = globalThis.__cleanDatePublished;

// Every { timezone, format } combination used by custom extractors.
function extractorOptions() {
  const dir = path.join(MERCURY, 'src/extractors/custom');
  const combos = new Map();
  for (const site of fs.readdirSync(dir)) {
    const file = path.join(dir, site, 'index.js');
    if (!fs.existsSync(file)) continue;
    const src = fs.readFileSync(file, 'utf8');
    const block = src.match(/date_published:\s*\{([\s\S]*?)\n\s{2}\}/);
    if (!block) continue;
    const tz = block[1].match(/timezone:\s*'([^']*)'/);
    const fmt = block[1].match(/format:\s*'([^']*)'/);
    if (!tz && !fmt) continue;
    const combo = { timezone: tz ? tz[1] : null, format: fmt ? fmt[1] : null };
    combos.set(JSON.stringify(combo), combo);
  }
  return [...combos.values()];
}

const TRICKY = [
  // RFC 2822 / toString / toUTCString
  'Tue, 05 Mar 2019 15:04:05 GMT',
  'Tue, 05 Mar 2019 15:04:05 +0000',
  'Tue, 5 Mar 2019 15:04:05 -0800',
  'Tue, 05 Mar 2019 15:04:05 EST',
  'Tue, 05 Mar 2019 15:04:05 PDT',
  'Tue Mar 05 2019 15:04:05 GMT-0500 (Eastern Standard Time)',
  'Tue Mar 05 2019 15:04:05 GMT+0530 (India Standard Time)',
  'Tuesday, March 5, 2019 at 3:04 PM EST',
  'Tuesday, March 5, 2019 3:04 PM EST',
  'Tuesday, March 5, 2019, 3:04 PM',
  'March 5, 2019 at 3:04 pm',
  'March 5, 2019 3:04:05.123 PM',
  'March 5, 2019 12:00 AM',
  'March 5, 2019 12:30 PM',
  'March 5, 2019 13:30 PM',
  'March 5, 2019 0:30 AM',
  'Mar 5 2019',
  'Mar 5, 2019',
  '5 Mar 2019',
  '5 March 2019 10:30',
  'Mar. 5, 2019',
  'Sept. 5, 2019',
  'September 5th, 2019',
  'Dec 1st 2016 1:01PM',
  'March 5',
  'March 2019',
  '2019 March 5',
  'junk 5/1/2020',
  'Updated March 5, 2019 3:04 p.m. ET',
  'Posted on: Mar 5, 2019 @ 3:04pm',
  'By Jane Doe | March 5, 2019',
  'Published: 12/25/2019',
  'published 2019-03-05',
  // numeric
  '12/25/2019',
  '25/12/2019',
  '12/25/19',
  '12-25-2019',
  '2019/12/25',
  '2019/12/25 10:30',
  '2019.12.25',
  '25.12.2019',
  '25.12.2019 10:30',
  '12/25',
  '1/2/3',
  '99/1/1',
  '20190305',
  '1551798245',
  '1551798245000',
  '155179824500',
  '15517982450000',
  '0',
  '9',
  '2019',
  '12',
  // ISO variants
  '2019-03-05',
  '2019-03',
  '2019-3-5',
  '2019-03-05T15:04',
  '2019-03-05T15:04:05',
  '2019-03-05T15:04:05.1',
  '2019-03-05T15:04:05.12',
  '2019-03-05T15:04:05.123456789',
  '2019-03-05T15:04:05.123456789Z',
  '2019-03-05T15:04:05Z',
  '2019-03-05T15:04:05z',
  '2019-03-05T15:04:05+05:30',
  '2019-03-05T15:04:05+0530',
  '2019-03-05T15:04:05-08',
  '2019-03-05T15:04-08:00',
  '2019-03-05T24:00:00Z',
  '2019-03-05T24:00:01Z',
  '2019-03-05T25:00:00Z',
  '2019-03-05 15:04:05',
  '2019-03-05 15:04:05Z',
  '2019-03-05 15:04:05 +0100',
  '2019-03-05 15:04:05 UTC',
  '2019-03-05 15:04:05 GMT+2',
  '2019-03-05t15:04:05',
  '2016-11-22T08:57-500',
  '2016 11 22 08:57 -500',
  '+002019-03-05T00:00:00Z',
  '-000001-03-05T00:00:00Z',
  '2019-02-29',
  '2019-02-30T10:00:00',
  '2019-13-01',
  '2019-00-10',
  // times first / am-pm
  '8:30 PM   1/1/2020',
  '8:30PM 1/1/2020',
  '1/1/2020 8:30 a.m.',
  '1/1/2020 8:30 am',
  '1/1/2020 8:30am',
  '    1/1/2020     ',
  '15 Apr 2016 10:59',
  'This page was last modified on 15 April 2016, at 10:59.',
  'November 29, 2016: 8:18 AM ET',
  'Mon Aug 03 12:45:00 EDT 2015',
  '10:30',
  '10:30 pm',
  // relative
  '5 hours ago',
  '1 hour ago',
  '10 months ago',
  '5 days ago',
  '2 weeks ago',
  '3 Years Ago',
  '45 SECONDS AGO',
  '1 minute ago',
  'updated 3 minutes ago',
  'a1 5 hours ago',
  '15 months ago',
  '13 months ago',
  'just now',
  'right now',
  'now',
  'Nowhere',
  'Now 5 things',
  // CJK and other scripts
  '2019年3月5日',
  '2019年03月05日 15時04分',
  '2019年3月5日 15:04',
  '2019年3月5日（火）',
  '2019/03/05（火）15:04',
  '令和元年5月1日',
  '5 mars 2019',
  '5. März 2019',
  '05.03.2019 - 15:04',
  // tz abbreviations / names
  'March 5, 2019 3:04 PM PST',
  'March 5, 2019 3:04 PM CDT',
  'March 5, 2019 3:04 PM MST',
  'March 5, 2019 3:04 PM PT',
  'March 5, 2019 3:04 PM CT',
  'March 5, 2019 3:04 PM ET',
  'March 5, 2019 3:04 PM BST',
  'March 5, 2019 3:04 PM IST',
  'March 5, 2019 3:04 PM UT',
  'March 5, 2019 3:04 PM Z',
  'March 5, 2019 15:04 UTC+0530',
  'March 5, 2019 15:04 GMT-8',
  'March 5, 2019 15:04 GMT+05:30',
  'March 5, 2019 15:04 (local time)',
  'Zeitung: 5.3.2019',
  // DST edges (for timezone options)
  '2019-03-10 02:30:00',
  '2019-11-03 01:30:00',
  '2019-03-31 02:30:00',
  '2019-10-27 02:30:00',
  'March 10, 2019 2:30 am',
  'November 3, 2019 1:30 am',
  // garbage
  'blargh',
  '',
  ' ',
  'T',
  'at',
  'Today',
  'Yesterday at 5:00 PM',
  'Invalid Date',
  'undefined',
  'null',
  '(none)',
  'Jan 5 2020 )',
  '12345678901234567890',
  '1970-01-01T00:00:00Z',
  '0001-01-01T00:00:00Z',
  '9999-12-31T23:59:59.999Z',
  '275760-09-13',
  '+275760-09-13T00:00:00Z',
  '+275760-09-13T00:00:00.001Z',
  'Jan 1 1900',
  'Jan 1 0050',
  '0050-01-01',
  'Genre:Electronic / ExperimentalLabel:Sacred BonesReviewed:June 7, 2019',
  'Last Update Posted  : November 21, 2018',
  '2017-03-13|19h18',
  '2025-June-10T17:11:00+02:00',
  '22:00, 11 July 2021 BST',
  '17 Jan 2025 09:06PM (Updated: 17 Jan 2025 10:04PM)',
  'Dec. 22, 2016 5:00 a.m. PT',
  'Apr 27th, 2020',
  '- 21 Dec 2024',
  '8 July 2025 at 1:40 am',
  '21 December 2024 at 09:31 AM EST.',
  'Oct 9, 2019, 11:05 IST',
  'Mon Aug 03 12:45:00 2015',
  'Aug 03, 2015, 12:45',
  '03 Aug 2015 12:45PM',
  '3 Aug 2015 12:45AM',
  '2015.8.3',
  '2015/8/3',
  '3.8.2015',
  '03.08.2015 - 12:45',
  'August 3, 2015',
  'August 3, 2015 12:45 pm',
  'Aug 3, 2015',
];

// Fixture strings (the log from collect_fixtures.js) are re-evaluated under
// the frozen clock and with every extractor option combination.
const fixtureLog = process.argv[2] ? JSON.parse(fs.readFileSync(process.argv[2], 'utf8')) : [];
const combos = [{ timezone: null, format: null }, ...extractorOptions()];

const seen = new Set();
const corpus = [];
function add(source, input, timezone, format, now) {
  const key = JSON.stringify([input, timezone, format, now]);
  if (seen.has(key)) return;
  seen.add(key);
  frozenNow = now;
  let expected;
  let threw = null;
  try {
    const opts = {};
    if (timezone !== null) opts.timezone = timezone;
    if (format !== null) opts.format = format;
    expected = clean(input, opts);
  } catch (e) {
    expected = null;
    threw = e.message;
  }
  const rec = { source, input, timezone, format, now, expected };
  if (threw) rec.threw = threw;
  corpus.push(rec);
}

for (const rec of fixtureLog) add('fixture', rec.input, rec.timezone, rec.format, NOWS[0]);
const inputs = [...new Set([...fixtureLog.map((r) => r.input), ...TRICKY])];
for (const now of NOWS) {
  for (const input of inputs) {
    for (const c of combos) add('extra', input, c.timezone, c.format, now);
  }
}

// Fuzzed strings, each with no options and one random extractor combination.
const r = rng(1440);
for (const input of dateStrings(7, 2500)) {
  const now = pick(r, NOWS);
  add('fuzz', input, null, null, now);
  const c = pick(r, combos);
  add('fuzz', input, c.timezone, c.format, now);
}

// Random formats with inputs rendered from them by dayjs (sometimes mangled).
const dayjs = require(require.resolve('dayjs', { paths: [MERCURY] }));
const TIMEZONES = [...new Set(combos.map((c) => c.timezone).filter(Boolean))];
for (const { format, render } of formatCases(11, 1500)) {
  const now = pick(r, NOWS);
  const when = Date.UTC(1995, 0, 1) + Math.floor(r() * 40 * 365.25 * 864e5);
  let input = dayjs(new RealDate(when)).format(format);
  if (render < 0.15) input = input.toUpperCase();
  else if (render < 0.3) input = input.slice(0, Math.floor(r() * input.length));
  else if (render < 0.4) input = `Posted ${input} by staff`;
  add('format', input, null, format, now);
  add('format', input, pick(r, TIMEZONES), format, now);
}

process.stderr.write(`${combos.length} option combos, ${inputs.length} inputs, ${corpus.length} cases\n`);
process.stdout.write(`[\n${corpus.map((c) => JSON.stringify(c)).join(',\n')}\n]\n`);
