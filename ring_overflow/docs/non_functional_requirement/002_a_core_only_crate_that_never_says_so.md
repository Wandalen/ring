# Non-Functional Requirement: A Core-Only Crate That Did Not Say So

### Scope

**Purpose:** Record the non-functional properties this crate holds — allocation
freedom, `core`-only dependence, memory ordering — against which of them it states,
tests, or enforces.

**Responsibility:** What the three-crate chain imports, what it allocates, and what
`resolve`'s doc block says about the atomic it performs.

**In Scope:** `resolve`'s doc block and its `record_drop` statement in
`ring_overflow/src/lib.rs`; `RingStats::record_drop` in
`ring_stats/src/lib.rs`; the `use` sets of `ring_overflow`,
`ring_stats`, and `ring_types`.

**Out of Scope:** The measured cost of that atomic is
[`nfr/001`](001_seven_times_the_cost_on_the_half_nobody_ships.md) § OV49. That the
counter moves before the `Err` is
[`pitfall/002`](../pitfall/002_counting_a_refusal_through_the_drop_counter.md)
§ OV32.

---

## What the Chain Imports, Allocates, and States

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- no_std attributes across the three crates in this chain --'
# anchored on the attribute, not the string: the comments explaining the
# attribute mention `no_std` too, and counting those inflated this census by two
command grep -rc '^#!\[ *no_std' --include=*.rs ring_overflow/src ring_stats/src ring_types/src | awk -F: '{ s += $2 } END { print "  " s }'
echo '  -- what those three import instead of std --'
command grep -rh 'use core::' --include=*.rs ring_overflow/src ring_stats/src ring_types/src | sort -u
echo '  -- heap-typed declarations across the same three --'
command grep -r 'Vec<\|String\|Box<' --include=*.rs ring_overflow/src ring_stats/src ring_types/src | wc -l
echo '  -- lines in resolve doc block mentioning ordering, atomics or threads --'
command grep -m1 -A27 -F '/// Apply `policy` to a publish that found the ring full, recording the outcome' ring_overflow/src/lib.rs | command grep -ci 'order\|atomic\|relax\|thread\|concurren' || true
```

Live output:

```
  -- no_std attributes across the three crates in this chain --
  3
  -- what those three import instead of std --
use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};
  -- heap-typed declarations across the same three --
1
  -- lines in resolve doc block mentioning ordering, atomics or threads --
0
```

---

### OV51 — The Whole Chain Is `core`-Only and Allocation-Free, and Now Says So on Every Crate

`ring_overflow` and both crates it depends on import from `core` and from nowhere
else: two `use core::` lines between them, `fmt` and
`sync::atomic::{ AtomicU64, Ordering }`. Across all seven source files there is not
one `Vec<`, `String`, or `Box<`. The property a ring buffer most wants from its
overflow policy — that deciding what to do about a full ring neither allocates nor
drags in an operating system — holds completely.

**Finding.** No file in the chain carried `#![no_std]`. The census returned zero
across all three crates, so the property was held by habit rather than by the
attribute that would check it. A `use std::collections::HashMap` added to any of
them compiled, passed every test, and silently converted three embeddable crates
into three that were not, with nothing in the suite or the build to notice.

All three now carry the attribute — the census above reads 3, one per crate. It
had to be all three rather than the one crate this finding is filed against: a
`no_std` crate depending on a `std` one is a `std` crate, so the property is only
worth asserting transitively. `ring_types` is the one that mattered most; 33
crates depend on it, and a `use std::` there would have been a `std` dependency
for the entire family. Nothing else changed — no `use` was rewritten and no
signature moved, because the property was already held. What changed is that
losing it is now a build error rather than a discovery months later by whoever
first tries an embedded target.

**Disposition:** applied — `#![ no_std ]` added to `ring_overflow`,
`ring_stats`, and `ring_types`, each with a comment stating why the assertion is
transitive. Proven able to fail rather than assumed: inserting
`use std::collections::HashMap;` into `ring_types/src/error.rs` and running
`cargo check -p ring_types` now exits 101 with `error[E0433]: cannot find module
or crate std in this scope`, where before the attribute it compiled clean. The
73-test suite across the three crates, their 20 doctests, and clippy at
`-D warnings` all pass unchanged. Now prints: `  3`

---

### OV52 — Every Non-Functional Property of `resolve` Belongs to a Function Two Crates Away, and Its Doc Names None of Them

`resolve`'s doc block runs 28 lines — a summary, a worked doctest asserting all
three policies and all three counters, and an `# Errors` section. Across those 28
lines there are zero mentions of ordering, atomics, threads, or concurrency.

Its body is two statements, and the first is
`stats.record_drop( policy, 1 )` → `counter.fetch_add( n, Ordering::Relaxed )`.

**Finding.** So the function's cost (4.86 ns against the pure half's 0.68 ns,
[`nfr/001`](001_seven_times_the_cost_on_the_half_nobody_ships.md) § OV49), its
synchronization (relaxed — a per-location total order and no happens-before edge
with anything else), and its saturation behaviour (a `u64` counter that wraps
rather than saturates) are all properties of `record_drop`, inherited whole and
described nowhere in the crate that exposes them.

What that costs a reader is precision on the one sentence the doc does offer:
"a stats read accounts for every full-ring event, not only the lossy ones." That
is exactly true of the eventual total, which relaxed ordering does guarantee — no
increment is lost and the count never moves backwards. It carries nothing about
reading two counters together, or about reading a counter against the state of the
ring it describes, because relaxed ordering supplies no edge between them. A
caller who reads the sentence as a statement about a consistent snapshot has read
more into it than it says, and there is no line nearby to correct them.

The correction is one clause on the summary rather than a new section — that the
counter is written with relaxed ordering, so totals are exact and cross-counter
readings are not. `ring_stats` § ST46 records the same tear from the reader's side;
this is the write that produces it, and the point at which a caller decides to
trust a reading.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`nfr/001`](001_seven_times_the_cost_on_the_half_nobody_ships.md) | The measured cost of the same inherited atomic |
| [`workaround/001`](../workaround/001_the_recorder_forecloses_const.md) | The other thing that atomic forecloses |
| [`pitfall/002`](../pitfall/002_counting_a_refusal_through_the_drop_counter.md) | What the counter records that it should not |
| [`integration/002`](../integration/002_the_stats_edge_and_the_function_nobody_imports.md) | The dependency edge these properties arrive over |

### Sources

| Fact | Where |
|------|-------|
| Three `no_std` attributes, one per crate in the chain | Census above |
| The two `use core::` lines, and no `std` | Census above |
| Zero heap-typed declarations across seven files | Census above |
| Zero ordering or threading lines in `resolve`'s doc | Census above, `resolve`'s doc block |
| The relaxed read-modify-write inherited | `ring_stats/src/lib.rs:293` |

### Tests

| Test | Covers |
|------|--------|
| `exactly_one_counter_moves_per_call` | The write whose ordering is unstated |
| `would_resolve_touches_no_counters` | The half that inherits none of these properties |
| `exactly_one_counter_moves_per_call` | The counting claim the summary makes |
