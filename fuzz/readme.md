# Fuzzing the lock-free core

`loom` checks thread interleavings; it says nothing about the input space —
capacity edges, claim/publish sequences that wrap the ring, partial batch
grants. These targets feed byte streams of ring operations into `ring_spsc`
and `ring_mpsc` and check every step against a FIFO oracle, so a wrong `Full`,
a lost record, or a reordered drain fails loudly.

## Reviewer quick start

Prerequisites: Linux, the nightly toolchain, `cargo install cargo-fuzz`.

```sh
cd fuzz
cargo fuzz build                       # both targets compile with instrumentation
cp seeds/spsc_ops/* corpus/spsc_ops/   # CI does this; local runs need it once
cargo fuzz run spsc_ops -- -max_total_time=60
```

A green run ends with `Done ...` and no `artifacts/` output. A red run
prints the failing input, saves it under `fuzz/artifacts/<target>/`, and
exits non-zero — see "When it finds a crash" below.

To replay one input (a seed, or a crash artifact) without fuzzing:

```sh
./target/x86_64-unknown-linux-gnu/release/spsc_ops fuzz/seeds/spsc_ops/overfill
```

## Mechanism

Each input is a byte stream. The first byte picks the capacity
(`byte % 6` over 1/2/4/8/32/128); the rest is read op by op, up to 1024 ops,
wrapping past the end as zeroes so long inputs exercise wrap-around instead
of stopping. Every run ends with a full drain that must empty the oracle.

`spsc_ops` (`byte % 5`):

| Op | Bytes after | Behaviour |
|----|-------------|-----------|
| 0  | u64 payload | `try_push`: lands and is remembered, or hands the record back with `Full` exactly when full |
| 1  | u64 payload | `claim` + `set` + drop through `Reservation`: same contract via the guard path |
| 2  | — | `drain` everything, compare FIFO against the oracle |
| 3  | 1 byte `max` | `drain_up_to(max)`, compare the taken prefix |
| 4  | — | refusal probe: a full ring must refuse the claim |

`mpsc_ops` (`byte % 6`): op 0 is `push`, op 1 is `push_batch` (width
`1 + byte % 16`; the granted prefix lands in order, the rest stays in the
vector), op 2 is `claim_batch` + hand write of every granted slot + drop
(the grant may be narrower than asked — adaptive partial grants on a
nearly-full ring), op 3 is `claim` + write + drop, ops 4/5 are the drains.

Invariants, checked on every step: FIFO order against a `VecDeque` oracle,
`Full` if and only if the oracle holds `capacity` records, refused pushes
keep their record (spsc) or vector (batch) untouched, and the final drain
empties the oracle. All single-threaded: the contended interleavings belong
to loom.

## Targets

| Target      | What it drives |
|-------------|----------------|
| `spsc_ops`  | `try_push`, `claim` + write + drop through `Reservation`, `drain`, `drain_up_to` |
| `mpsc_ops`  | `push`, `push_batch`, `claim_batch` + hand write + drop, `claim` + write + drop, `drain`, `drain_up_to` |

## When it finds a crash

1. Reproduce: run the saved artifact as a single input (command above) —
   it must fail the same way deterministically.
2. Minimise: libFuzzer already shrinks the crashing input before saving it;
   if it is still large, re-run with `-merge=1` over the corpus.
3. Regress: copy the minimised input into `fuzz/seeds/<target>/` with a name
   saying what it caught — a regression seed is the proof this harness can
   fail.

## CI mapping

`.github/workflows/fuzz.yml` runs `short` (120 s/target) on pull requests
touching the fuzzed crates and `soak` (600 s/target) on the nightly schedule
plus manual dispatch. `soak` shows SKIPPED on pull requests by design — the
short job is the PR signal. See the workflow header for the rationale.

## Seeds

`fuzz/seeds/<target>/` is checked in and copied into libfuzzer's
`fuzz/corpus/<target>/` before a run: hand-built edge cases
(`push_drain`, `overfill`, `claim_batch`, …) plus one fixed-seed
pseudo-random blob per target for breadth.

> Windows note: the prebuilt toolchains ship no AddressSanitizer runtime
> for MSVC, so `cargo fuzz run` (asan by default) cannot start there and
> `-s none` does not link either (missing `sancov` runtime). Build with
> `cargo fuzz build` to check the targets compile, run them for real on
> Linux — which is what CI does.
