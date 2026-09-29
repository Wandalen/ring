# non_functional_requirement

The properties this crate is judged on other than correctness — what it costs, what
it allocates, what it needs from its host, and what it guarantees to a concurrent
reader. All four are decided by one statement, `stats.record_drop( policy, 1 )`,
and that statement belongs to another crate.

The crate's own answer to every one of them is the same: `would_resolve` has none
of them, and it is the half production takes. So the properties recorded here are
real, measured, and currently unexercised — which is the most useful thing to know
about them, and the thing that changes the day a second consumer appears.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_seven_times_the_cost_on_the_half_nobody_ships.md) | Seven Times the Cost, on the Half Nobody Ships | The measured per-call ratio, and who executes which half |
| [002](002_a_core_only_crate_that_never_says_so.md) | A Core-Only Crate That Did Not Say So | Allocation, `core`-only dependence, and the unstated ordering |

## A Number With a Bounded Use

`resolve` costs 4.86 ns against `would_resolve`'s 0.68 ns — a median ratio of 7.00x,
reproduced at 7.02x on a second independent run, nine paired repetitions each. The
whole difference is one relaxed `fetch_add`.

The ratio looks like an argument for the pure half and is not one. `ring_core`
takes the pure half because it holds no `&RingStats`, and 4 ns against a full-ring
event that has already cost a failed push is not a figure anyone should design
around. What the measurement is good for is bounding: the recording half is cheap
enough to ignore, and no shipping code path pays even that.

The number it does not supply is the contended one, which is the case that actually
occurs — a full ring means every producer arrives at the same counter at once. No
crate under `any crate root` has a `benches/` directory, `ring_bench` included.

## Properties Held by Habit

The three-crate chain imports `core::fmt` and `core::sync::atomic` and nothing
else, and declares no `Vec`, `String`, or `Box` across seven files. No file carries
`#![no_std]`, so the property that makes these crates embeddable is enforced by
nobody.

The same shape covers ordering. `resolve`'s doc block spends 28 lines on which
counter moves and none on the fact that it moves with `Ordering::Relaxed` — exact
totals, no consistent snapshot. Its one sentence about what a stats read
"accounts for" is true as a total and silent on the distinction that matters to a
caller reading two counters together.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the one statement that separates the halves, and what it reaches --'
command grep -m1 -F '  stats.record_drop( policy, 1 );' ring_overflow/src/lib.rs
command grep -m1 -F '    counter.fetch_add( n, Ordering::Relaxed );' ring_stats/src/lib.rs
echo '  -- no_std attributes, then benches directories, across the crate tree --'
command grep -rc 'no_std' --include=*.rs ring_overflow/src ring_stats/src ring_types/src | awk -F: '{ s += $2 } END { print "  " s }'
ls -d */benches 2>/dev/null | wc -l
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV49 | `ring_overflow` | **measured cost** | `resolve` costs a median 4.86 ns per call against `would_resolve`'s 0.68 ns — a 7.00x ratio reproduced at 7.02x across two independent nine-repetition paired runs, all of it one relaxed `fetch_add` — and `ring_core` imports and calls only `would_resolve`, so every execution of the expensive half in this workspace happens under `cargo test` and the shipping binary pays none of it |
| OV50 | `ring_overflow` | n/a — coverage | The measured ratio is uncontended, while the condition that triggers `resolve` is by construction every producer arriving at one counter simultaneously — so the number that would inform a design decision is the one not taken, and no crate under `any crate root` carries a `benches/` directory to take it, `ring_bench` included, which never mentions this crate |
| OV51 | `ring_overflow` | **latent hazard** | `ring_overflow`, `ring_stats`, and `ring_types` import only `core::fmt` and `core::sync::atomic` and declare no `Vec`, `String`, or `Box` across seven source files, yet none carried `#![no_std]` — so a `use std::` added to any of them compiled, passed every test, and silently ended the embeddability of all three with nothing in the suite or the build to notice; all three now carry it, proven by a `cargo check` that fails on an inserted `use std::` |
| OV52 | `ring_overflow` | n/a — doc gap | `resolve`'s 28-line doc block contains zero lines mentioning ordering, atomics, threads, or concurrency, while its first statement is a relaxed read-modify-write on shared state — so its cost, its synchronization, and its `u64` wrap behaviour are all inherited from `ring_stats::record_drop` and described nowhere in the crate that exposes them, leaving "a stats read accounts for every full-ring event" to be read as a snapshot guarantee that relaxed ordering does not supply |
