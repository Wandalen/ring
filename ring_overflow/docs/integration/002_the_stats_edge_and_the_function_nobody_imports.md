# Integration: The Stats Edge and the Function Nobody Imports

### Scope

**Purpose:** Record that this crate's whole `ring_stats` dependency exists for one
statement inside one function, and that the function has no production caller.

**Responsibility:** The `ring_stats` edge, the single `record_drop` call it carries,
and what reaches that call in a shipping build.

**In Scope:** `ring_overflow/Cargo.toml`;
`ring_overflow/src/lib.rs:31`, `:192`, `:199`; `ring_core/src/lib.rs:80`.

**Out of Scope:** The consumer's import list is
[`integration/001`](001_one_consumer_one_import_one_site.md). What `record_drop`
does to the counters it writes is
[`pitfall/002`](../pitfall/002_counting_a_refusal_through_the_drop_counter.md).

---

## What the Edge Carries

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every use of the stats dependency in this crate --'
command grep 'ring_stats\|RingStats\|record_drop' ring_overflow/src/lib.rs
echo '  -- the one line that actually writes a counter --'
command grep '^  stats\.' ring_overflow/src/lib.rs
echo '  -- and how often the sole consumer names the function holding it --'
command grep -c 'ring_overflow::resolve\|[( ]resolve(' ring_core/src/lib.rs || true
echo '  -- against how often it names the pure half --'
command grep -c 'would_resolve' ring_core/src/lib.rs || true
```

Live output:

```
  -- every use of the stats dependency in this crate --
//! Depends on `ring_types` and `ring_stats`.
// `no_std` here is an assertion, not a convenience. This crate, `ring_stats`,
use ring_stats::RingStats;
/// so a `RingStats` read in that build reflects nothing about full-ring
/// use ring_stats::RingStats;
/// let stats = RingStats::new();
pub fn resolve( policy : OverflowPolicy, stats : &RingStats ) -> Result< Resolution, RingError >
  stats.record_drop( policy, 1 );
/// **Also the right half for a caller that holds no `&RingStats` at all.**
  -- the one line that actually writes a counter --
  stats.record_drop( policy, 1 );
  -- and how often the sole consumer names the function holding it --
0
  -- against how often it names the pure half --
2
```

---

### OV19 — A Whole Dependency for One Statement Nothing Production Reaches

The `ring_stats` edge exists for `src/lib.rs:199` — `stats.record_drop( policy, 1 )`
— and for nothing else. Of the six lines that mention the dependency, one is the
module doc, one is the `use`, two are doctest scaffolding, one is `resolve`'s
signature, and one is the statement itself.

`resolve` is the only function taking a `&RingStats`, and `ring_core` — the sole
crate declaring `ring_overflow` — names it zero times. It names `would_resolve`
twice.

**Finding.** So the dependency is real, correctly declared, correctly used, and
traversed in a shipping build by nothing. Every execution of `record_drop` that
this crate is responsible for happens under `cargo test`: three in the doctest at
`:170-172`, the rest in `overflow_test.rs`.

This is the writer's half of a gap already recorded from the reader's side.
`ring_stats` records that its counters have no production writer at all
([`ring_stats` § ST17](../../../ring_stats/docs/integration/001_the_write_path_and_two_callers_that_are_not_there.md));
this instance records the same break from the crate that owns the writer — the
edge is here, the statement is here, and the consumer reaches neither.

**Disposition:** declined — the edge cannot be wired from either end without
breaking a stated property of a third crate. `ring_core/src/lib.rs` carries
a section headed "This crate adds no atomic of its own", whose reason is that
`ring_spsc` asserts zero read-modify-writes across a run and every publish reaches
it through a `ring_core` method: a counter added there would break that assertion
with nothing in `ring_spsc`'s own dependency tree to blame. `ring_stats` is
"deliberately not a dependency" of `ring_core` in the same paragraph. So switching
the call site from `would_resolve` to `resolve` is not a one-line change — it
would require `ring_core` to hold a `&RingStats` and would put a `fetch_add` on
the publish path that `ring_spsc`'s benchmark is written to catch. The correct
consumer for `resolve` is an instrumented wrapper above `ring_core` that does not
exist yet, and inventing one to give this edge a caller would be building an
unneeded crate to satisfy a census. Recorded and left standing: the dependency is
correct, the statement is correct, and the gap is a fact about what has been built
so far rather than a defect in this crate.

---

### OV20 — The Crate Documents a Contract About Counters That Only Its Own Tests Exercise

`resolve`'s doc comment makes a specific promise: "Exactly one counter is
incremented per call, whichever branch is taken — so a stats read accounts for
every full-ring event, not only the lossy ones."

That is a statement about what a `RingStats` reading means, and it is the strongest
claim this crate makes. It is also a claim about a system that, in production, has
no writer — because the caller that produces real full-ring events uses
`would_resolve`, which touches nothing.

**Finding.** The promise is kept by the code and unreachable by the system. A
reader of `RingStats` in a shipping build sees zeros from this path regardless of
how many full-ring events occurred, and the sentence guaranteeing otherwise sits
on a function nothing in that build calls.

The two crates are individually consistent and jointly hollow: `ring_overflow`
guarantees it records every event it is asked to handle, `ring_stats` guarantees
it reports what it was told, and no production path connects them. Neither crate's
documentation can detect this, because the missing piece is in a third —
`ring_core`'s choice of which half to import
([`integration/001`](001_one_consumer_one_import_one_site.md) § OV18).

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -A5 -F 'This guarantee is about calls to' ring_overflow/src/lib.rs
```

Live output:

```
/// **This guarantee is about calls to `resolve`, not about a build.** No
/// production caller in this workspace currently calls it — the crate's own
/// sole consumer takes [`would_resolve`] instead, which touches no counter —
/// so a `RingStats` read in that build reflects nothing about full-ring
/// events regardless of how many occurred. The promise above holds fully; it
/// is a contract on the function, not a claim about which build exercises it.
```

**Disposition:** applied — `resolve`'s doc comment in
`ring_overflow/src/lib.rs` now states directly that the "exactly one
counter is incremented per call" guarantee is a contract on the function, not
a claim about which build exercises it, and names the current reality: no
production caller in this workspace calls `resolve` today, so a `RingStats`
read in that build reflects nothing about full-ring events regardless of how
many occurred. Now prints: `This guarantee is about calls to`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/001`](001_one_consumer_one_import_one_site.md) | The import that makes this edge dead |
| [`pitfall/002`](../pitfall/002_counting_a_refusal_through_the_drop_counter.md) | What the statement counts when it does run |
| [`algorithm/001`](../algorithm/001_two_mappings_over_three_policies.md) | The one statement that separates the two mappings |
| [`workaround/001`](../workaround/001_the_recorder_forecloses_const.md) | What that statement costs the signature |

### Sources

| Fact | Where |
|------|-------|
| The declared dependency | `ring_overflow/Cargo.toml` |
| Every mention of it in the crate | Census above |
| The single counter write | `ring_overflow/src/lib.rs:199` |
| The contract it is meant to keep | `ring_overflow/src/lib.rs:152-154` |
| The consumer's import list | `ring_core/src/lib.rs:80` |

### Tests

| Test | Covers |
|------|--------|
| `exactly_one_counter_moves_per_call` | The documented contract, in-crate |
| `counts_accumulate_and_stay_separated` | That repeated calls stay per-policy |
| `a_refusal_is_counted_even_though_it_loses_nothing` | The `Fail` branch's counter write |
| `would_resolve_touches_no_counters` | The half the consumer actually uses |
