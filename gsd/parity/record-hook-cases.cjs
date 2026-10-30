#!/usr/bin/env node
'use strict';
// Records the calls an upstream hook test suite makes to its hook script, with the answers upstream's hook gave:
//
//   node gsd/parity/record-hook-cases.cjs <gsd-core checkout at v1.18.0> <hook file name> <suite file> <out.json>
//
// The checkout must have its dependencies installed (the suites use fast-check). The recording is what
// `gsd/core/src/hooks/**/tests.rs` replays through the Rust hook: the case list is upstream's, not retyped.
// The environment is not recorded except for GSD_* variables.
const fs = require('fs');
const path = require('path');

if (!process.env.REC_OUT) {
  const [dir, hook, suite, out] = process.argv.slice(2);
  if (!out) {
    console.error('usage: record-hook-cases.cjs <checkout> <hook file name> <suite file> <out.json>');
    process.exit(2);
  }
  const r = require('child_process').spawnSync(
    process.execPath,
    ['--require', __filename, path.join(dir, 'tests', suite)],
    { cwd: dir, env: { ...process.env, REC_OUT: path.resolve(out), REC_HOOK: hook }, stdio: 'inherit' },
  );
  process.exit(r.status === null ? 1 : r.status);
}

const cp = require('child_process');
const nt = require('node:test');
const stack = [];
let current = null;
let currentDescribe = [];
const calls = [];
const shorten = (v) => {
  if (typeof v === 'string') return v.length > 160 ? `${v.slice(0, 160)}…` : v;
  if (Array.isArray(v)) return v.map(shorten);
  if (v && typeof v === 'object') return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, shorten(x)]));
  return v;
};
const answer = (text) => {
  try {
    const v = JSON.parse(text);
    // A block's `reason` is prose; the typed fields (`decision`, `code`, `path`, `tool`) are the answer.
    if (v && typeof v === 'object' && v.decision === 'block') delete v.reason;
    return shorten(v);
  } catch { return text.length > 400 ? `${text.slice(0, 400)}…` : text; }
};
const spawnSync = cp.spawnSync;
cp.spawnSync = function (cmd, args, opts) {
  const r = spawnSync.apply(this, arguments);
  const argv = [cmd, ...(Array.isArray(args) ? args : [])].map(String);
  if (argv.some((a) => a.includes(process.env.REC_HOOK))) {
    const env = opts && opts.env
      ? Object.fromEntries(Object.entries(opts.env).filter(([k]) => /^GSD_/.test(k)))
      : {};
    let size = null;
    const input = opts && opts.input !== undefined ? String(opts.input) : null;
    if (input !== null && input.length > 2000) size = input.length;
    calls.push({
      describe: currentDescribe, test: current, env,
      input: size === null ? input : null, input_len: size,
      status: r.status, stdout: answer(r.stdout ? String(r.stdout) : ''),
    });
  }
  return r;
};
for (const name of ['describe', 'suite', 'test', 'it']) {
  const orig = nt[name];
  if (typeof orig !== 'function') continue;
  const wrapped = function (title, ...rest) {
    const i = rest.findIndex((x) => typeof x === 'function');
    if (typeof title === 'string' && i >= 0) {
      const f = rest[i];
      // A suite body runs while it registers its tests; a test body runs later, so the describe titles are taken now.
      const registered = [...stack];
      rest[i] = function (...a) {
        if (name === 'describe' || name === 'suite') {
          stack.push(title);
          try { return f.apply(this, a); } finally { stack.pop(); }
        }
        const prev = [current, currentDescribe];
        current = title;
        currentDescribe = registered;
        try { return f.apply(this, a); } finally { [current, currentDescribe] = prev; }
      };
    }
    return orig.call(this, title, ...rest);
  };
  Object.assign(wrapped, orig);
  nt[name] = wrapped;
}
process.on('exit', () => fs.writeFileSync(process.env.REC_OUT, `${JSON.stringify(calls)}\n`));
