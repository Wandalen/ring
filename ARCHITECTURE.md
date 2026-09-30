# Architecture

A map for finding your way, not a spec. Each crate's `docs/` holds the detail and the
evidence; this file says where to look and what must stay true. The tier graph and the crate
table are in [`README.md`](README.md).

## Bird's eye view

Many producer threads hand records to one consumer, which drains them in a single total order.
No locks, bounded capacity, exactly-once delivery. One mechanism per crate, so each concern is
verified on its own: 33 `ring_*` crates plus `bench_harness`, which grades them.

Vocabulary used everywhere:

- **`Seq`** — a `u64` sequence number, monotonic and assumed never to wrap (`ring_types::Seq`). Only
  the slot index wraps: `seq & mask` (`ring_index`). Capacity is a power of two, enforced by
  `ring_types::Capacity`.
- **Cursor** — an atomic `Seq` padded to its own 64-byte cache line (`ring_cursor::PaddedCursor`).
- **Gating** — a producer may not claim at or beyond `slowest consumer + capacity`
  (`ring_gating`, `ring_seqno::may_claim`).
- **Claim → write → publish → drain → commit** — the life of one slot.

## How a record moves

**MPSC** (`ring_mpsc`), the ring that matters:

1. **Claim.** `ring_claim::Claimer::claim` — a compare-exchange loop whose condition is the
   gate, re-read on every retry (success `AcqRel`, failure `GATING` = `Acquire`). Sequences are
   unique because the CAS linearizes claims.
2. **Write** through the `Reserved` guard. No synchronization: the slot is exclusively the
   claimant's.
3. **Publish.** Dropping `Reserved` stores the sequence into the slot's stamp with
   `PUBLISH` = `Release`. Producers publish out of order; nothing waits.
4. **Drain.** The consumer scans stamps with `OBSERVE` = `Acquire` and stops at the first
   stamp not equal to the expected sequence. Equality, not `>=`: a stale stamp from the
   previous lap must read as unpublished.
5. **Commit.** Dropping the consumer's `Batch` stores its cursor with `COMMIT` = `Release`,
   which the producers' `GATING` load pairs with. That pair is what stops a producer lapping
   the consumer.

**SPSC** (`ring_spsc`): no read-modify-write anywhere. The producer and consumer cursors hand
off with `HANDOFF` = `Release` against `GATING` = `Acquire`.

`ring_publish`, `ring_barrier` and `ring_consume` implement the generic published-cursor
handshake, loom-modelled in `ring_publish/tests/handshake_test.rs`. Neither production ring uses
them: `ring_mpsc` replaced them with per-slot stamps.

## Code map

| Layer         | Crates                                                                                                                                                           | Notes                                                                                                                                                                                  |
|---------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Vocabulary    | `ring_types`, `ring_config`, `ring_align`, `ring_seqno`, `ring_index`                                                                                            | Ids, errors, policies, construction parameters, the 64-byte line, unfolded `Seq` arithmetic, the fold                                                                                  |
| Atomics       | `ring_atomic`, `ring_cursor`                                                                                                                                     | Every cursor, gate, claim and barrier gets its atomic from `ring_atomic` — the `cfg(loom)` seam. Every method takes an explicit `Ordering`                                             |
| Mechanisms    | `ring_gating`, `ring_claim`, `ring_publish`, `ring_barrier`, `ring_consume`, `ring_wait`, `ring_overflow`, `ring_store`, `ring_slot`, `ring_batch`, `ring_event` | One step each. `ring_store` is a plain `Box<[S]>`; `ring_wait` spins or parks, always bounded                                                                                          |
| Rings         | `ring_spsc`, `ring_mpsc`                                                                                                                                         | The only crates with `unsafe`: one `UnsafeCell` per slot                                                                                                                               |
| Composition   | `ring_core`, `ring_handle`, `ring_factory`, `ring_registry`, `ring_tls`, `ring_flush`, `ring_shutdown`, `ring_poll`                                              | `ring_core` picks a backend from the config (SPSC, MPSC, or `crossbeam` behind a feature) and adds no atomics. `ring_handle` turns it into a `Split` → `Ends` → `(Producer, Consumer)` |
| Observability | `ring_stats`, `ring_trace`, `ring_debug`                                                                                                                         | Counters, an optional operation log, runtime invariant checks                                                                                                                          |
| Verification  | `ring_testkit`, `ring_bench`, `bench_harness`                                                                                                                    | Scripted deterministic runs and loom helpers; the write-path comparison; the stage gates                                                                                               |

**Export contract.** Outside code may depend on five crates only: `ring_types`, `ring_handle`,
`ring_tls`, `ring_flush`, `ring_factory` —
[`export_surface.txt`](bench_harness/gate/declared/ring/export_surface.txt), enforced by G5.
Everything else is internal and may change shape freely.

## Invariants

What a change must not break. Each is argued, with evidence, in the crate docs named after it.

- **Exactly once, in one order.** Every published record is drained once, in `Seq` order
  (`ring_mpsc` invariant/001). Which producer gets which `Seq` is a race; the drained order
  given the publications is not.
- **Single consumer by borrowing.** `Consumer` is neither `Clone` nor `Sync`, and `split` takes
  `&mut`. Trybuild cases in `ring_handle/tests/ui/` pin what must not compile.
- **Orderings are named, never defaulted.** `GATING`, `PUBLISH`, `OBSERVE`, `COMMIT`, `HANDOFF`,
  `OWN` are constants whose docs name the load or store they pair with. None is `SeqCst`
  (`ring_atomic` decisions/001, `ring_mpsc` invariant/002).
- **`unsafe` stays in two crates.** `ring_spsc` and `ring_mpsc`; one operation per block, each
  with `// SAFETY:`, justified in the crate's `docs/workaround/`. The allowlist is G6's input.
- **Eviction is refused.** `OverflowPolicy::DropOldest` fails at construction on the in-house
  backends. Under the default `DropNewest`, `try_push` returns `Ok` for a record it dropped —
  `Ok` is not "kept".
- **`Seq` is assumed never to wrap.** A `u64` at 10⁹ records/s lasts centuries. Saturating
  reads would report a wrapped or corrupted ring as empty and healthy; `ring_debug` exists to
  catch what they hide.
- **Measured before adopted.** Nothing replaces an existing mechanism without winning the
  family benchmark (`ring_mpsc` non_functional_requirement/001). `crossbeam-queue` is an
  interim backend with a written removal condition (`ring_core` workaround/001).

## Cross-cutting concerns

**Testing.** Per crate: `tests/*_test.rs`, doctests, and a manual plan in
`tests/manual/readme.md`. Concurrency: `loom` models (`./verb/loom`), run under
`RUSTFLAGS="--cfg loom"`; test code that is not a model is gated `cfg(not(loom))`.
Compile-fail: trybuild in `ring_handle`, sensitive to the compiler version (hence
`rust-toolchain.toml`) and to the source of every type named in a diagnostic.

**Gates.** `bench_harness/gate/` grades the family: coverage (G1), docs (G2), features (G3),
manual records (G4), export surface (G5), unsafe (G6), mutation (G12, G13), and the doc corpus
(G14–G17, G20, G21). Each gate pairs its assertion with a check that there was something to
measure.
Start at [`bench_harness/gate/readme.md`](bench_harness/gate/readme.md).

**Doc corpus.** Every crate carries `docs/` with 13 typed definitions (algorithm, api,
invariant, pitfall, decisions, …), numbered instances and findings, and shell recipes whose
quoted output G15 re-runs. See `CODESTYLE.md` § Doc corpus.

**Features and `no_std`.** One feature, `crossbeam`, declared in `ring_core` and forwarded up.
`ring_types`, `ring_stats` and `ring_overflow` are `no_std`.

**Tooling.** Every operation is a `verb/` script; each crate has thin `verb/` wrappers over
`verb/_crate_dispatch`. The gate harness expects the checkout to live in a directory named
`ring`.

## Known drift

Extraction from a monorepo left references this repository cannot resolve: module docs cite
`docs/feature/`, `docs/decision/` and `docs/plan/` files that live elsewhere, and the tier
labels in some module docs disagree with `README.md`. `ring_core` is on the unsafe allowlist
without using `unsafe`. Trust manifests and code over prose when they disagree, and fix the
prose.
