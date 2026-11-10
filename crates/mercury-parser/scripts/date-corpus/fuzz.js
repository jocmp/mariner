// Seeded random cases that stress the V8 Date parser and the customParseFormat
// emulation. Exports generators used by build_corpus.js / build_js_date_corpus.js.

function rng(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July',
  'August', 'September', 'October', 'November', 'December'];
const WORDS = ['am', 'pm', 'AM', 'PM', 'a.m.', 'p.m.', 'GMT', 'UTC', 'UT', 'Z', 'z',
  'EST', 'EDT', 'CST', 'CDT', 'MST', 'MDT', 'PST', 'PDT', 'PT', 'ET', 'BST', 'IST',
  'T', 't', 'at', 'on', 'st', 'th', 'Tuesday', 'Tue', 'Mon,', 'Sun', 'ago', 'hours',
  'days', 'published', 'Updated:', '年', '月', '日', '時', '分', 'x', 'junk', 'Mayday'];
const SYMBOLS = ['+', '-', ':', '.', '/', ',', '(', ')', '(comment)', '(a (b) c)',
  ' ', ' ', ' ', '  ', ' ', '\t', '\n', '|', '@', '::', '--', '+05:30', '-0800',
  '+0000', '-500'];

function pick(r, xs) {
  return xs[Math.floor(r() * xs.length)];
}

function randomNumber(r) {
  const len = 1 + Math.floor(r() * (r() < 0.8 ? 4 : 10));
  let s = '';
  for (let i = 0; i < len; i += 1) s += Math.floor(r() * 10);
  return s;
}

function randomMonth(r) {
  const m = pick(r, MONTHS);
  const v = r();
  if (v < 0.4) return m;
  if (v < 0.7) return m.slice(0, 3);
  if (v < 0.8) return `${m.slice(0, 3)}.`;
  if (v < 0.9) return m.toUpperCase();
  return m.toLowerCase();
}

// Loosely date-shaped strings: realistic templates plus token soup.
function dateStrings(seed, count) {
  const r = rng(seed);
  const out = [];
  const pad = (n, w = 2) => String(n).padStart(w, '0');
  for (let i = 0; i < count; i += 1) {
    const kind = r();
    if (kind < 0.35) {
      // token soup
      const n = 1 + Math.floor(r() * 9);
      let s = '';
      for (let j = 0; j < n; j += 1) {
        const k = r();
        if (k < 0.4) s += randomNumber(r);
        else if (k < 0.55) s += randomMonth(r);
        else if (k < 0.7) s += pick(r, WORDS);
        else s += pick(r, SYMBOLS);
        if (r() < 0.5) s += ' ';
      }
      out.push(s);
    } else {
      // perturbed realistic dates
      const y = r() < 0.9 ? 1990 + Math.floor(r() * 45) : Math.floor(r() * 3000);
      const mo = 1 + Math.floor(r() * 13);
      const d = 1 + Math.floor(r() * 32);
      const h = Math.floor(r() * 25);
      const mi = Math.floor(r() * 61);
      const se = Math.floor(r() * 61);
      const ms = Math.floor(r() * 1000);
      const tz = pick(r, ['', 'Z', '+00:00', '-05:00', '+0530', '-08', ' GMT', ' UTC',
        ' EST', ' PDT', ' GMT+0200', ' GMT-8', ' (CET)', ' +0100', ' -500', ' IST']);
      const mon = randomMonth(r);
      const h12 = h % 12 || 12;
      const ap = pick(r, ['am', 'pm', 'AM', 'PM', 'a.m.', 'p.m.', ' am', ' PM']);
      const templates = [
        `${y}-${pad(mo)}-${pad(d)}`,
        `${y}-${pad(mo)}-${pad(d)}T${pad(h)}:${pad(mi)}${tz}`,
        `${y}-${pad(mo)}-${pad(d)}T${pad(h)}:${pad(mi)}:${pad(se)}${tz}`,
        `${y}-${pad(mo)}-${pad(d)}T${pad(h)}:${pad(mi)}:${pad(se)}.${pad(ms, 3)}${tz}`,
        `${y}-${pad(mo)}-${pad(d)} ${pad(h)}:${pad(mi)}:${pad(se)}${tz}`,
        `${y}-${mo}-${d} ${h}:${mi}`,
        `${y}/${mo}/${d}`,
        `${y}/${pad(mo)}/${pad(d)} ${h}:${pad(mi)}`,
        `${mo}/${d}/${y}`,
        `${mo}/${d}/${String(y).slice(-2)}`,
        `${mo}-${d}-${y} ${h12}:${pad(mi)} ${ap}`,
        `${d}.${mo}.${y}`,
        `${pad(d)}.${pad(mo)}.${y} - ${pad(h)}:${pad(mi)}`,
        `${mon} ${d}, ${y}`,
        `${mon} ${d}, ${y} ${h12}:${pad(mi)} ${ap}${tz}`,
        `${mon} ${d}, ${y} at ${h12}:${pad(mi)}${ap}`,
        `${d} ${mon} ${y} ${pad(h)}:${pad(mi)}${tz}`,
        `${pick(r, ['Mon', 'Tuesday', 'Wed,', 'Thu.'])} ${mon} ${d} ${y} ${pad(h)}:${pad(mi)}:${pad(se)}${tz}`,
        `${pick(r, ['Mon', 'Tue', 'Wed'])}, ${d} ${mon} ${y} ${pad(h)}:${pad(mi)}:${pad(se)}${tz}`,
        `${y}年${mo}月${d}日`,
        `${y}年${pad(mo)}月${pad(d)}日 ${pad(h)}時${pad(mi)}分`,
        `${y}年${mo}月${d}日 ${pad(h)}:${pad(mi)}`,
        `${y}.${mo}.${d}`,
        `${pick(r, ['Published', 'Updated:', 'By Jane Doe |', 'Posted on', '-'])} ${mon} ${d}, ${y}`,
        `${h12}:${pad(mi)}${ap} ${mo}/${d}/${y}`,
        `${Math.floor(r() * 40)} ${pick(r, ['seconds', 'minute', 'hours', 'Days', 'weeks', 'month', 'years'])} ago`,
        `${d} ${mon}`,
        `${mon} ${y}`,
        `${pad(d)} ${mon} ${y} ${pad(h12)}:${pad(mi)}${ap.trim().toUpperCase()}`,
      ];
      out.push(pick(r, templates));
    }
  }
  return out;
}

const FORMAT_TOKENS = ['YYYY', 'YY', 'M', 'MM', 'MMM', 'MMMM', 'D', 'DD', 'Do', 'H', 'HH',
  'h', 'hh', 'm', 'mm', 's', 'ss', 'S', 'SS', 'SSS', 'A', 'a', 'Q', 'Y', 'ddd', 'dddd',
  'zz', 'Z', '年', '月', '日', '時', '分', 'at', '[at]', '[T]'];
const FORMAT_SEPARATORS = [' ', ', ', '-', '/', '.', ':', ' - ', '', 'T', ' at '];

// Random formats paired with inputs rendered from them (and perturbed).
function formatCases(seed, count) {
  const r = rng(seed);
  const cases = [];
  for (let i = 0; i < count; i += 1) {
    const n = 1 + Math.floor(r() * 6);
    let fmt = '';
    for (let j = 0; j < n; j += 1) {
      fmt += pick(r, FORMAT_TOKENS);
      if (j < n - 1) fmt += pick(r, FORMAT_SEPARATORS);
    }
    cases.push({ format: fmt, render: r() });
  }
  return cases;
}

module.exports = { dateStrings, formatCases, rng, pick };
