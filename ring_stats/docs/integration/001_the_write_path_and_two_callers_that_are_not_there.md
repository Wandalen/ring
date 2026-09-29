# Integration: The Write Path, and Two Callers That Are Not There

### Scope

**Purpose:** Trace who actually writes to a `RingStats` in this workspace, and
compare that against what the crate's own documentation says writes to it.

**Responsibility:** Every production call of a recorder, the dependency edges that
allow them, and the two callers the documentation names that do not exist.

**In Scope:** `ring_overflow::resolve` and `ring_overflow::would_resolve` in
`ring_overflow/src/lib.rs`; the `use ring_overflow::` line and the full-ring
branch in `ring_core/src/lib.rs`; the module documentation and
`RingStats::reset`'s doc in `ring_stats/src/lib.rs`;
`ring_shutdown/Cargo.toml`.

**Out of Scope:** Who reads the counters, and the one crate that does, is
[`integration/002`](002_the_read_path_and_the_removed_edge.md). What `in_flight`
computes for those readers is
[`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md).

---

## Every Write, and the Two Doors That Are Shut

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- who declares ring_stats --'
for f in */Cargo.toml; do if command grep -q '^ring_stats' "$f"; then echo "    ${f#ring/}"; fi; done
echo '  -- every production call of a recorder --'
command grep -r 'stats\.record_' --include=*.rs */src/ | command grep -v '//!' | sed 's|ring/||'
echo '  -- who imports the one that counts --'
command grep -r 'use ring_overflow::' --include=*.rs */src/ */tests/ | sed 's|ring/||'
echo '  -- what ring_core reaches for at the full-ring branch --'
command grep -m1 -B2 -A4 -F '      Err( record ) => match would_resolve( self.overflow )' ring_core/src/lib.rs
echo '  -- and what the crate now says about that, up front --'
command grep -m1 -A4 -F "//! **Nothing on a live ring's write path moves any of these counters, and that" ring_stats/src/lib.rs
echo '  -- and the caller reset names --'
command grep -m1 -A2 -F '  /// Used by `ring_shutdown`'"'"'s reset, so a recycled ring does not carry the' ring_stats/src/lib.rs
printf '    ring_shutdown manifest lines mentioning ring_stats: %s\n' \
  "$( command grep -c 'ring_stats' ring_shutdown/Cargo.toml || true )"
```

Live output:

```
  -- who declares ring_stats --
    ring_bench/Cargo.toml
    ring_overflow/Cargo.toml
  -- every production call of a recorder --
ring_bench/src/lib.rs:  stats.record_claim( received as u64 );
ring_bench/src/lib.rs:  stats.record_publish( received as u64 );
ring_bench/src/lib.rs:  stats.record_consume( received as u64 );
ring_bench/src/lib.rs:  stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 );
ring_overflow/src/lib.rs:  stats.record_drop( policy, 1 );
ring_stats/src/lib.rs:/// stats.record_claim( 4 );
ring_stats/src/lib.rs:/// stats.record_publish( 4 );
ring_stats/src/lib.rs:/// stats.record_drop( OverflowPolicy::DropNewest, 1 );
  -- who imports the one that counts --
ring_core/src/lib.rs:use ring_overflow::{ would_resolve, Resolution };
ring_overflow/src/lib.rs:/// use ring_overflow::Resolution;
ring_overflow/src/lib.rs:  /// use ring_overflow::Resolution;
ring_overflow/src/lib.rs:  /// use ring_overflow::Resolution;
ring_overflow/src/lib.rs:  /// use ring_overflow::Resolution;
ring_overflow/src/lib.rs:/// use ring_overflow::{ resolve, Resolution };
ring_overflow/src/lib.rs:/// use ring_overflow::{ would_resolve, Resolution };
ring_overflow/tests/overflow_test.rs:use ring_overflow::{ resolve, would_resolve, Resolution };
  -- what ring_core reaches for at the full-ring branch --
    {
      Ok( () ) => Ok( () ),
      Err( record ) => match would_resolve( self.overflow )
      {
        Resolution::DroppedIncoming => Ok( () ),
        Resolution::EvictedOldest | Resolution::Refused => Err( record ),
      },
  -- and what the crate now says about that, up front --
//! **Nothing on a live ring's write path moves any of these counters, and that
//! is worth knowing before reading one.** The only production line in the
//! workspace that does is `stats.record_drop( policy, 1 )` inside
//! `ring_overflow::resolve` — and no `src/` anywhere imports `resolve`.
//! `ring_core`, the crate that owns a ring and handles the full-ring case,
  -- and the caller reset names --
    ring_shutdown manifest lines mentioning ring_stats: 0
```

---

### ST17 — The Only Production Recorder Lives in a Function Nothing Imports

Two crates in thirty-three declare `ring_stats`. Outside `ring_bench`, which is a
measurement harness, there is exactly one production line in the workspace that moves
a counter: `stats.record_drop( policy, 1 )` in `ring_overflow/src/lib.rs`.

That line is inside `resolve( policy : OverflowPolicy, stats : &RingStats )`. And
`resolve` is imported by exactly one file anywhere in the workspace —
`ring_overflow/tests/overflow_test.rs`. No `src/` in any crate imports it. Its only
other appearances are in its own doctest.

`ring_core` is the crate that owns a ring and handles the full-ring case. It declares
`ring_overflow` and imports from it — and what it imports is `would_resolve`, the
counter-free half, documented as "The pure half of [`resolve`], for callers deciding
what a policy *would* do". At the branch where a record is actually refused, it calls
that.

**Finding.** This crate's own stated claim for these counters is that they "are the only thing
that distinguishes a ring under mild pressure from one that is quietly discarding
traffic". The path that would distinguish those two cases exists, is correct, is
tested — and is not on the route the ring takes. When `ring_core` drops a record, no
counter moves anywhere.

There is no bug here in the sense of a wrong line. `would_resolve` is documented as
the pure variant and `ring_core` uses it as documented, without a `RingStats` to hand
and without declaring the crate. What was missing is that nothing anywhere recorded
that the counting variant is unreachable from the write path, so the feature read as
delivered and the diagnosis it promises could not happen.

Wiring it is not this crate's edit — it would put a `&RingStats` through
`ring_core`'s full-ring branch, which is that crate's API and that crate's corpus.
What was in reach here is that a reader arriving at `ring_stats` is told, before any
method, that every number it can produce is structural rather than measured, and
which single missing edge is why.

**Disposition:** applied — the crate's module documentation opens with the census
this instance runs: one production recorder, inside `ring_overflow::resolve`, which
no `src/` imports; `ring_core` reaching for `would_resolve` instead; and
`ring_bench` as the one crate whose numbers are real. The census above is unchanged
in what it measures, which is the point — the fix is that
`ring_stats/src/lib.rs` now says it where a reader will hit it first. Now prints: `//! **Nothing on a live ring's write path moves any of these counters, and that`

---

### ST18 — `reset` Names a Caller in a Crate That Does Not Depend on This One

`RingStats::reset`'s doc comment says: "Used by `ring_shutdown`'s reset, so a
recycled ring does not carry the previous world's numbers, per
`docs/feature/184_close_reset_and_drain_all.md`."

`ring_shutdown`'s manifest declares four dependencies — `ring_cursor`, `ring_wait`,
`ring_core`, `ring_types` — and `ring_stats` is not among them. Zero lines of that
manifest mention it. No `.rs` file in `ring_shutdown` contains the string `stats` in
any case. The call the comment describes could not compile.

`RingStats::reset()` is called from three places in the workspace: two tests in
`ring_stats/tests/stats_test.rs` and its own doctest.

**Finding.** This is a stated fact about the system that is false, not an omission —
the same shape as the corrections already carried against `ring_atomic`, where a doc
comment names a test file as the loom seam's only user. A reader auditing shutdown
behaviour finds `reset`'s contract, sees a named caller and a feature reference, and
has no reason to check whether the edge exists.

The correction is one line and belongs on `reset`: either the dependency is missing
from `ring_shutdown` and the feature is unimplemented, or the comment is describing
an intention and should say so. Which of the two is a question for `ring_shutdown`'s
own design, not for this doc — but the sentence as written asserts the first and the
manifest shows the second.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats
command grep -F 'not declare this crate as a dependency, so that call does not exist yet' src/lib.rs
```

Live output:

```
  /// not declare this crate as a dependency, so that call does not exist yet;
```

**Disposition:** applied — `reset`'s doc comment now says "Named for" rather than
"Used by", and adds the fact the census establishes: `ring_shutdown` does not
declare this crate as a dependency, so the described call does not exist yet, and
today `reset` is exercised only by this crate's own tests and doctest — describing
an intention truthfully rather than asserting a caller that cannot compile.
Now prints: `not declare this crate as a dependency, so that call does not exist yet`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/002`](002_the_read_path_and_the_removed_edge.md) | The read side, and the one crate that uses these counters for real |
| [`lifecycle/001`](../lifecycle/001_zero_to_reset_to_zero.md) | The reset this comment describes, measured from outside |
| [`decisions/002`](../decisions/002_per_policy_drop_counters_stated_and_delivered.md) | `reset` as a method the requirement never asked for |
| [`non_functional_requirement/002`](../non_functional_requirement/002_counters_that_must_not_distort_what_they_measure.md) | What a counter on the write path would have to cost |

### Sources

| Fact | Where |
|------|-------|
| Two declaring crates | Census above |
| The single production recorder line | Census above |
| `resolve` imported only by a test | Census above |
| `ring_core` imports `would_resolve`, and its full-ring branch | Census above |
| `would_resolve` documented as the pure half | `ring_overflow::would_resolve`'s doc |
| `reset`'s named caller | Census above |
| The module doc that now states the absence | Census above |
| `ring_shutdown`'s four dependencies | `ring_shutdown/Cargo.toml` |
| The feature's claim for the counters | [`non_functional_requirement/001`](../non_functional_requirement/001_cheap_enough_to_leave_on.md)'s own census |

### Tests

| Test | Covers |
|------|--------|
| `a_drop_is_counted_not_absorbed` | That `resolve` records — in `ring_overflow`'s own suite, the only caller |
| `reset_returns_every_counter_to_the_fresh_state` | `reset`, called from `ring_stats`' own tests |
| `a_reset_set_counts_again` | The same |
| *(not creatable)* | Nothing can assert that a full ring in `ring_core` moves a counter, because the edge that would move one does not exist — what the module doc now states in prose is the thing no test in this crate can reach |
