// JS port of deck_engine/lib.rs, answering byte for byte the same JSON. Used only where
// WebAssembly is blocked; mirror_test.cjs holds it to the wasm build.
function createJsEngine() {
  const PRODUCERS = 3;
  let sim = null;

  function fresh(cap) {
    return {
      cap, claimed: 0, committed: 0, stamps: new Array(cap).fill(null),
      held: new Array(PRODUCERS).fill(null), sent: new Array(PRODUCERS).fill(0),
      written: new Map(), next: 0, received: 0, full: 0, inOrder: true, intact: true,
    };
  }
  const slot = (seq) => seq % sim.cap;
  const published = (seq) => sim.stamps[slot(seq)] === seq;
  function contiguousEnd(from) {
    let end = from;
    for (let i = 0; i < sim.cap && published(end); i++) end++;
    return end;
  }
  const holding = (p) => `{"kind":"holding","p":${p + 1},"seq":${sim.held[p]}}`;
  function full(p) {
    sim.full++;
    return `{"kind":"full","p":${p + 1}}`;
  }
  // ring_claim's headroom check: never a full lap ahead of the consumer.
  function take() {
    if (sim.claimed - sim.committed >= sim.cap) return null;
    return sim.claimed++;
  }

  function claim(p) {
    if (sim.held[p] !== null) return holding(p);
    const seq = take();
    if (seq === null) return full(p);
    sim.held[p] = seq;
    return `{"kind":"claimed","p":${p + 1},"seq":${seq}}`;
  }
  function publish(p) {
    const seq = sim.held[p];
    if (seq === null) return `{"kind":"idle","p":${p + 1}}`;
    sim.held[p] = null;
    const n = ++sim.sent[p];
    sim.written.set(seq, { from: p + 1, n });
    sim.stamps[slot(seq)] = seq;
    return `{"kind":"published","p":${p + 1},"seq":${seq},"n":${n}}`;
  }
  function push(p) {
    if (sim.held[p] !== null) return holding(p);
    const seq = take();
    if (seq === null) return full(p);
    const n = ++sim.sent[p];
    sim.written.set(seq, { from: p + 1, n });
    sim.stamps[slot(seq)] = seq;
    return `{"kind":"pushed","p":${p + 1},"seq":${seq},"n":${n}}`;
  }
  function drain() {
    const start = sim.committed, end = contiguousEnd(start), records = [];
    for (let seq = start; seq < end; seq++) {
      const got = sim.written.get(seq);
      sim.written.delete(seq);
      sim.inOrder = sim.inOrder && seq === sim.next;
      sim.next = seq + 1;
      sim.received++;
      records.push(`{"seq":${seq},"from":${got.from},"n":${got.n}}`);
    }
    sim.committed = end;
    const p = sim.held.indexOf(end);
    return `{"kind":"drained","records":[${records.join(',')}],"next":${end},"blocked_by":${p < 0 ? 'null' : p + 1}}`;
  }

  function state() {
    const s = sim, end = contiguousEnd(s.committed);
    const pub = [...s.written.keys()].sort((a, b) => a - b).map((k) => `{"seq":${k},"from":${s.written.get(k).from},"n":${s.written.get(k).n}}`);
    const nul = (v) => (v === null ? 'null' : String(v));
    return `"ring":{"cap":${s.cap},"claimed":${s.claimed},"committed":${s.committed},"published_through":${end === s.committed ? 'null' : end - 1},`
      + `"available":${end - s.committed},"stamps":[${s.stamps.map(nul).join(',')}]},`
      + `"held":[${s.held.map(nul).join(',')}],"sent":[${s.sent.join(',')}],"published":[${pub.join(',')}],`
      + `"check":{"received":${s.received},"in_order":${s.inOrder},"intact":${s.intact},"full":${s.full}}`;
  }
  const answer = (result) => `{"ok":true,"result":${result},${state()}}`;
  const error = (message) => `{"ok":false,"error":"${message}"}`;

  function run(command) {
    const words = command.split(/\s+/).filter(Boolean);
    if (words[0] === 'reset') {
      if (words.length !== 2 || !/^\+?\d+$/.test(words[1])) return error('usage: reset cap');
      const n = Number(words[1]);
      if (n === 0) return error('ring capacity must be at least 1');
      if (n & (n - 1)) return error(`ring capacity ${n} is not a power of two`);
      sim = fresh(n);
      return answer('{"kind":"reset"}');
    }
    if (!sim) return error('no ring yet: reset first');
    const producer = () => {
      const p = /^\+?\d+$/.test(words[1] || '') ? Number(words[1]) : 0;
      return words.length === 2 && p >= 1 && p <= PRODUCERS ? p - 1 : null;
    };
    const ops = { claim, publish, push };
    if (Object.hasOwn(ops, words[0])) {
      const p = producer();
      return p === null ? error('producer must be 1, 2 or 3') : answer(ops[words[0]](p));
    }
    if (words[0] === 'drain' && words.length === 1) return answer(drain());
    if (words[0] === 'state' && words.length === 1) return answer('{"kind":"state"}');
    return error('unknown command');
  }

  return { kind: 'js', call: (cmd) => JSON.parse(run(cmd)), raw: run };
}
if (typeof module !== 'undefined') module.exports = { createJsEngine };
