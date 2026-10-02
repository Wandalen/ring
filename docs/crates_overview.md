# Crates overview of `ring/`

The `ring_*` crates plus `bench_harness`. Lock-free MPSC write-path (many producers → one ordered consumer), `unsafe-code=deny` workspace-wide (only `ring_spsc`/`ring_mpsc` carry a justified `unsafe impl Sync` + accessors).

The `Cargo.toml` member list is in dependency order, so the dependency graph is acyclic by inspection. The public contract is `ring_types`, `ring_handle`, `ring_tls`, `ring_flush`, `ring_factory`. The rest are internal composition.

## Quick reference

The dependency tree by tier is in the root [readme](../readme.md#architecture).

`★` marks the crates meant to be depended on from outside the family.
Everything else is internal composition.

`Direct deps` lists each crate's `[dependencies]`, with test-only edges marked
`+dev`. `Tier` is one more than the highest tier among those, dev-dependencies
included, the same rule as the root diagram. Both come from the crate manifests.

| Crate | Tier | Direct deps | Purpose |
|---|---|---|---|
| `ring_types` ★ | 0 | none | Shared ids, errors, and policy enums that form the family's vocabulary |
| `ring_align` | 0 | none | Cache-line padding to prevent false-sharing |
| `ring_atomic` | 1 | types | Atomic sequence ops with explicit memory orderings |
| `ring_config` | 1 | types | Validated ring construction parameters |
| `ring_index` | 1 | types | Maps a sequence number to its slot |
| `ring_seqno` | 1 | types | Sequence arithmetic (laps, distance, may-claim) |
| `ring_slot` | 1 | types | Typed and raw-byte slot payload views |
| `ring_stats` | 1 | types | Claim/publish/drop counters, observability only |
| `ring_trace` | 1 | types | Optional, off-by-default operation log |
| `ring_batch` | 2 | atomic, index, seqno, types | Claims N items with a single atomic fence |
| `ring_store` | 2 | index, slot, types | The slot array itself, with no synchronization |
| `ring_cursor` | 2 | align, atomic, seqno, types | Cache-padded producer/consumer position cursors |
| `ring_overflow` | 2 | stats, types | What happens when the ring is full |
| `ring_event` | 3 | slot, types (+dev store) | Uniform fill/peek across slot shapes |
| `ring_gating` | 3 | cursor, seqno, types | Producer-side bound: may I claim more? |
| `ring_spsc` | 3 | config, cursor, slot, store, types | Single-producer single-consumer ring |
| `ring_wait` | 3 | cursor, types | Wait strategies: none / spin / yield / park |
| `ring_barrier` | 4 | cursor, types, wait (+dev gating) | Consumer-side bound: what's ready to read? |
| `ring_claim` | 4 | cursor, gating, types | Reserves a sequence range, never blocks |
| `ring_tls` ★ | 4 | atomic, batch, types (+dev event, slot, store) | Thread-local staging ahead of a ring flush |
| `ring_consume` | 5 | barrier, cursor, seqno, types | What may be read, plus commit/ack |
| `ring_mpsc` | 5 | atomic, claim, config, cursor, gating, slot, store, types | Multi-producer single-consumer ring |
| `ring_core` | 6 | config, mpsc, overflow, slot, spsc, types (+opt crossbeam-queue) | Composition point over SPSC/MPSC/crossbeam |
| `ring_publish` | 6 | cursor, types (+dev barrier, claim, consume, gating) | Makes claimed slots visible, in order |
| `ring_debug` | 7 | atomic, core, cursor, types (+dev config) | Runtime invariant checks over a live ring |
| `ring_flush` ★ | 7 | core, tls, types (+dev config) | Policy deciding when staging reaches the ring |
| `ring_handle` ★ | 7 | core (+dev config, types) | Shareable, narrowed producer/consumer ends |
| `ring_poll` | 7 | core (+dev config, types) | Non-blocking, tick-safe helpers |
| `ring_shutdown` | 7 | core, cursor, types, wait (+dev config) | Close, drain, reopen |
| `ring_registry` | 8 | handle (+dev config, core) | Named-ring registry |
| `ring_testkit` | 8 | core, shutdown, tls (+dev config, types) | Deterministic scripted test fixtures |
| `ring_factory` ★ | 9 | config, core, handle, registry, types | The construction entry point; start here |
| `ring_bench` | 10 | core, factory, flush, mpsc, slot, spsc, stats, tls, types | Comparative write-path benchmark |
| `bench_harness` | n/a | none (must never gain `ring_*`) | Family-neutral stage-gate grader |

## Big picture in plain words (for granny)

Imagine a post office with a wall of 8 numbered boxes that get reused forever.
Many people (producers) want to drop letters in, but there is only one mailman (consumer) who picks them up in strict order.

You cannot just throw a letter in, because someone might still be reading the old letter in that box. So everyone follows 4 steps, always in this order:

What `claim → publish → available → commit` means, in words:

1. **Claim = "reserve me a box and a ticket number".** You ask: "can I have ticket 12, 13, 14?" If the mailman is too far behind, you are told "not now, no room". Nothing is visible yet. This is `ring_claim` + `ring_gating` checking with `ring_seqno`.
2. **Publish = "my letter is in the box, you may now look".** You put the letter in, then raise the flag. The flag goes up only after the letter is fully inside, and only when the person before you already raised theirs. Otherwise the mailman would see an empty envelope. This is `ring_publish`.
3. **Available = "mailman, what is ready for you?"** The mailman looks at the flags and computes: "tickets 12-15 are all flagged, I can take them". He does not take them yet, just lists them. This is `ring_consume` + `ring_barrier`.
4. **Commit = "mailman says: I am done with these, reuse them".** Only after he has read the letters and put them in his bag does he move his own pointer forward. That move is what tells producers "those boxes are free now". This is the second half of `ring_consume`.

If you skip a step: publish before writing = mailman reads garbage. Commit before reading = producer overwrites a letter you are still reading. Claim without publish = box stays reserved forever and everyone stalls.

The other crates are just helpers for this story: `ring_types` is the shared vocabulary (ticket, box number), `ring_store` is the wooden wall with boxes, `ring_index` is "ticket 12 → box 4", `ring_wait` is "how to wait politely", `ring_tls` is "write a pile at home first, bring it once", `ring_flush` is "when to bring the pile", `ring_shutdown` is "shop is closed", etc.

## 0. Foundation vocabulary

### `ring_types`. Shared ids, errors, policies. No logic. Leaf, `no_std`.
Holds `Seq(u64)` (never-wrapping position), `SlotIndex` (wrapping position), `Capacity` (pow2-validated + `mask()`), `WaitKind::None/Spin/Yield/Park`, `OverflowPolicy::Fail/DropNewest/DropOldest`, `RingError`. Owns only discriminants. Handlers live in `ring_wait`/`ring_overflow` so a config can travel without dragging parking code.
> Granny: this is the dictionary. Ticket number (`Seq` keeps counting 0,1,2... forever), box number (`SlotIndex` wraps 0..7,0..7...), how big the wall is (`Capacity`), what to do when full, how to wait. No post office logic here, just words everyone agrees on.
```rust
use ring_types::{ Capacity, Seq, OverflowPolicy, WaitKind };
let cap = Capacity::new( 1024 ).unwrap(); // Err if not pow2
```

### `ring_align`. Cache-line separation. Leaf.
`CACHE_LINE=64` + `CacheAligned<T>` (`#[repr(align(64))]` ⇒ size==align ⇒ one value per line). Exists so padding decision is made once; `ring_cursor` just wears it. Prevents false-sharing between producer/consumer cursors.
> Granny: two people shouting into the same small room make each other slow even if they talk about different things. Give each their own room (64 bytes). This crate is just "rooms are 64 bytes big".
```rust
use ring_align::{ CacheAligned, CACHE_LINE };
let p = CacheAligned::new( 7u64 );
assert_eq!( *p.get(), 7 );
```

### `ring_atomic`. All atomics + orderings in one place.
`SeqCell` trait (explicit `Ordering` on every method, never defaults to `SeqCst`), `AtomicSeq` (prod, wraps `AtomicU64`), `CountingSeq` (test double counting `load/fetch_add/CAS` to prove "N items = 1 fence, 0 atomics while staging"). `Sync` is a stated supertrait so `&dyn SeqCell` stays shareable.
> Granny: the counters on the wall must be readable by many eyes at once without tearing. This crate is the only place that touches those shared counters, and it forces you to say how carefully you want to look (`Ordering`). Plus a fake counter for tests that counts how many times you looked.
```rust
use ring_atomic::{ AtomicSeq, SeqCell };
use core::sync::atomic::Ordering;
let c = AtomicSeq::new( Seq( 4 ) );
c.fetch_add( 3, Ordering::AcqRel );
```

### `ring_config`. Construction params as data.
`RingConfig::new(capacity).with_wait().with_overflow().with_producers().with_batch()`. All validation lives here, so legal configs are enumerable and a future manifest language maps 1:1. `ring_factory`/`ring_core` only consume it.
> Granny: the order form for a new post office: "I want 1024 boxes, 4 senders, if full then say No, don't sleep while waiting". Checked once here, so nobody can order "3.5 boxes".
```rust
let cfg = RingConfig::new( 1024 ).unwrap()
  .with_overflow( OverflowPolicy::Fail )
  .with_producers( 4 )
  .with_batch( 64 );
```

## 1. Math / storage

### `ring_seqno`. Sequence arithmetic.
Functions only, no types or state: `laps_between`, `may_claim(prod,cons,cap)` (exclusive at exactly 1 lap, the classic off-by-one), `free_slots`, `pending`, `distance`. Everything `ring_gating`/`ring_barrier` compute builds on this.
> Granny: ticket math. "If I am at ticket 12 and mailman is at 4 with 8 boxes, am I a full lap ahead?" Answer decides if you may reserve. Gets the edge exactly right. At precisely one lap you must stop.
```rust
assert!( ring_seqno::may_claim( Seq( 3 ), Seq( 0 ), cap ) );
assert!( !ring_seqno::may_claim( Seq( 4 ), Seq( 0 ), cap ) );
```

### `ring_index`. `seq → slot`.
`of(seq,cap) = (seq as usize) & mask`, `aliases(a,b,cap)` (same slot = whole laps apart). Its claim "this fold is the only one" is verified by auditing all callers.
> Granny: "ticket 13 goes into which box?" With 8 boxes: ticket mod 8. Ticket 5 and ticket 13 share a box at different times. That is normal reuse, not a bug.
```rust
assert_eq!( ring_index::of( Seq( 8 ), cap ), SlotIndex( 0 ) );
```

### `ring_slot`. Payload views.
`Slot` trait (`is_empty/clear`) + `TypedSlot<T>` (owns value, `set/get/take`, drop runs on `clear`) + `BytesSlot<N>` (memcpy into fixed storage; `clear` only moves len, bytes not zeroed). What `ring_event` fills and `ring_store` holds.
> Granny: what a single box can hold: either a proper typed parcel (`TypedSlot<u32>`) or raw bytes (`BytesSlot`). Empty means "you must not read me", and cleaning a bytes-box just forgets the length, it does not shred the paper inside.
```rust
let mut s = TypedSlot::< u32 >::empty();
s.set( 7 );
assert_eq!( s.get(), Some( &7 ) );
```

### `ring_cursor`. Padded cursors.
`PaddedCursor(AtomicSeq in CacheAligned)`, `CursorPair{producer,consumer}`, `slowest(&[PaddedCursor])->Option<Seq>` (shared fold for gating+barrier so both read `Acquire`), `GATING=Acquire` const. Size+align+real-address (`on_distinct_lines`) all asserted.
> Granny: the two bookmarks: "last ticket handed out" and "last ticket mailman finished". Each lives in its own room (see `ring_align`) so the two people don't slow each other. Plus "who is slowest?" helper both sides share.
```rust
let pair = CursorPair::new( cap );
pair.producer().store( Seq( 3 ), Ordering::Release );
```

### `ring_store`. Slot array.
`Buffer<S>::new(cap)`, `get/get_mut(SlotIndex)`, `capacity()`. No sync itself. The `unsafe` that touches it lives in `ring_spsc`/`ring_mpsc`. Derived `Debug` walks all slots (expensive/leaky, so rings print cursors only).
> Granny: the wooden wall itself. Just boxes in a row, no locks, no thinking. Ask "give me box 2" and you get it. Safety (who may touch when) is decided upstairs.
```rust
let mut b : Buffer< TypedSlot< u32 > > = Buffer::new( cap );
b.get_mut( SlotIndex( 2 ) ).set( 7 );
```

## 2. Protocol in words: reserve, then show, then list, then free

Instead of arrows, read it as a sentence: **first reserve tickets privately, then make them visible in order, then ask what is visible, then mark it done so boxes can be reused.** The four crates below each own one verb, and two helper crates own the "may I?" questions.

### `ring_wait`. Wait strategies.
`WaitKind` enum lives in `ring_types`, 4 handlers here: `None` (probe once and return, for tick path), `Spin/Yield/Park` (bounded counted `for`, budget not deadline ⇒ same samples on every machine). No types of its own, no atomics/`unsafe`/cast. `escalation_hint(Spin)=Yield, Yield=Park`.
> Granny: four ways to wait for a busy toilet: peek once and leave (`None`), pace in place (`Spin`), let others pass (`Yield`), sit down and nap until woken (`Park`). Same question asked N times, only the "what to do between asks" differs.
```rust
assert_eq!(
  ring_wait::escalation_hint( WaitKind::Spin ),
  Some( WaitKind::Yield )
);
```

### `ring_gating`. Producer side bound.
`GatingSet::new(cap,n)` owns N consumer cursors; `headroom(prod)->Result<usize>` = `cap - (prod-min)`, `may_advance`, `slowest()`. Empty set = ungated (not consumer-at-zero, else first lap deadlocks). One stalled consumer stops all. That is correct, and worth detecting.
> Granny: "may I hand out more tickets?" Look at the slowest mailman. If he is one full wall behind you, stop. The next box you would give away is the one he is still reading. If nobody reads at all (empty set), you never stop.
```rust
let g = GatingSet::new( cap, 2 );
g.headroom( producer_seq ).unwrap();
```

### `ring_barrier`. Consumer side bound.
`Barrier::over(&[PaddedCursor])` (borrows, incl. `Publisher::cursor` or `from_ref` for single dep); `frontier()->Option<Seq>`, `available(from)->u64`. No capacity in answer, unlike gating. Same `slowest` fold, opposite question.
> Granny: mirror question: "up to which ticket may I read?" As far as the people in front of me finished. Wall size does not matter here, only "what is ready".
```rust
let b = Barrier::over( &published );
b.frontier();
b.available( Seq( 2 ) );
```

### `ring_claim`. Take ownership of a range.
`Claimer::claim(n)->Result<Claim,RingError>`, `Claim{start,len}` (`#[must_use]`; a dropped un-published claim strands slots, cursor already advanced). Never waits/blocks. The caller composes `ring_wait::for_space + claim`, so `WaitKind::None` stays implementable above.
> Granny: step 1 in words: "these 3 tickets are mine now, nobody else gets them". Returns immediately with yes or "full, try later". If you throw the ticket away without publishing, those boxes are lost forever. Hence must-use.
```rust
let c = Claim::new( Seq( 4 ), 3 );
c.sequences().collect::< Vec< _ > >();
```

### `ring_publish`. Make claimed visible.
`Publisher::try_publish(start,len)->Result<Seq>` via CAS, only when predecessor published; cursor is *not* claim cursor (else consumer sees uninitialised memory; passes single-thread, fails loom). Hosts the claim/publish/consume handshake test (real threads plus an exhaustive loom model). Only unbounded spin in family.
> Granny: step 2 in words: "letters are in, flags up, in ticket order". You cannot raise flag 14 before flag 13 is up, otherwise mailman sees a hole. Uses a separate "published" bookmark so nobody peeks at half-written letters.
```rust
let p = Publisher::new();
p.try_publish( Seq::ZERO, 3 ).unwrap();
```

### `ring_consume`. What may be read + ack.
`Available{start,len}` (`Copy`, `start/end/len/sequences/is_empty`), `Consumer::available()->Available` + `commit(up_to)` (monotonic + clamped to available; over-commit frees unread slots, under-commit re-reads). Split (not RAII guard) so partial commit + borrowed-slot window is explicit.
> Granny: steps 3 and 4 in words. 3: "what pile is ready starting from where I stopped?" 4: "done with these, you may reuse the boxes". Never say done for what you did not read, never go backwards.
```rust
let r = Available::new( Seq( 2 ), 3 );
assert_eq!( r.len(), 3 );
```

### `ring_overflow`. Full-ring policy. `no_std`.
`would_resolve(policy)->Resolution`, `Resolution::Refused/DroppedIncoming/EvictedOldest::lost_an_item()`, `ALL=[3]`. `OverflowPolicy` owns config discriminant, this owns behaviour. No `Default` on `Resolution` (nothing happened yet ⇒ no outcome).
> Granny: "wall is full, what now?" Say no to the newcomer (`Refused/Fail`), quietly throw the newcomer away (`DropNewest`), or throw the oldest mail away to make room (`DropOldest/EvictedOldest`). Records which choice lost a letter.
```rust
assert!( Resolution::DroppedIncoming.lost_an_item() );
```

### `ring_batch`. One fence for N items.
`BatchClaim{start,count}` + `claim(n,cell,Ordering)->BatchClaim`. Exactly 1×`fetch_add` whatever N (asserted via `CountingSeq`); contiguous (preserves TLS buffer order). Below ~8 items nothing to amortise; batch-of-1 == single cost. Range only, no storage.
> Granny: instead of queuing 64 times for 64 tickets, take 64 consecutive tickets with one ask at the counter. Cheaper, and your home pile stays in order when it lands.
```rust
let b = BatchClaim::new( Seq( 10 ), 3 );
```

### `ring_event`. Unify write path across slot shapes.
`Fill<S>::fill(self,slot)`, `Peek<S>` (GAT; `Typed` returns `&T`, `Bytes` returns `&[u8]` without copy), `publish_into/drain_from` written once generically. Declared but not yet called by `ring_core` (calls `TypedSlot::set/take` directly). The adoption gap is explicit.
> Granny: "how does a letter get into any kind of box the same way?" One instruction that works for both parcel-boxes and byte-boxes, so the two paths cannot drift apart. Reading borrows without photocopying.
```rust
7u32.fill( &mut slot ).unwrap();
```

## 3. Rings

### `ring_spsc`. Single-producer ring.
`Ring<S>::new(cap)/with_config`, `split()->(Producer,Consumer)` (borrowed, `!Sync` stops cross-thread share at compile time), `Producer::claim/push_with/try_push`, `Consumer::drain/drain_up_to->Batch`. 2×`Release` stores, no RMW. `unsafe` (the slot accessors + `Sync`) sited here with loom model (x86 passes 100k items even with wrong ordering; loom catches it).
> Granny: one sender, one mailman. Simplest post office: no queue at the counter, just two flags. The compiler itself forbids a second sender or sharing ends across threads.
```rust
let mut r : Ring< TypedSlot< u8 > > = Ring::new( cap );
let ( mut p, mut c ) = r.split();
p.try_push( 1 ).unwrap();
c.drain();
```

### `ring_mpsc`. Multi-producer ring.
`Ring::new(cap)`, `ends()->Ends`, `Ends::split()->(Producer:Copy,Consumer:!Clone/!Sync)`, `Reserved(DerefMut)->publish on drop`, `published_through()`. Per-slot stamp (=seq when published) + consumer scan. No producer waits for its predecessor (unlike `Publisher::publish`); the consumer pays for the scan. `ring_publish`/`ring_consume` deliberately *not* used (cursor-advance shape doesn't fit).
> Granny: many senders, one mailman. Each sender stamps its box with its ticket number when done; mailman scans forward "is the stamp what I expect?" No sender waits for another sender. The mailman does a bit more looking instead. Sender handle is copyable (pass it to threads), mailman handle is not.
```rust
let mut r : Ring< TypedSlot< u8 > > = Ring::new( cap );
let mut e = r.ends();
let ( p, c ) = e.split();
```

### `ring_core`. Composition point.
`Ring<T>::new(&RingConfig)`, `ends/split`, `Producer::try_push/try_clone` (`None` on SPSC, `Some` on MPSC/crossbeam), `Consumer::try_recv`, `backend()->Backend::Spsc/Mpsc/Crossbeam`, overflow/batch/event support. Value-shaped API (crossbeam has no slots ⇒ no uniform reservation API; need in-place ⇒ use `ring_spsc/mpsc` directly). Crossbeam is feature-gated interim until in-house rings earn history.
> Granny: the switchboard. You say "I need a post office for this form" and it picks the small one (SPSC), the big one (MPSC), or a rented one next door (crossbeam) while ours earns trust. Simple push-a-value / pop-a-value buttons; if you need to build the letter directly inside the box, go to SPSC/MPSC downstairs.
```rust
let mut r : Ring< u32 > = Ring::new( &cfg ).unwrap();
let mut e = r.ends();
let ( mut p, mut c ) = e.split();
p.try_push( 7 ).unwrap();
c.try_recv();
```

## 4. Composition / ops and the public contract

### `ring_handle`. Shareable ends. Contract crate.
Adds 4 narrowings over `ring_core`: ring taken by value (no double-split), no `try_clone` (no 2nd producer at compile time), bounded `Drain` iterator (live-producer drain terminates), no `Deref/inner` (backend unreachable). `&mut self` receivers make "exactly one producer" a borrow-checker property. No `is_closed` (would need `ring_wait` ⇒ parks on tick path, so refused).
> Granny: the two keys to the post office, cut once. You cannot copy the sender key here, cannot open the back door to the machinery, and "empty everything" stops after what was there when you started (so it ends even if new mail keeps coming).
```rust
let mut s = Split::new( ring );
let ( mut p, mut c ) = s.ends().split();
p.try_push( 1 ).unwrap();
```

### `ring_tls`. Thread-local staging. Contract crate.
`TlsBuffer<T>::with_capacity(limit)` (`Vec` reserved once to refusal bound), `push()->Err` when full, `len/is_empty`, `flush_into(cell,Ordering)->Flush` (1×`fetch_add` for N via `ring_batch`, asserts 0 atomics while accumulating via `CountingSeq`). Never publishes itself. Needs a `ring_flush` trigger.
> Granny: each sender has a notepad at home. Scribble 64 notes with no talking to anyone (zero shared counters), then bring the whole pile to the post office in one trip (one counter bump). The notepad never walks to the post office by itself.
```rust
let mut b = TlsBuffer::with_capacity( 64 );
b.push( 1 ).unwrap();
b.flush_into( &cursor, Ordering::AcqRel );
```

### `ring_flush`. When staging lands. Contract crate, decision not thing.
`FlushPolicy::OnFull/OnBarrier/OnBatch(n)` (only `OnBatch` owns its param; counter == `buffer.len()` so no extra state), `Flusher::new(buf,producer,policy)` (by value ⇒ 1 policy/buffer, `ZeroBatch` rejected), `append/drive/drive_at_barrier`, opt-in `FlushLog`. 3 obligations (all caller's): drive it (no thread/timer/`Drop`), announce barriers truthfully, retry rejected final drain (else silent loss).
> Granny: "when do I carry the notepad to the post office?" When it is full (`OnFull`), when the boss shouts "barrier!" (`OnBarrier`), or every N notes (`OnBatch(8)`). Nobody carries it for you. You must drive, tell the truth about barriers, and retry the last carry if refused or notes vanish.
```rust
let f = Flusher::new( buf, producer, FlushPolicy::OnBatch( 8 ) );
f.append( x );
f.drive();
```

### `ring_factory`. The door (verb). Contract crate.
`Factory.build::<T>(cfg)->Result<Split<T>,BuildError>`, `build_named(cfg,registry,name)`, `build_crossbeam`. Returns owner `Split`, not handle pair (pair borrows `Split` ⇒ self-referential if returned together). Re-exports `RingConfig,Registry` so contract consumers name nothing else. Only path to `SPSC/MPSC/registry`.
> Granny: the front desk. You hand in the order form, you get back the whole post office in a box. Then you open it into two keys (sender + mailman). There is deliberately only one front desk so nobody builds a crooked post office around the back.
```rust
let mut s = Factory.build::< u32 >( RingConfig::new( 8 ).unwrap() ).unwrap();
let ( mut p, mut c ) = s.ends().split();
p.try_push( 7 ).unwrap();
```

### `ring_registry`. Named rings.
`Registry::new()`, `register(name,Split)->Result` (owns; refusal hands ring back, which avoids `HashMap::insert` silently dropping live ring + unread records), `get_mut/remove/contains/len/names`. One `T` per registry (no downcast), no `get` (`&Split` permits nothing).
> Granny: a phone book "events → that post office". Registering a taken name does not bulldoze the old post office (with unread mail!). It says no and gives your box back. Borrowing is always mutable because read-only keys are useless.
```rust
registry.register( "events", Split::new( ring ) ).unwrap();
registry.get_mut( "events" );
```

### `ring_shutdown`. Liveness.
Family's only `closed: AtomicBool`. `Shutdown::close()->Stopped`, `Stopped::drain_all/reopen`, `Guarded` producer (only push that checks flag; raw `Producer` can still publish into closed ring, so the flag is advisory, a documented pitfall). Typing enforces `close→drain` order (drain is method on `Stopped`; open-ring drain would never terminate).
> Granny: the OPEN/CLOSED sign. Closing gives you a special token; only with that token can you say "sweep everything left and finish". The sign is advisory. A sender who ignores it can still drop mail in, unless you gave them the guarded sender that checks the sign first.
```rust
let s = Shutdown::new();
let stopped = s.close();
stopped.reopen();
```

### `ring_poll`. Tick-safe helpers. Never depends on `ring_wait`, not even transitively; a test enforces this.
`Budget::once()/new(n)` (attempts not time; `0→1`), `Progress`, `try_push/try_recv/drain_up_to` bounded helpers + `tick` accumulator, `PARKING_CRATES=[barrier,shutdown,wait]`. Non-parking ≠ bounded latency (`Budget(1M)` never deadlocks but drops frame, so default is `once()`).
> Granny: for a game loop with 16ms per frame: "try once, never nap here". A budget is a number of peeks, not milliseconds. Big budget still returns, just late, so default is one peek. The crate also keeps the list of who is allowed to nap, and tests that this list never leaks in here.
```rust
let b = Budget::once();
poll::try_push_with_budget( &mut p, v, b );
```

## 5. Observe / test / measure

### `ring_stats`. Counters. `no_std`.
`RingStats::record_claim/publish/drop(policy,n)`, `claimed/published/dropped(policy)/dropped_total/in_flight/reset`. No production path reads them yet, and nothing on a hot path depends on them. Observability only. No `Clone/Copy/PartialEq` (one atomic per counter ⇒ no snapshot; `Debug` loads them one at a time, so it can show a spread that never existed).
> Granny: the tally sheet: how many tickets given, published, thrown away. For the manager, not for the workers. Nobody on the fast path reads it. Reading the counters one at a time can show a combination that never existed at once.
```rust
stats.record_claim( 4 );
stats.record_publish( 4 );
stats.claimed();
```

### `ring_trace`. Op log.
`TraceOp::Claim/Publish/Consume/Commit/Drop::name()`, `ALL=[5]`, `Trace::record(op,seq)`. Optional, off by default. For post-mortem ordering disputes, not the hot path.
> Granny: the CCTV notebook: "at ticket 12: reserved, published, read, freed, thrown". Off unless you ask. You replay it after an argument about who did what when.
```rust
trace.record( TraceOp::Claim, Seq( 1 ) );
```

### `ring_debug`. Invariants.
`check(&CursorPair)` (consumer-ahead, producer>1 lap-ahead), `Watch` (adds backwards-move over ≥2 obs), `check_ends(&Split)` (two public readings disagree). Never called on claim path. Catches the silent case: violated ring reads as healthy (`free=cap,pending=0,may_claim=true`).
> Granny: the health inspector who never works during rush hour. Call him in tests: "is the mailman ahead of senders? Did anyone go backwards? Do two dials disagree?" Broken rings look perfectly healthy to normal dials, so you need him.
```rust
assert!( ring_debug::check( &pair ).is_ok() );
```

### `ring_testkit`. Deterministic scripts.
`Script::new(staging_cap).then(Step::Push/PushMany/Recv/RecvMany/Stage/StageMany/Flush/Close/Reopen/DrainAll).run(&mut Ring)->Outcome{received,dropped,…}` (no timing ⇒ runs compare equal; `audit()` checks conservation). Exists for the `DropNewest-Ok-but-destroyed` measurement (same counts, different records vs `Fail`).
> Granny: a recipe card: "push 8, read 3, close, sweep". Run the same card on two post offices and compare the bags letter-for-letter. No stopwatch is involved, so equal cards mean equal results. Catches sneaky "said OK but threw the letter away" behaviour.
```rust
Script::new( 0 )
  .then( Step::PushMany( 8 ) )
  .then( Step::DrainAll )
  .run( &mut ring );
```

### `ring_bench`. The number.
`Workload::new(cfg).with_records_per_producer(n)`, `Comparison::run(w)->{mutex,ring,staged}`, `fastest()->Option` (filters `is_lossless` first; fastest refuser must not win), `Outcome{offered/reported/received/dropped}`. Times write only; drain afterwards for accounting. Never asserts ranking (flaky on shared box).
> Granny: the race track. Same pile of letters race on three bikes: plain locked box (mutex), post office (ring), notepad-then-post-office (TLS+flush). Only the write is timed; counting happens after. A bike that throws letters away is disqualified even if fastest.
```rust
let c = Comparison::run(
  Workload::new( cfg ).with_records_per_producer( 256 ).unwrap()
);
c.fastest();
```

### `bench_harness`. Neutral grader. Depends on no `ring_*` crate, because the grader must run before graded builds.
`Workload` (seeded items), `Accumulator/Write` (fold semantics), `ByteParity/Parity` (table agreement + first divergence), `gate/declared/<family>/gates.txt` + `gate/run_all.sh --family <name>`. `ring_bench` grades write-path candidates; this grades a declared family. Lives here because ring was its first target.
> Granny: the exam board, independent of any school so it can examine empty classrooms on day one. Hands out the same seeded homework to everyone, then checks byte-for-byte if final tables match and points at the first difference. Run `gate/run_all.sh --family ring` for the current score, don't trust a number copied here.
```rust
let w = Workload::seeded( 42, 1000 );
ByteParity::check( &a, &b );
```
