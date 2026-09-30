# Pitfall: Counting a Refusal Through the Drop Counter

### Scope

**Purpose:** Record that `resolve` records a drop before it branches, so the one
outcome the crate defines as losing nothing is counted through a method named
`record_drop` and summed into a total documented as items lost.

**Responsibility:** The unconditional record, the counter a refusal lands in, and
what a caller receiving `Err` can no longer assume.

**In Scope:** `ring_overflow/src/lib.rs:116`, `:199-205`;
`ring_stats/src/lib.rs:291`, `:376`, `:378`.

**Out of Scope:** That no production path reaches this code is
[`integration/002`](../integration/002_the_stats_edge_and_the_function_nobody_imports.md).
The unreachable variant is
[`pitfall/001`](001_the_variant_a_default_build_cannot_reach.md).

---

## What Runs Before the Branch, and Where It Lands

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the record runs before the branch, on every policy --'
command grep -m1 -A6 -F '  stats.record_drop( policy, 1 );' ring_overflow/src/lib.rs
echo '  -- what this crate says a refusal costs --'
command grep -m1 -F '      Self::DroppedIncoming | Self::EvictedOldest => true,' ring_overflow/src/lib.rs
command grep 'a_refusal_loses_nothing' ring_overflow/tests/overflow_test.rs
echo '  -- and what the Errors section now says about the write --'
command grep -m1 -A1 -F '/// **This error path is not effect-free.** The counter is incremented first, so' ring_overflow/src/lib.rs
echo '  -- which counter the refusal lands in --'
command grep -m1  -A6 -F '  pub fn record_drop( &self, policy : OverflowPolicy, n : u64 )' ring_stats/src/lib.rs | tail -n 1
echo '  -- and what sums it --'
sed -n '/^  \/\/\/ Items lost across every policy\.$/p;/^    OverflowPolicy::ALL\.iter()\.map( | p | self\.dropped( \*p ) )\.sum()$/p' ring_stats/src/lib.rs
```

Live output:

```
  -- the record runs before the branch, on every policy --
  stats.record_drop( policy, 1 );
  match policy
  {
    OverflowPolicy::DropNewest => Ok( Resolution::DroppedIncoming ),
    OverflowPolicy::DropOldest => Ok( Resolution::EvictedOldest ),
    OverflowPolicy::Fail => Err( RingError::Full ),
  }
  -- what this crate says a refusal costs --
      Self::DroppedIncoming | Self::EvictedOldest => true,
fn a_refusal_loses_nothing()
  -- and what the Errors section now says about the write --
/// **This error path is not effect-free.** The counter is incremented first, so
/// `Err` returns with shared state already mutated — against the usual reading
  -- which counter the refusal lands in --
      OverflowPolicy::Fail => &self.failed,
  -- and what sums it --
```

---

### OV31 — The Crate Records a Drop for the Outcome It Defines as Losing Nothing

`lost_an_item` excludes `Refused` — the pattern names `DroppedIncoming` and
`EvictedOldest` and nothing else — and a test called `a_refusal_loses_nothing`
asserts it. That is the crate's own position: refusing is not a loss.

`resolve` nonetheless calls `stats.record_drop( policy, 1 )` for
`OverflowPolicy::Fail`, and calls it before the `match` that would distinguish
the case.

**Finding.** The event lands in `RingStats`' `failed` counter, which is a separate
field from `dropped_newest` and `dropped_oldest` — so `dropped( Fail )` reports
refusals distinctly and is not itself misleading. The conflation happens one level
up: `dropped_total()` sums all three and is documented as "Items lost across every
policy."

So within a single crate, `Resolution::Refused.lost_an_item()` is `false` and the
same event increments a counter that a documented total calls a loss. Both
statements are reachable from the same `resolve` call, and neither doc comment
mentions the other. `ring_stats` records the reading side of this
([`ring_stats` § ST43](../../../ring_stats/docs/pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md));
what belongs here is that the write is issued by this crate, through a method whose
name already asserts the interpretation.

The naming is the fixable part. `record_drop` is the only recorder available, and
`resolve` is right to call it — the alternative is not recording refusals at all,
which loses real information. What was missing was a sentence saying that the
call counts an event rather than a loss, and that `Fail`'s share is retrievable
separately; it now sits at `:194-198`, directly above the call.

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -A5 -F 'Counts an event, not a loss' ring_overflow/src/lib.rs
```

Live output:

```
    // Counts an event, not a loss: this runs on every policy, including `Fail`,
    // whose own share is retrievable separately via `stats.dropped(
    // OverflowPolicy::Fail )` — `Resolution::Refused.lost_an_item()` is `false`,
    // so this call and that predicate answer different questions about the
    // same arrival.
    stats.record_drop(policy, 1);
```

**Disposition:** applied — a source comment now sits directly above
`stats.record_drop( policy, 1 )` in `ring_overflow/src/lib.rs`, stating
exactly what this instance named as missing: the call counts an event rather
than a loss, and `Fail`'s share is retrievable separately via
`stats.dropped( OverflowPolicy::Fail )`. Now prints: `Counts an event, not a
loss`

**Correction (2026-09-28):** the Finding above states "neither doc comment
mentions the other." `RingStats::dropped_total`'s own doc comment has since
grown the qualifier its own [`ring_stats` § ST43](../../../ring_stats/docs/pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md) disposition records: "Items lost across every policy — except
`OverflowPolicy::Fail`, whose count is refusals folded in here anyway: the
record-and-read pattern gives every policy the same drop verb, so this sum
cannot tell a loss from a refusal." One of the two doc comments now names the
conflation; only `record_drop`'s own call site in this crate still doesn't,
which is the half this instance's own fix, above, already closes. The census
above also lost its last line for an unrelated reason: `dropped_total`'s fold
became `.fold( 0, u64::saturating_add )` under
`Fix(ring_stats_dropped_total_overflow)`, an overflow guard unrelated to this
finding, so the recipe's two exact-text searches — the old one-line doc
sentence, the old `.sum()` call — now match nothing in `ring_stats/src/lib.rs`,
and that section prints no line beneath its own heading.

---

### OV32 — `resolve` Mutates Shared State on the Path That Returns `Err`

The record is the function's first statement. Every `resolve` call performs an
atomic `fetch_add` before deciding anything, including the call that returns
`Err( RingError::Full )`.

A caller reading only the signature — `Result< Resolution, RingError >` — has no
signal that the error path has already written. The usual reading of a `Result`-
returning function is that `Err` means the operation did not take effect; here it
means the operation took its only effect and then reported that the caller must
decide what to do next.

**Finding.** This is correct behaviour and an unusual contract, documented in the
right place but not in the terms a caller would search for. `resolve`'s doc comment
states "Exactly one counter is incremented per call, whichever branch is taken",
which does say it — under a heading about counting completeness, not about error
semantics. The `# Errors` section immediately below says only that
`RingError::Full` arises "under `OverflowPolicy::Fail`, which is the policy's
entire purpose rather than a failure of this function", and does not mention that
a counter moved.

The consequence for a caller: retrying a refused publish by calling `resolve` again
double-counts the same full-ring event, and nothing in the type or the `# Errors`
section warned against it. There is no production caller today
([`integration/002`](../integration/002_the_stats_edge_and_the_function_nobody_imports.md)),
so the hazard was entirely prospective — which is the cheapest moment to write the
sentence.

The sentence is now in the `# Errors` section, in the terms a caller reading it
would search for: the error path is not effect-free, `resolve` is not idempotent,
and a retry double-counts. It ends by naming the correct pattern rather than only
the trap — decide with `would_resolve` as often as needed, record with `resolve`
once.

**Disposition:** applied — `resolve`'s `# Errors` section in
`ring_overflow/src/lib.rs` extended to state the counter write, the
non-idempotence, and the retry-safe pairing, and
`retrying_a_refusal_counts_the_same_arrival_twice` added to
`ring_overflow/tests/overflow_test.rs` pinning the behaviour so that
"fixing" the ordering to make `Err` effect-free fails rather than silently
changing what every stats reader is counting. The test also asserts eight
`would_resolve` calls move no counter, which is the pairing the doc now
recommends. Full suite green at 73 tests. Now prints: `      Self::DroppedIncoming | Self::EvictedOldest => true,`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/001`](001_the_variant_a_default_build_cannot_reach.md) | The other outcome with a gap between name and reach |
| [`integration/002`](../integration/002_the_stats_edge_and_the_function_nobody_imports.md) | Why neither hazard fires today |
| [`algorithm/001`](../algorithm/001_two_mappings_over_three_policies.md) | The statement's placement before the branch |
| [`decisions/002`](../decisions/002_fail_returns_an_error_not_a_resolution.md) | Why `Fail` leaves through the error channel |

### Sources

| Fact | Where |
|------|-------|
| The record, before the branch | `ring_overflow/src/lib.rs:199` |
| `lost_an_item` excluding `Refused` | `ring_overflow/src/lib.rs:116` |
| The counting contract | `ring_overflow/src/lib.rs:152-154` |
| The `# Errors` section | `ring_overflow/src/lib.rs:179-182` |
| The counter a refusal lands in | `ring_stats/src/lib.rs:291` |
| The total that sums it | `ring_stats/src/lib.rs:376`, `:378` |

### Tests

| Test | Covers |
|------|--------|
| `a_refusal_is_counted_even_though_it_loses_nothing` | The behaviour itself, named exactly |
| `a_refusal_loses_nothing` | The type's opposing reading |
| `exactly_one_counter_moves_per_call` | That the write happens on every branch |
| `counts_accumulate_and_stay_separated` | That per-policy counters stay distinct |
