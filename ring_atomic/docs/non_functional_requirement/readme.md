# non_functional_requirement

The crate states no budget. Not a latency figure, not a throughput figure, not a
ceiling on anything — the source, the readme, and the test file contain zero of
them. What it answers to instead are two acceptance criteria owned by
`bench_harness`, both written as *counts*: "zero atomic operations" and "one fence,
not 64". Both are the right properties to demand. Neither is stated in a unit the
crate's own instrument reports, and one of them names a unit the target hardware
does not emit at all.

So the two instances here measure what nobody specified. What the instrument costs
against the cell it wraps, and what three plausible changes to it are actually
worth — two of them nothing, one of them a factor of 2.5 and a correctness fix
thrown in.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_what_the_instrument_costs.md) | What the Instrument Costs | The shim's overhead, the absent budget, and the three units the `ring_batch` criterion mixes |
| [002](002_the_optimizations_that_are_not.md) | Two Optimizations That Are Not, and One That Is | Weak CAS, padding, and packing the counters into one word |

## Measure in Pairs or Do Not Measure

Every ratio in this definition is a median of nine **paired** ratios: the two
variants run back to back inside one repetition, and the ratio is taken there. The
full min/max spread is printed beside each median, and no claim is made that did not
survive two independent runs.

The method is not ceremony. Taking the ratio of two separately-measured medians —
the obvious approach — produced 1.47× and 0.55× for identical code on consecutive
runs, because this host carries concurrent build load around 18 on 16 cores and the
two halves of each comparison drift apart between measurements. Pairing removes the
drift from the ratio; nothing else here would be trustworthy without it.

## Counting Is Not Timing

A counted call issues two hardware atomics where the production call issues one, but
the cost does not follow the count. `fetch_add` runs about 1.9× — an RMW doubled by
another RMW. `load` runs about 2.7×, because acquire on aarch64 is a load
instruction rather than a barrier, so counting replaces something nearly free with a
full read-modify-write. A read-weighted benchmark is therefore penalised harder than
a claim-weighted one, and the shim's own output reports 1 for both.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every cost figure the crate states --'
command grep -rcE '[0-9]+ *(ns|us|ms)\b|nanosecond|per second|throughput|budget|latency' \
  ring_atomic/src/lib.rs ring_atomic/readme.md ring_atomic/tests/atomic_test.rs
echo '  -- the two criteria it answers to instead --'
sed -n '/^| 175 | Thread-local buffer and flush-into | `ring_tls` | S2 | A `TlsBuffer` accumulates `N` items with zero atomic operations (asserted by a counting allocator\/atomic shim), and one `flush_into` moves all `N` into the ring as a single contiguous claim | `ring_tls\/tests\/tls_test\.rs` |$/p;/^| 177 | Batch claim and batch drain | `ring_batch` | S2 | A claim of 64 slots issues one fence, not 64 (asserted against a counting ordering shim); the 64 sequences returned are contiguous; a batch drain reads them in issue order | `ring_batch\/tests\/batch_test\.rs` |$/p' bench_harness/docs/acceptance/001_feature_reached_tests.md | cut -c1-100
echo '  -- and the machine every ratio here was taken on --'
nproc
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT33 | `ring_atomic` | **measured cost** | A counted `load` costs about 2.7× the production one against `fetch_add`'s 1.9×, because acquire on aarch64 is a load instruction and counting replaces something nearly free with an RMW — so a read-weighted benchmark is distorted harder than a claim-weighted one, and the crate states no budget anywhere against which either could be judged |
| AT34 | `bench_harness` | **misleading doc** | The `ring_batch` criterion asks that a 64-slot claim "issues one fence, not 64", and the claim disassembles to nine instructions with one atomic and zero fence instructions on aarch64 — three units in one sentence (fences said, atomics executed, calls asserted), where "one atomic operation, not 64" would cost four words and remove the reconstruction |
| AT35 | `ring_atomic` | n/a — observation | `compare_exchange_weak` in the family's own retry-loop shape measures at 0.91×–1.15× with every spread straddling 1.00 and two runs disagreeing on the sign — no effect to find, against a fourth trait method every future implementor would owe |
| AT36 | `ring_atomic` | n/a — observation | Packing the four counters into one `AtomicU64` as 16-bit lanes measures ~2.5× faster at 8 and 16 threads *and* makes `counts()` a single load, taking impossible snapshots from 17 to 0 in 500,000 — blocked only by a 65,535 ceiling against a largest-known counted run of 40,000 |
