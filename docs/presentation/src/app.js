(() => {
'use strict';

const NAMES = { 1: 'Оксана', 2: 'Богдан', 3: 'Ярина' };
const INITIAL = { 1: 'О', 2: 'Б', 3: 'Я' };
const FEMININE = { 1: true, 2: false, 3: true };
const took = (p) => (FEMININE[p] ? 'взяла' : 'взяв');
const stamped = (p) => (FEMININE[p] ? 'поставила' : 'поставив');
const sent = (p) => (FEMININE[p] ? 'надіслала' : 'надіслав');
const person = (p) => `<span class="person" data-t="${p}">${NAMES[p]}</span>`;
const reader = '<span class="person" data-t="0">Читач</span>';
const seqList = (records) => records.map((r) => `#${r.seq}`).join(' · ');
const records = (n) => `${n} ${n % 10 === 1 && n % 100 !== 11 ? 'запис' : [2, 3, 4].includes(n % 10) && ![12, 13, 14].includes(n % 100) ? 'записи' : 'записів'}`;

/* ---------- engine: Rust via WebAssembly, JS mirror as fallback ---------- */
let wasmModule = null;
async function bootEngine() {
  try {
    const bin = atob(document.getElementById('wasm-b64').textContent.trim());
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    wasmModule = await WebAssembly.compile(bytes);
    const probe = await newEngine();
    if (!probe.call('reset 8').ok) throw new Error('probe');
  } catch (e) {
    wasmModule = null;
  }
  const badge = document.getElementById('engine-badge');
  if (wasmModule) {
    badge.textContent = 'Rust → WebAssembly';
    badge.classList.add('wasm');
    badge.title = 'Кільця рахує ring_mpsc, скомпільований у wasm32 і вбудований у сторінку';
  } else {
    badge.textContent = 'JS-дзеркало кільця';
    badge.title = 'WebAssembly недоступний у цьому вікні. Працює JS-копія правил, звірена з Rust на 200 тис. команд. Відкрийте HTML-файл локально, щоб запустити Rust.';
  }
}
async function newEngine() {
  if (!wasmModule) return createJsEngine();
  const inst = await WebAssembly.instantiate(wasmModule, {});
  const ex = inst.exports, enc = new TextEncoder(), dec = new TextDecoder();
  let dead = false;
  return {
    kind: 'wasm',
    call(cmd) {
      if (dead) return { ok: false, error: 'кільце зупинилося' };
      try {
        const b = enc.encode(cmd);
        const p = ex.io_input(b.length);
        new Uint8Array(ex.memory.buffer, p, b.length).set(b);
        const n = ex.io_run();
        return JSON.parse(dec.decode(new Uint8Array(ex.memory.buffer, ex.io_output(), n)));
      } catch (e) {
        dead = true;
        return { ok: false, error: 'кільце зупинилося' };
      }
    },
  };
}

/* ---------- the ring ---------- */
// Which sequence occupies each slot right now, and in what state: claimed (a producer holds the
// guard) or published (stamped, waiting for the reader). Everything else is free.
function slotsOf(st) {
  const { cap, claimed, committed } = st.ring;
  const pub = new Map(st.published.map((l) => [l.seq, l]));
  const out = Array.from({ length: cap }, (_, i) => ({ i, kind: 'free' }));
  for (let seq = committed; seq < claimed; seq++) {
    const i = seq % cap, p = st.held.indexOf(seq);
    if (p >= 0) out[i] = { i, kind: 'claimed', seq, t: p + 1 };
    else if (pub.has(seq)) out[i] = { i, kind: 'pub', seq, t: pub.get(seq).from, n: pub.get(seq).n };
  }
  return out;
}

const CX = 180, R1 = 140, R0 = 88;
const f = (n) => n.toFixed(2);
const pt = (r, a) => [CX + r * Math.cos(a), CX + r * Math.sin(a)];
function sector(r0, r1, a0, a1) {
  const [x0, y0] = pt(r1, a0), [x1, y1] = pt(r1, a1), [x2, y2] = pt(r0, a1), [x3, y3] = pt(r0, a0);
  const lg = a1 - a0 > Math.PI ? 1 : 0;
  return `M${f(x0)} ${f(y0)}A${r1} ${r1} 0 ${lg} 1 ${f(x1)} ${f(y1)}L${f(x2)} ${f(y2)}A${r0} ${r0} 0 ${lg} 0 ${f(x3)} ${f(y3)}Z`;
}
function tri(rTip, rBase, a, half) {
  const [tx, ty] = pt(rTip, a), [bx, by] = pt(rBase, a), nx = -Math.sin(a), ny = Math.cos(a);
  return `M${f(tx)} ${f(ty)}L${f(bx + nx * half)} ${f(by + ny * half)}L${f(bx - nx * half)} ${f(by - ny * half)}Z`;
}
const label = (cls, [x, y], text) => `<text class="${cls}" x="${f(x)}" y="${f(y)}" text-anchor="middle" dominant-baseline="middle">${text}</text>`;

function ringSvg(st, prev) {
  const { cap, claimed, committed } = st.ring;
  const step = 2 * Math.PI / cap, gap = Math.min(0.05, step * 0.08);
  const cur = slotsOf(st), before = prev ? slotsOf(prev) : null;
  let g = '';
  for (const s of cur) {
    const a0 = -Math.PI / 2 + s.i * step + gap / 2, a1 = a0 + step - gap, am = (a0 + a1) / 2;
    const b = before && before[s.i];
    const changed = b && (b.kind !== s.kind || b.seq !== s.seq) && s.kind !== 'free';
    const title = s.kind === 'free' ? `комірка ${s.i}: вільна`
      : `комірка ${s.i}: квиток #${s.seq}, ${NAMES[s.t]}, ${s.kind === 'claimed' ? 'ще пише' : 'штамп стоїть, чекає читача'}`;
    g += `<path class="rs ${s.kind}${changed ? ' flash' : ''}"${s.t ? ` data-t="${s.t}"` : ''} d="${sector(R0, R1, a0, a1)}"><title>${title}</title></path>`;
    g += label('t-idx', pt(R1 + 11, am), s.i);
    if (s.kind !== 'free') {
      const [x, y] = pt((R0 + R1) / 2, am);
      g += label('t-seq', [x, y - 6], `#${s.seq}`);
      g += label('t-own', [x, y + 14], `${INITIAL[s.t]} ${s.kind === 'claimed' ? 'пише' : '✓'}`);
    }
  }
  const mid = (seq) => -Math.PI / 2 + ((seq % cap) + 0.5) * step;
  const inFlight = claimed - committed, full = inFlight >= cap;
  g += `<path class="mk-claim${full ? ' full' : ''}" d="${tri(R1 + 20, R1 + 36, mid(claimed), 9)}"><title>наступний квиток: #${claimed}</title></path>`;
  g += `<path class="mk-cons" d="${tri(R0 - 3, R0 - 19, mid(committed), 9)}"><title>читач чекає на #${committed}</title></path>`;
  g += `<circle class="hub" cx="${CX}" cy="${CX}" r="62"/>`;
  const blocker = st.held.indexOf(committed);
  const status = full ? ['bad', 'повне: Full']
    : blocker >= 0 ? ['bad', `чекає на #${committed}`]
    : st.ring.available ? ['ok', `готово: ${st.ring.available}`]
    : inFlight === 0 ? ['idle', 'порожньо'] : ['ok', 'можна брати'];
  g += `<text class="t-c1" x="${CX}" y="${CX - 24}" text-anchor="middle">у польоті</text>`;
  g += `<text class="t-c2" x="${CX}" y="${CX + 10}" text-anchor="middle">${inFlight}/${cap}</text>`;
  g += `<text class="t-c3 ${status[0]}" x="${CX}" y="${CX + 32}" text-anchor="middle">${status[1]}</text>`;
  return `<svg class="ring" viewBox="0 0 360 360" role="img" aria-label="Кільце: квитки #${committed}–#${claimed - 1} у польоті">${g}</svg>`;
}

const LEGEND = '<div class="legend"><span><i class="sw"></i>вільна</span><span><i class="sw claimed"></i>пише</span><span><i class="sw pub"></i>штамп стоїть</span><span><i class="tri claim"></i>наступний квиток</span><span><i class="tri cons"></i>читач</span></div>';
function renderRing(el, st, { prev = null, head = '', note = '', legend = true, still = false } = {}) {
  const top = head ? `<div class="ringbox-head"><b>${head}</b><span>${note}</span></div>` : '';
  el.innerHTML = `${top}<div class="${still ? 'still' : ''}">${ringSvg(st, still ? null : prev)}</div>${legend ? LEGEND : ''}`;
}

/* ---------- static rings: the title and the storyboard ---------- */
const scenes = {};
async function buildStatic() {
  let e = await newEngine();
  for (const c of ['reset 8', 'push 2', 'push 3', 'drain', 'claim 1', 'push 2', 'push 3', 'claim 3', 'push 2']) scenes.title = e.call(c);

  e = await newEngine();
  e.call('reset 8');
  const s1 = [1, 2, 3].map((p) => e.call(`claim ${p}`));
  scenes.s1 = s1[2];
  const s2 = [e.call('publish 2'), e.call('publish 3')];
  const wait = e.call('drain');
  scenes.s2 = wait;
  const s3 = e.call('publish 1');
  const read = e.call('drain');
  scenes.s3 = read;

  const li = (cls, html) => `<li${cls ? ` class="${cls}"` : ''}>${html}</li>`;
  document.getElementById('s1-evs').innerHTML = s1.map((a) => li('', `${person(a.result.p)} ${took(a.result.p)} <span class="mono">#${a.result.seq}</span>`)).join('')
    + li('', 'Кожен квиток — один CAS');
  const blocker = wait.result.blocked_by;
  document.getElementById('s2-evs').innerHTML = s2.map((a) => li('', `${person(a.result.p)} ${stamped(a.result.p)} штамп на <span class="mono">#${a.result.seq}</span>`)).join('')
    + li('wait', `${reader} чекає на <span class="mono">#${wait.result.next}</span>: ${NAMES[blocker]} ще пише, хоча <span class="mono">#1</span> і <span class="mono">#2</span> готові`);
  document.getElementById('s3-evs').innerHTML = li('', `${person(s3.result.p)} ${stamped(s3.result.p)} штамп на <span class="mono">#${s3.result.seq}</span>`)
    + li('read', `${reader} забрав <span class="mono">${seqList(read.result.records)}</span> строго за номерами`)
    + li('', `Комірки знову вільні, наступний квиток <span class="mono">#${read.ring.claimed}</span>`);
}
function renderStatic() {
  if (!scenes.title) return;
  const rings = [
    ['title-ring', scenes.title, { head: 'ring_mpsc · 8 комірок', note: 'порахував справжній ring_mpsc' }],
    ['s1-ring', scenes.s1, { legend: false }],
    ['s2-ring', scenes.s2, { legend: false }],
    ['s3-ring', scenes.s3, { legend: false }],
  ];
  for (const [id, st, opt] of rings) {
    const el = document.getElementById(id);
    if (el.offsetParent !== null) renderRing(el, st, { still: true, ...opt });
  }
}

/* ---------- sandbox ---------- */
const SB_SETUP = ['reset 8', 'push 1', 'push 2', 'drain', 'claim 1', 'push 2', 'push 3'];
const sb = {
  engine: null, cur: null, prev: null, tape: [], got: [], el: {},
  async reset() {
    this.engine = await newEngine();
    this.tape = [];
    this.got = [];
    for (const c of SB_SETUP) this.exec(c);
    this.prev = this.cur;
    this.card('Пісочниця', `${person(1)} тримає квиток <span class="mono">#2</span>, Богдан і Ярина вже надіслали <span class="mono">#3</span> і <span class="mono">#4</span>`, []);
    this.render(true);
  },
  exec(cmd) {
    const ans = this.engine.call(cmd);
    if (!ans.ok) return ans;
    this.prev = this.cur || ans;
    this.cur = ans;
    const [cls, line] = this.line(ans.result);
    if (line) this.tape.unshift(`<li class="${cls}">${line}</li>`);
    this.tape.length = Math.min(this.tape.length, 40);
    if (ans.result.kind === 'drained') this.got.push(...ans.result.records);
    return ans;
  },
  line(r) {
    const m = (s) => `<span class="mono">#${s}</span>`;
    switch (r.kind) {
      case 'claimed': return ['', `${person(r.p)} ${took(r.p)} квиток ${m(r.seq)}`];
      case 'published': return ['', `${person(r.p)} ${stamped(r.p)} штамп на ${m(r.seq)}, лист ${r.n}`];
      case 'pushed': return ['', `${person(r.p)} ${sent(r.p)} ${m(r.seq)}, лист ${r.n}`];
      case 'full': return ['bad', `${person(r.p)}: Full, кільце повне`];
      case 'drained':
        if (r.records.length) return ['read', `${reader} забрав ${r.records.map((x) => m(x.seq)).join(' · ')}`];
        return r.blocked_by ? ['bad', `${reader} чекає на ${m(r.next)}: ${NAMES[r.blocked_by]} ще пише`] : ['', `${reader}: забирати нічого`];
      default: return ['', ''];
    }
  },
  badges(r) {
    switch (r.kind) {
      case 'claimed': return [['ok', `Квиток #${r.seq} · комірка ${r.seq % this.cur.ring.cap}`]];
      case 'holding': return [['bad', `Уже тримає квиток #${r.seq}: спершу поставте штамп`]];
      case 'full': return [['bad', 'Full: кільце повне'], ['mid', 'Непрочитане не затерто']];
      case 'published': return [['ok', `Штамп на #${r.seq} · лист ${r.n}`]];
      case 'idle': return [['mid', 'Квитка немає: спершу візьміть його']];
      case 'pushed': return [['ok', `Надіслано #${r.seq} · лист ${r.n}`]];
      case 'drained': {
        const out = [];
        if (r.records.length) out.push(['ok', `Забрано ${seqList(r.records)}`]);
        if (r.blocked_by) out.push(['bad', `Чекає на #${r.next}: ${NAMES[r.blocked_by]} ще пише`]);
        if (!out.length) out.push(['mid', 'Нічого готового немає']);
        return out;
      }
      default: return [];
    }
  },
  card(tag, body, badges) {
    const el = this.el.inc;
    el.innerHTML = `<div class="inc-head"><span class="tag">${tag}</span></div><div class="inc-order">${body}</div>${badges.length ? `<div class="outcome">${badges.map(([c, t]) => `<span class="badge ${c}">${t}</span>`).join('')}</div>` : ''}`;
    el.classList.remove('flash');
    void el.offsetWidth;
    el.classList.add('flash');
  },
  act(cmd, tag, body) {
    const ans = this.exec(cmd);
    this.card(tag, body, ans.ok ? this.badges(ans.result) : [['bad', ans.error]]);
    this.render();
  },
  fill() {
    const before = this.cur.check.full;
    let pushed = 0;
    for (let k = 0; k < this.cur.ring.cap * 2 && this.cur.check.full === before; k++) {
      const p = (k % 3) + 1;
      if (this.cur.held[p - 1] !== null) continue;
      if (this.exec(`push ${p}`).result.kind === 'pushed') pushed++;
    }
    const refused = this.cur.check.full > before;
    this.card('Усі виробники', `Надсилають, доки кільце не відмовить`, [['ok', `Надіслано ${pushed}`], ...(refused ? [['bad', 'Наступний отримав Full'], ['mid', 'Непрочитане не затерто']] : [])]);
    this.render();
  },
  init() {
    const $ = (id) => document.getElementById(id);
    this.el = { prods: $('sb-prods'), inc: $('sb-inc'), ring: $('sb-ring'), got: $('sb-got'), count: $('sb-count'), avail: $('sb-avail'), tape: $('sb-tape'), foot: $('sb-foot') };
    this.el.prods.addEventListener('click', (e) => {
      const b = e.target.closest('button[data-act]');
      if (!b) return;
      const p = Number(b.dataset.p), act = b.dataset.act;
      const body = { claim: `${person(p)} бере квиток`, publish: `${person(p)} дописує лист і ставить штамп`, push: `${person(p)} надсилає лист одразу` }[act];
      this.act(`${act} ${p}`, `${NAMES[p]} · ${act}`, body);
    });
    $('sb-drain').addEventListener('click', () => this.act('drain', 'Читач · drain', `${reader} забирає все готове підряд`));
    $('sb-fill').addEventListener('click', () => this.fill());
    $('sb-reset').addEventListener('click', () => this.reset());
  },
  render(still = false) {
    const st = this.cur;
    if (!st) return;
    this.el.prods.innerHTML = [1, 2, 3].map((p) => {
      const held = st.held[p - 1];
      const state = held === null ? 'нічого не тримає' : `тримає квиток #${held} · комірка ${held % st.ring.cap}`;
      const acts = held === null
        ? `<button class="btn" type="button" data-act="claim" data-p="${p}">Взяти квиток</button><button class="btn" type="button" data-act="push" data-p="${p}">Надіслати</button>`
        : `<button class="btn primary" type="button" data-act="publish" data-p="${p}">Поставити штамп</button>`;
      return `<div class="prod"><div class="prod-head">${person(p)}<span class="st">${state} · листів: ${st.sent[p - 1]}</span></div><div class="prod-acts">${acts}</div></div>`;
    }).join('');
    renderRing(this.el.ring, st, { prev: this.prev, still, head: 'ring_mpsc · 8 комірок', note: st.ring.claimed > st.ring.committed ? `квитки #${st.ring.committed}…#${st.ring.claimed - 1} у польоті` : 'нічого в польоті' });
    this.el.avail.textContent = `готово підряд: ${st.ring.available}`;
    this.el.count.textContent = `${st.check.received} шт.`;
    this.el.got.innerHTML = this.got.length
      ? this.got.slice(-30).map((r) => `<span class="lt" data-t="${r.from}" title="квиток #${r.seq}: ${NAMES[r.from]}, лист ${r.n}">#${r.seq} ${INITIAL[r.from]}${r.n}</span>`).join('')
      : '<span class="empty">Ще нічого не забрано</span>';
    this.el.tape.innerHTML = this.tape.length ? this.tape.join('') : '<li class="empty">Подій ще немає</li>';
    const c = st.check;
    const order = c.in_order ? `<span class="ok">✓ ${records(c.received)}: кожен рівно раз, у порядку квитків</span>` : '<span class="bad">✗ порядок порушено</span>';
    const intact = c.intact ? '<span class="ok">✓ вміст цілий</span>' : '<span class="bad">✗ вміст пошкоджено</span>';
    this.el.foot.innerHTML = `${order}${intact}<span>відмов Full: ${c.full}</span>`;
  },
};

/* ---------- speaker notes and the full script ---------- */
const NOTES = {
  title: { t: '0:00', text: 'Добрий день! Наш проєкт — ring, lock-free шлях запису на Rust. За чотири хвилини покажу, яку проблему він розв’язує, як працює і що ми зробили, щоб його можна було брати в продакшен.' },
  problem: { t: '0:15', text: 'Уявіть: сотні потоків одночасно надсилають записи. Це заявки на біржу, телеметрія чи рядки журналу. А обробляє їх один потік, строго по черзі. Найпростіше рішення — черга під замком. Але тоді, поки Оксана пише, Богдан і Ярина стоять і чекають на ключ. І черга мусить відповісти на чотири питання. Хто перший? Чи є вільне місце? Чи запис уже цілий? І що буде, якщо потік із замком засне?' },
  how: { t: '0:45', text: 'Ring обходиться без замка. Комірок вісім, і вони йдуть по колу. Спершу кожен бере квиток, і це одна атомарна операція. Оксана взяла нульовий, Богдан перший, Ярина другий. Пишуть вони хто як встигне. Богдан і Ярина вже поставили штамп, а Оксана ще пише. Читач чекає саме на нульовий квиток, тому стоїть, хоча перший і другий готові. Щойно Оксана поставила штамп, читач забирає всі три, строго за номерами, і звільняє комірки. Ці кільця на слайді порахував справжній ring_mpsc, вбудований у сторінку.',
    alt: 'Якщо спитають, що таке CAS: compare-and-swap, атомарна інструкція процесора «заміни, якщо значення досі те, що я бачив».' },
  why: { t: '1:25', text: 'Навіщо без замка? У липні 1997 року Mars Pathfinder на Марсі раз у раз перезавантажувався. Низькопріоритетна задача тримала м’ютекс, її витісняла задача середнього пріоритету, а найважливіша задача чекала на той самий м’ютекс, не встигала у свій цикл, і система перезавантажувалась. Праворуч наш замір. З одним виробником м’ютекс і кільце йдуть поруч. З одинадцятьма м’ютекс падає з 276 до 37 мільйонів записів за секунду, а кільце тримає 203. І наше правило: механізм потрапляє в код, лише коли виграв замір.',
    alt: 'Pathfinder виправили, увімкнувши успадкування пріоритету для того м’ютекса; латку завантажили на зонд дистанційно. Замір — readme.md, розділ Measured results, i7-10750H, пачки по 32 записи з обох боків.' },
  promises: { t: '2:05', text: 'Звідси чотири гарантії. По черзі: кожен запис доходить рівно один раз, у порядку квитків. Не затре: якщо кільце повне, виробник отримує відмову, а непрочитаного ніхто не торкається. Цілком: штамп ставиться після запису, тож половини запису читач не побачить. І ніхто не тримає замок: квиток — один CAS, штамп — один запис у пам’ять.' },
  hardware: { t: '2:35', text: 'Тепер про залізо. Кожен курсор лежить на своїй кеш-лінії, щоб ядра не відбирали її одне в одного. Виробник тримає копію курсора читача і перечитує справжній, лише коли копія каже «повно». Штамп — це сам номер квитка, тож його не треба скидати. А пачку з тридцяти двох записів ми беремо одним CAS. У підсумку SPSC-кільце на 83% швидше за rtrb. Чесно скажу: без пачок ArrayQueue з crossbeam поки швидший, і це наша наступна ціль.',
    alt: 'Цифри rtrb і ArrayQueue — з того самого прогону, що й наші; абсолютні значення між машинами не переносяться.' },
  checks: { t: '3:05', text: 'Готовим до продакшену кільце роблять не цифри, а перевірки, які вміють падати. За capstone ми підключили до CI вісім процесів, хоча до того не було жодного. loom перебирає всі переплетення потоків. Фазинг звіряє кільце з простою чергою. Мутаційні гейти ламають код навмисно і не рахують тест, який цього не помітив. А кожен PR у гарячий шлях перевіряє, що кільце досі б’є м’ютекс.',
    alt: 'Якщо спитають про unsafe: він дозволений лише в ring_spsc, ring_mpsc і ring_store, і кожен такий крейт пояснює це у своєму docs/workaround/readme.md. Гейт G6 падає, якщо unsafe з’явиться деінде.' },
  sandbox: { t: '3:35', text: 'І наостанок: це не слайди. Тут працює справжній ring_mpsc, скомпільований у WebAssembly. Оксана тримає квиток, тому читач чекає. Ставимо штамп, і читач забирає все по черзі. А якщо заповнити кільце, наступний запис отримає відмову, а не затре непрочитане. Дякуємо! Будемо раді питанням.',
    alt: 'Сценарій: «Забрати готове» — нічого, читач чекає на Оксану; в Оксани «Поставити штамп»; знову «Забрати готове» — #2 · #3 · #4; «Заповнити до відмови» — Full. Якщо на значку внизу написано «JS-дзеркало кільця», скажіть: тут працює копія правил, звірена з Rust на 200 тисячах команд.' },
};

/* ---------- deck ---------- */
const slides = [...document.querySelectorAll('.slide')];
let cur = 0;
const notesEl = document.getElementById('notes');
const notesBtn = document.getElementById('notes-btn');
const sectionsEl = document.getElementById('sections');
sectionsEl.innerHTML = slides.map((s, i) => `<button type="button" data-i="${i}"><span>${s.dataset.label}</span></button>`).join('');
sectionsEl.addEventListener('click', (e) => { const b = e.target.closest('button'); if (b) show(Number(b.dataset.i)); });

function renderNotes() {
  const id = slides[cur].dataset.id, n = NOTES[id];
  notesEl.innerHTML = n ? `<div class="nt"><div class="nt-head"><span>${slides[cur].dataset.label} · з ${n.t}</span><span>N — сховати</span></div><p>${n.text}</p>${n.alt ? `<p class="alt">${n.alt}</p>` : ''}</div>` : '';
}
function show(i) {
  cur = Math.max(0, Math.min(slides.length - 1, i));
  slides.forEach((s, j) => s.classList.toggle('is-active', j === cur));
  [...sectionsEl.children].forEach((b, j) => { b.setAttribute('aria-current', String(j === cur)); b.classList.toggle('done', j < cur); });
  document.getElementById('counter').textContent = `${cur + 1}/${slides.length}`;
  renderNotes();
  const id = slides[cur].dataset.id;
  try { history.replaceState(null, '', '#' + id); } catch (e) { /* hash is optional */ }
  requestAnimationFrame(() => {
    renderStatic();
    if (id === 'sandbox') sb.render(true);
  });
}
document.getElementById('next').addEventListener('click', () => show(cur + 1));
document.getElementById('prev').addEventListener('click', () => show(cur - 1));

function toggleNotes(force) {
  const on = force ?? notesEl.hidden;
  notesEl.hidden = !on;
  notesBtn.setAttribute('aria-pressed', String(on));
}
notesBtn.addEventListener('click', () => toggleNotes());
const scriptEl = document.getElementById('script');
document.getElementById('script-body').innerHTML = slides.map((s) => {
  const n = NOTES[s.dataset.id];
  return n ? `<section class="seg-t"><span class="tm">${n.t} · ${s.dataset.label}</span><p>${n.text}</p></section>` : '';
}).join('');
const words = Object.values(NOTES).reduce((n, x) => n + x.text.split(/\s+/).filter((w) => /[\p{L}\d]/u.test(w)).length, 0);
document.getElementById('script-words').textContent = String(Math.round(words / 10) * 10);
const toggleScript = (on = scriptEl.hidden) => { scriptEl.hidden = !on; };
document.getElementById('script-btn').addEventListener('click', () => toggleScript(true));
document.getElementById('script-close').addEventListener('click', () => toggleScript(false));

// timer
const timerEl = document.getElementById('timer');
let tStart = null, tAcc = 0, tTick = null;
function drawTimer() {
  const ms = tAcc + (tStart ? Date.now() - tStart : 0), s = Math.floor(ms / 1000);
  timerEl.textContent = `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
  timerEl.classList.toggle('over', s >= 300);
}
function toggleTimer() {
  if (tStart) { tAcc += Date.now() - tStart; tStart = null; clearInterval(tTick); }
  else { tStart = Date.now(); tTick = setInterval(drawTimer, 250); }
  drawTimer();
}
function resetTimer() { tStart = null; tAcc = 0; clearInterval(tTick); drawTimer(); }
timerEl.addEventListener('click', toggleTimer);
timerEl.addEventListener('dblclick', resetTimer);

document.addEventListener('keydown', (e) => {
  if (e.target.closest('input, select, textarea')) return;
  if ((e.key === ' ' || e.key === 'Enter') && e.target.closest('button')) return;
  if (!scriptEl.hidden) { if (e.key === 'Escape' || e.code === 'KeyS') toggleScript(false); return; }
  const k = e.key;
  if (k === 'ArrowRight' || k === 'PageDown' || k === ' ') { if (!tStart && tAcc === 0) toggleTimer(); show(cur + 1); }
  else if (k === 'ArrowLeft' || k === 'PageUp') show(cur - 1);
  else if (k === 'Home') show(0);
  else if (k === 'End') show(slides.length - 1);
  else if (e.code === 'KeyN') toggleNotes();
  else if (e.code === 'KeyS') toggleScript(true);
  else if (e.code === 'KeyT') { if (e.shiftKey) resetTimer(); else toggleTimer(); }
  else if (e.code === 'KeyF') { (document.fullscreenElement ? document.exitFullscreen() : document.documentElement.requestFullscreen?.())?.catch?.(() => {}); }
  else if (e.key === 'Escape') toggleNotes(false);
  else return;
  e.preventDefault();
});

let resizeT = null;
window.addEventListener('resize', () => {
  clearTimeout(resizeT);
  resizeT = setTimeout(() => { renderStatic(); if (slides[cur].dataset.id === 'sandbox') sb.render(true); }, 120);
});

/* ---------- boot ---------- */
(async () => {
  sb.init();
  await bootEngine();
  await buildStatic();
  await sb.reset();
  const start = slides.findIndex((s) => '#' + s.dataset.id === location.hash);
  show(start >= 0 ? start : 0);
})();
})();
