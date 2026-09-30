# Backlog

Known problems in the workspace and how to fix them. Every item was checked against the code on
`master` (`c7d2042`) on 2026-09-30. References name symbols, not lines: line numbers drift.

How to use: one item, one PR, unless the order table says otherwise. In that PR tick the item
`[x]` and add the PR number. The usual rules apply: a bug fix starts with a failing test; a change
to atomics starts with a loom model (`./verb/loom`) and the `lock-free-review` checklist; a
behaviour change updates the crate's `docs/` in the same PR.

Priorities: **P0** — data is lost or the numbers lie; **P1** — the main win; **P2** —
noticeable; **P3** — when convenient.

## 1. Performance

### Measure first

The recorded `ring_bench` numbers (`ring_bench/tests/manual/readme.md`) cannot support any claim
about speed: debug build, candidates measured under different conditions. Every optimisation
below starts from a baseline taken after M1–M5 and brings a before/after table in its PR.

M1–M7 overlap capstone topic 7 (`docs/capstone/07_benchmarking.md`): agree with its owner.
Several of them reverse positions `ring_bench/docs` records on purpose; the PR changes those
instances and their recipes too.

- [ ] **M1 · P0 · Measure in release.** `verb/bench` runs `cargo run` without `--release`; the
  manual numbers come from `cargo test` (test profile, `opt-level = 0`). No `Cargo.toml` has a
  `[profile.*]`.
  Fix: `--release` in `verb/bench`; a `[profile.bench]` (or a dedicated `[profile.perf]`) with
  `lto = "fat"`, `codegen-units = 1`, `debug = 1` for the profiler. Leave `overflow-checks` off:
  BN22 (`ring_bench/docs/invariant/001`) already chose an unconditional `assert!`, and
  `both_ordering_subtractions_are_guarded_unconditionally` plus a G15 recipe pin that.
- [ ] **M2 · P0 · Same timed region.** `run_mutex_queue` and `run_direct_mpsc` create threads
  (`std::thread::scope` + `spawn`) inside the timer; `run_direct_spsc`, `run_contract_ring`,
  `run_tls_over_ring` and `run_off_the_shelf` run in the calling thread. Spawn and join land in
  the first two's numbers. `ring_bench/docs/non_functional_requirement/002` accepts this as
  "Residue 1" (also BN23, BN35).
  Fix: threads start before the timer and wait on a `std::sync::Barrier`; time runs from the
  barrier to the last write. The same for every candidate.
- [ ] **M3 · P0 · The consumer runs during the measurement.** Every runner drains after the
  timer stops; on the roomy workload the ring holds the whole run (4096 slots, 256 records): no
  cursor ping-pong between cores, no back-pressure. Not the load the ring was built for. Draining
  outside the clock is a stated choice in the same NFR 002.
  Fix: the consumer is a separate thread started at the same time; measure end to end, from push
  (`try_push`/`push`) to receive (`try_recv`/`drain`).
- [ ] **M4 · P1 · Latency, not only total time.** The root `readme.md` promises to remove
  lock-convoy jitter; the bench reports one `write_nanos` per candidate per run, and NFR 002
  accepts "no percentiles".
  Fix: a per-operation latency histogram (every Nth operation, log buckets or `hdrhistogram`),
  reporting p50/p99/p999/max. For latency the producer runs at a fixed rate (open loop), or
  coordinated omission hides the tail. Throughput is a separate line.
- [ ] **M5 · P1 · Size and repeats.** The example offers 256 records (`Workload::new` defaults to
  1024) with no warm-up. Sections A–C run once; section D (`stability`) repeats 10 times but
  reports only the distinct winners and `contract_ring`'s min/max. Spread between rounds reaches
  2.9× (`direct_spsc` 30800–90240 ns, `contract_ring` 32960–95800 ns).
  Fix: ≥ 10⁶ records, warm-up, N repeats, median and interquartile range. The report names the
  CPU, core count and commit. Optionally pin threads to cores.
- [ ] **M6 · P2 · Same work.** `mutex_queue` takes the lock once per batch (32; 16 on the
  cramped workload, clamped to capacity); `direct_spsc` and `direct_mpsc` push one at a time;
  `contract_ring`, `tls_over_ring` and `off_the_shelf` go through `try_push_batch`, which is a
  per-record loop inside (BN28). Fix: two modes per candidate, one at a time and batched.
- [ ] **M7 · P2 · Tool.** `docs/capstone/07_benchmarking.md` and
  `ring_mpsc/docs/non_functional_requirement/001_measured_before_adopted.md` call `ring_bench` a
  criterion harness; no `Cargo.toml` has `criterion`. Decide: criterion for per-operation cost
  micro-benchmarks, plus the own harness for MPSC latency under contention. The p99 regression
  gate is topic 7's.

After M1–M5, re-take the tables in `ring_bench/tests/manual/readme.md` and
`ring_bench/docs/decisions/002_no_test_asserts_an_ordering.md`, and write the first measurement
and verdict into `ring_mpsc/docs/non_functional_requirement/001_measured_before_adopted.md`,
which has none yet. Only then is it known whether MPSC loses to the mutex (today it does, in
debug numbers only: 1107849 vs 533764 ns at 4 producers).

### Optimisations

Each one after the baseline, each with before/after numbers at 1, 2, 4 and 8 producers.

- [ ] **P1 · P1 · SPSC caches the other side's position.** `ring_spsc::Producer` holds only
  `&Ring`; every `claim` goes through `occupancy`, which reads the consumer cursor with `GATING`.
  With the consumer running in parallel that is a cache miss per operation. `Consumer::drain`
  (and `drain_up_to`, `available`, `is_empty`) read the producer cursor the same way.
  Fix: the producer keeps the last consumer position it saw (a plain field or `Cell` — the end is
  `!Sync`) and re-reads the atomic only when the cache shows too little room; the consumer caches
  the producer position the same way. A stale cache is conservative: cursors are monotonic, so
  room is only underestimated. `free_capacity` stays an honest fresh read. Extend the SPSC loom
  model (`ring_spsc/tests/spsc_test.rs`, `mod exhaustive`).
- [ ] **P2 · P1 · Batch reservation through `ring_core`.** `ring_core::Producer::try_push_batch`
  is a loop of `try_push`, one reservation per record.
  Fix: SPSC — one room check for n, n writes, one `store(HANDOFF)`; MPSC —
  `Claimer::claim_up_to(n)` (exists, unused in `src/`): one CAS, n writes, n stamps. Needs a range
  guard in `ring_mpsc` that publishes every stamp on `Drop`. The iterator has no known length and
  a claimed MPSC range cannot be given back, so bound n first (`ExactSizeIterator`, `size_hint`,
  or pull the records, then claim). Reserving before taking records also closes L1.
- [ ] **P3 · P1 · `ring_flush` through batch reservation.** `Flusher::run` does
  `buffer.drain()` → `try_push_batch`, one `try_push` per record. The one-`fetch_add`-per-batch
  path (`ring_tls::TlsBuffer::flush_into` → `ring_batch::claim`) is called by nothing in `src/`:
  `ring_core` does not expose its cursor. It also uses the ungated `claim`, which is unsafe on a
  bounded ring. After P2 a flush is one reservation; then decide on `flush_into` and `ring_batch`
  (see A1).
- [ ] **P4 · P2 · Consumer commits per batch.** `ring_core::Consumer::try_recv` is
  `drain_up_to(1)`, `take`, then the batch's `Drop`: one Release store of the cursor per record,
  on the line the producers read. `Batch::drop` stores even when the batch is empty, so an empty
  poll writes it too. `ring_poll::drain_up_to` is a loop of `try_recv`.
  Fix: skip the store for an empty batch; document `try_recv_batch` as the main path and add a
  bounded batch receive; experiment with a lazy commit (every K records or when empty). Cost:
  slots are freed later, `Full` comes sooner.
- [ ] **P5 · P2 · MPSC claim caches its limit.** `Claimer::claim` (and `claim_up_to`) re-reads
  `GatingSet::headroom`, that is the consumer cursors, on every CAS iteration — by design, argued
  in `ring_claim` and `ring_gating`.
  Fix: keep the computed limit (`GatingSet::limit`, `slowest + capacity`) in an atomic shared by
  the producers, and re-read the consumer only when `current + n` passes it. Write the cache with
  Release and read it with Acquire, or a producer using another's cached limit loses the
  COMMIT → GATING edge. A stale limit is conservative. Needs a loom model.
- [ ] **P6 · P2 · Stamp next to the slot.** In `ring_mpsc::Ring` the stamps are a separate
  unpadded array `stamps: Box<[AtomicSeq]>`. A publish touches two lines (slot and stamp),
  neighbouring producers share a stamp line (8 per line), and the consumer scans it. The field
  comment says there is no contention here — check on M3/M4.
  Experiment: a `{ stamp, payload }` cell, as in Vyukov's bounded MPMC — one line per publish.
- [ ] **P7 · P2 · `CACHE_LINE` 64 → 128.** `ring_align::CACHE_LINE = 64`. On x86_64 the
  adjacent-line prefetcher pulls 128-byte pairs, so cursors on neighbouring 64-byte lines still
  interfere; big aarch64 cores have 128-byte lines. `crossbeam_utils::CachePadded` aligns to 128
  on both for these reasons. `ring_cursor::CursorPair` puts the SPSC cursors on adjacent lines.
  The constant alone changes no layout: `CacheAligned` has a literal `#[repr(align(64))]`, and
  `ring_mpsc`'s `on_distinct_lines`, two doctests and `ring_align/tests/align_test.rs` hard-code
  64. Measure cursor ping-pong on M3 before and after.
- [ ] **P8 · P2 · Consumer waiting.** There is no blocking `recv`. `WaitKind::Park` is
  `thread::sleep(50 µs)` with no wake-up (`ring_wait::pause`), a latency floor when parking.
  `ring_wait::for_data` takes a `CursorPair`, which `ring_core` does not expose and MPSC does not
  have.
  Fix: spin → yield → park with a real `unpark` and a "consumer asleep" flag the producer checks
  with one load. Needs a decision: the tick path must not park (`ring_poll::PARKING_CRATES`).
  Related to A3.
- [ ] **P9 · P3 · Claim CAS ordering.** `CLAIM_SUCCESS = AcqRel` (private) in `ring_claim`. Check
  whether the Acquire half is needed when the limit is read separately with `GATING`. No
  difference on x86, possibly on ARM. Only with loom evidence; leave `SeqCst` alone. `ring_claim`
  has no loom model, and none of the existing ones laps with two or more producers — write one.
- [ ] **P10 · P3 · Dispatch in `ring_core`.** Every operation matches on `ProducerInner` /
  `ConsumerInner`. Look only if it shows up in a profile; one option is a generic
  `Ring<B: …>` (the name `Backend` is taken by an existing enum).

## 2. Data loss and correctness

- [ ] **L1 · P0 · `try_push_batch` destroys a record on `Fail`.** In
  `ring_core::Producer::try_push_batch` a record is taken from the iterator and passed to
  `try_push`. On refusal it comes back in `Err(record)`, but the loop only checks `.is_err()`
  and the record is dropped. `ring_handle::Producer::try_push_batch`'s doc promises the opposite:
  the iterator stays at the first record that did not fit. Checked: capacity 4, `Fail`, batch of
  6 — 4 accepted, record 4 destroyed, the iterator yields 5. `ring_core`'s own doctest and
  `ring_poll`'s `push_batch_within_eats_one_record_per_attempt` pin the loss; the fix updates
  both. Also hits `ring_poll::push_batch_within` and `Tick::push_batch` (`Tick::lost`). In
  `ring_flush` only with a second producer (it checks `free_capacity` first), and there the
  refused tail is dropped by `Vec::drain` anyway.
  Fix: a failing test first. Then P2 (reserve before taking a record) or, faster, hand back the
  refused record (`(usize, Option<T>)` or a dedicated type). That changes the API of the exported
  `ring_handle`.
- [ ] **L2 · P0 · `Ok` on loss.** The default policy is `DropNewest`
  (`ring_types::OverflowPolicy`). On a full ring `ring_core::Producer::try_push` returns `Ok(())`
  and discards the record; `try_push_batch` counts it as accepted. `ring_testkit`: 8 "accepted",
  4 vanished. `ring_bench` at capacity 16: 256 offered, 256 reported, 16 received — 240 lost
  without a single error.
  Options: (a) `Fail` by default; (b) a distinguishable result, `Ok(Pushed::Stored |
  Pushed::Dropped)`; (c) keep it, but count it (L3) and document it plainly. Needs a decision
  record in `ring_core/docs/decisions/`. (a) and (b) change the behaviour of exported crates.
- [ ] **L3 · P1 · Statistics are not wired.** No live ring holds a `RingStats`: `ring_core`
  reaches `ring_stats` only transitively through `ring_overflow`, and calls the stats-free
  `would_resolve`. `ring_overflow::resolve`, which moves the drop counters, is called only by its
  own tests; `ring_bench` records drops itself after the run. `ring_core`'s module doc says
  `ring_stats` is deliberately not a dependency — the fix revises that.
  Fix: count at least the cold events in `ring_core` — `dropped_newest` and `failed`. Hot counters
  (`published` per operation) are an atomic shared by every producer, which is contention: a
  counter per producer, or no counting. Measure on M3.
- [ ] **L4 · P2 · A hole that never closes.** A leaked `ring_mpsc::Reserved` (`mem::forget`) or a
  dropped `ring_claim::Claim` leaves an unpublished `Seq`; the consumer stalls forever, the
  producers fill up and get `Full`, and nothing reports it. `ring_mpsc/docs/lifecycle/003` says
  this cannot be forgotten and is bounded — false under `mem::forget`.
  Fix: a detector in `ring_debug` ("the consumer has stood on one `Seq` longer than X while
  `claimed > position`"), a `pitfall/` instance in `ring_mpsc`, and the lifecycle text corrected.
- [ ] **L5 · P2 · `#[must_use]` in `ring_mpsc`.** `ring_mpsc::Reserved` and `ring_mpsc::Batch`
  lack it; `ring_spsc::Reservation` and `ring_spsc::Batch` have it. The `ring_handle` UI tests
  quote none of these types.
- [ ] **L6 · P2 · Debug invariant checks.** Saturating arithmetic (`ring_seqno::free_slots`,
  `Seq::distance_to`) shows a ring whose consumer is ahead of its producer as empty — `ring_debug`
  invariant `c ≤ p` (`docs/invariant/001`), measured in `docs/pitfall/001`.
  Fix: `debug_assert!` (`consumer ≤ producer ≤ consumer + capacity`) on the hot paths of
  `ring_spsc` and `ring_mpsc`. Free in release.
- [ ] **L7 · P3 · A previous lap's record published again.** The `Drop` of the
  `ring_mpsc::Reserved` guard publishes the slot even if nothing was written. If the consumer
  read with `get` rather than `take`, the slot still holds the previous lap's record, and it is
  delivered a second time (checked). `Reserved`'s doc assumes the slot was left `Default`, which
  `push`'s own comment contradicts. `ring_core` always writes and always `take`s, so the risk is
  only for direct users of `ring_mpsc`.
  Fix: document it, or clear the slot on `claim`.

## 3. API and architecture

- [ ] **A1 · P1 · Crates off the working path.** The production rings do not use
  `ring_publish`, `ring_consume`, `ring_barrier`, `ring_event`, `ring_trace`; `ring_poll` and
  `ring_debug` are leaf add-ons over `ring_core` that nothing depends on. `ring_barrier` is used
  only by `ring_consume`; `ring_batch` only by `ring_tls::flush_into`, which nothing in `src/`
  calls; `ring_wait` only by `ring_barrier` and `ring_shutdown`. Each has a full `docs/` corpus
  and gates.
  Fix: a decision for each — wire it in, mark it "reference model for loom", or remove it.
  Removal touches `bench_harness/gate/declared/ring/*`, `readme.md`, `architecture.md`.
- [ ] **A2 · P2 · `RingConfig` promises too much.** The assembled ring reads neither `wait` nor
  `batch` (`ring_factory/readme.md` marks both unobservable). Wire them (`batch` as the default
  batch size for P2/P3, `wait` for P8) or remove them.
- [ ] **A3 · P2 · No blocking API.** `ring_core` and `ring_handle` offer only `try_*` (and a
  bounded `drain`), so users write their own wait loops. Try-only is a stated requirement
  (feature 183, cited in `ring_poll`), so this needs a decision. Do it together with P8.
- [ ] **A4 · P2 · Shutdown can be bypassed.** A raw `Producer` does not see the `ring_shutdown`
  flag. `Shutdown::close` hands out a new `Stopped` on every call, so after two `close` and one
  `reopen` the ring is open while a live token can still drain (checked). Handing out only
  `Guarded` is not enough: `Guarded::shutdown()` can close the ring, `Guarded::into_inner()`
  returns the raw producer, and `Stopped::shutdown()` mints more tokens.
  Fix: `close` returns a token only on the Open → Closed transition (`Option<Stopped>`), and the
  three escape hatches go or are restricted.
- [ ] **A5 · P3 · Errors.**
  - `RingError::NameUnknown` is constructed nowhere outside tests; `ring_registry` returns
    `Option`.
  - `ring_wait::wait_until` reports every timeout as `Empty`; `for_space` maps it to `Full`.
  - `ring_consume::Consumer::commit` returns `Empty` for two different refusals (not read yet,
    backwards).
  - `BytesSlot::write` returns `BatchTooLarge` for bytes.

  `RingError` is `#[non_exhaustive]`: adding a variant is fine, removing one is a breaking change.
- [ ] **A6 · P3 · Unbounded spin.** `ring_publish::Publisher::publish` spins forever if a
  predecessor dropped its `Claim`. The module doc argues against a budget, and `try_publish`
  already exists as the bounded variant. Only its own tests use the crate; if A1 keeps it, make
  the bounded path the default.
- [ ] **A7 · P3 · Two consumers on one cursor.** `ring_consume::Consumer` does not prevent a second
  instance over the same cursor (checked: both commit `Ok`). It is also `Sync` with `&self`
  methods, so one instance shared across threads does the same. Decide together with A1.

## 4. Documentation and process

- [ ] **D1 · P2 · Links to nowhere.** 32 files in `ring_*/src` cite 21 documents under
  `docs/feature/` and `docs/decision/` (features 167–175, 177–180, 182–185; decisions 050, 121,
  123, 124). Neither directory exists in the repository. `bench_harness/src` has two more; the
  crates' `tests/` and `docs/` cite them too. Check:
  `grep -rlE 'docs/(feature|decision|plan)/' ring_*/src bench_harness/src`.
  Fix: replace them with links inside the repository or remove them; extend link checking (like
  G16) to doc comments in `src/` so they do not come back (`new-gate` skill).
- [ ] **D2 · P2 · Prose against code:**
  - the tier graph in `readme.md` counts dev-dependencies without saying so; by normal
    dependencies `ring_event` is tier 2 (not 3), `ring_publish` 3 (not 6), `ring_tls` 3 (not 4).
    Pick the definition, state it, and generate the graph from the manifests;
  - `ring_align/readme.md` and its `lib.rs` say it depends on `ring_types` — it has no
    dependencies;
  - `ring_tls/readme.md` lists `ring_types`, `ring_slot`; the manifest has `ring_types`,
    `ring_atomic`, `ring_batch` (`ring_slot` is dev-only). `ring_batch/readme.md` omits
    `ring_atomic` and `ring_index`;
  - `ring_handle`: the manifest description says "Shareable" ends, yet the handles are neither
    `Clone` nor `Sync`; its readme counts 5 UI tests, there are 7;
  - the `lib.rs` of `ring_spsc` and `ring_core` say `publish = false`, the manifests `true`;
  - `ring_tls/docs/` describes an API that does not exist (acknowledged in `decisions/001`);
  - `ring_trace::Trace::disabled` calls itself "the default a ring is built with"; no crate
    depends on `ring_trace`;
  - `ring_overflow` cites `ring_core:337`, which has moved;
  - `bench_harness/readme.md` and its docs mention an "orbital" family, but
    `gate/declared/orbital` does not exist; `verb/gate`, `verb/test` and `verb/readme.md` say
    "G1–G21", but G22 exists (and G11 does not);
  - `ring_bench/readme.md`: "every candidate is measured under both accumulator semantics" — the
    code supports both (`with_semantics`), but nothing selects `Delta`;
  - `docs/capstone/07_benchmarking.md`: "criterion-based" (see M7).
- [ ] **D3 · P2 · Stale allowlist entry.** `ring_core` is on
  `bench_harness/gate/declared/ring/unsafe_allowlist.txt` without a single `unsafe`, and G6's
  stale-entry check makes G6 red on `master` for it. Removing the entry turns G6 green;
  `capstone.md`, `docs/capstone/readme.md` and the allowlist header ("meant to stay three")
  change with it.

## PR order

| #  | PR                                                                                  | Items         | Depends on          |
|----|-------------------------------------------------------------------------------------|---------------|---------------------|
| 1  | `fix(ring_core): hand back the refused record from try_push_batch`                  | L1            | —                   |
| 2  | `fix(ring_mpsc): mark Reserved and Batch must_use`                                  | L5            | —                   |
| 3  | `perf(ring_bench): release build, equal timed region, concurrent consumer, repeats` | M1–M3, M5, M6 | topic 7             |
| 4  | `perf(ring_bench): per-operation latency percentiles`                               | M4            | 3                   |
| 5  | decision: `DropNewest` and `Ok`, drop counters                                      | L2, L3        | 3 (counter cost)    |
| 6  | `perf(ring_core): batch reservation for spsc and mpsc`                              | P2            | 1, 4                |
| 7  | `perf(ring_flush): flush through one reservation`                                   | P3            | 6                   |
| 8  | `perf(ring_spsc): cache the peer cursor`                                            | P1            | 4                   |
| 9  | `perf(ring_claim): cache the gating limit`                                          | P5            | 4                   |
| 10 | experiments, one PR each                                                            | P4, P6, P7    | 4                   |
| 11 | decisions on the unused crates                                                      | A1, A2        | —                   |
| 12 | `docs: remove references to files outside the repository`                           | D1, D2        | —                   |
| 13 | `fix(bench_harness): drop ring_core from the unsafe allowlist`                      | D3            | —                   |

PRs 6 to 10 are not accepted without before/after numbers from the harness after PR 4.
