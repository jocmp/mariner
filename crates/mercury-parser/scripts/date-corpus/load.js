// Loads a patched copy of mercury-parser's dist bundle that records every
// cleanDatePublished call and exposes the original function.
const fs = require('fs');
const path = require('path');
const Module = require('module');

const MERCURY =
  process.env.MERCURY_PARSER_DIR || path.resolve(__dirname, '..', '..', '..', '..', '..', 'mercury-parser');

function loadMercury() {
  const src = fs.readFileSync(path.join(MERCURY, 'dist/mercury.js'), 'utf8');
  const marker = 'function cleanDatePublished(';
  if (!src.includes(marker)) throw new Error('marker not found');
  const patched = src.replace(
    marker,
    `function cleanDatePublished(dateString, opts) {
  const r = cleanDatePublished__orig(dateString, opts);
  if (globalThis.__dateLog) {
    const o = opts || {};
    globalThis.__dateLog.push({
      input: dateString,
      timezone: o.timezone === undefined ? null : o.timezone,
      format: o.format === undefined ? null : o.format,
      expected: r === undefined ? null : r,
    });
  }
  return r;
}
globalThis.__cleanDatePublished = cleanDatePublished__orig;
globalThis.__cleanDateString = cleanDateString;
function cleanDatePublished__orig(`
  );
  const filename = path.join(MERCURY, 'dist/mercury.patched.js');
  const m = new Module(filename, module);
  m.filename = filename;
  m.paths = Module._nodeModulePaths(path.join(MERCURY, 'dist'));
  m._compile(patched, filename);
  return m.exports;
}

module.exports = { loadMercury, MERCURY };
