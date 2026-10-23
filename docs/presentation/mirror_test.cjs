// Random sessions through the wasm build and src/mirror.js; exits 1 on the first answer that differs.
// Usage: node mirror_test.cjs [sessions]   (after build.py has built the wasm)
const { readFileSync } = require('node:fs');
const path = require('node:path');
const { createJsEngine } = require('./src/mirror.js');
const bytes = readFileSync(path.join(__dirname, 'deck_engine/target/wasm32-unknown-unknown/release/deck_engine.wasm'));
const N = Number(process.argv[2] || 2000);

function wasmEngine() {
  const { exports: ex } = new WebAssembly.Instance(new WebAssembly.Module(bytes), {});
  const enc = new TextEncoder(), dec = new TextDecoder();
  return { raw(cmd) {
    const b = enc.encode(cmd); const p = ex.io_input(b.length);
    new Uint8Array(ex.memory.buffer, p, b.length).set(b);
    const n = ex.io_run();
    return dec.decode(new Uint8Array(ex.memory.buffer, ex.io_output(), n));
  } };
}
let seed = 12345;
const rnd = (n) => { seed = (seed + 0x6D2B79F5) | 0; let t = Math.imul(seed ^ (seed >>> 15), 1 | seed); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return (((t ^ (t >>> 14)) >>> 0) % n); };
const pick = (a) => a[rnd(a.length)];
const tally = {};
let cmds = 0, laps = 0;
for (let run = 0; run < N; run++) {
  const w = wasmEngine(), j = createJsEngine();
  const log = [rnd(40) ? `reset ${pick([2, 4, 8, 8, 8, 16])}` : pick(['reset 0', 'reset 6', 'reset', 'reset 8 1', 'reset x', 'drain'])];
  if (!log[0].match(/^reset (2|4|8|16)$/)) log.push('reset 8');
  const len = 10 + rnd(120);
  for (let i = 0; i < len; i++) {
    const r = rnd(100);
    const p = rnd(30) ? String(1 + rnd(3)) : pick(['0', '4', '+2', '01', 'x', '']);
    if (r < 22) log.push(`claim ${p}`.trim());
    else if (r < 44) log.push(`publish ${p}`.trim());
    else if (r < 72) log.push(`push ${p}`.trim());
    else if (r < 96) log.push('drain');
    else log.push(pick(['state', 'drain 1', 'claim 1 2', 'nope', '']));
  }
  for (const cmd of log) {
    cmds++;
    const a = w.raw(cmd), b = j.raw(cmd);
    if (a !== b) {
      console.log('MISMATCH run', run, 'cmd', JSON.stringify(cmd)); console.log(log.slice(0, log.indexOf(cmd) + 1).join('\n'));
      console.log('wasm', a); console.log('js  ', b); process.exit(1);
    }
    const o = JSON.parse(a);
    if (o.ok && !(o.check.in_order && o.check.intact)) { console.log('CHECK FAILED', cmd, a); process.exit(1); }
    const k = o.ok ? o.result.kind + (o.result.kind === 'drained' ? (o.result.records.length ? '+records' : '+empty') + (o.result.blocked_by ? '+blocked' : '') : '') : 'error';
    tally[k] = (tally[k] || 0) + 1;
    if (o.ok) laps = Math.max(laps, Math.floor(o.ring.claimed / o.ring.cap));
  }
}
console.log(`ok: ${N} sessions, ${cmds} commands identical, up to ${laps} laps of one ring`); console.log(tally);
