# Data Structure: Three Drop Counters Behind One Enum, One of Which Is Not a Drop

### Scope

**Purpose:** Record how three of the seven counters are addressed by a
three-variant enum rather than by name, and what the third of them actually counts.

**Responsibility:** `dropped_newest`, `dropped_oldest` and `failed`; the two
identical `match` blocks that select between them; and the vocabulary the crate uses
for all three.

**In Scope:** the module comment's per-policy paragraph, the three drop fields, and
the two three-arm matches in `RingStats::record_drop` and `RingStats::dropped` in
`ring_stats/src/lib.rs`; `OverflowPolicy` in `ring_types/src/policy.rs`.

**Out of Scope:** The layout the seven fields sit in is
[`data_structure/001`](001_seven_counters_on_one_line.md). What `dropped_total`
returns to a caller, and what that costs, is
[`pitfall/002`](../pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md).

---

## Three Fields, One Index, and What the Third One Means

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- three of the seven fields, and the odd name --'
command grep -m1 -A2 -F '  dropped_newest : AtomicU64,' ring_stats/src/lib.rs
echo '  -- the same five-line match, in both directions --'
command grep -m1 -A9 -F '  pub fn record_drop( &self, policy : OverflowPolicy, n : u64 )' ring_stats/src/lib.rs
command grep -m1 -A9 -F '  pub fn dropped( &self, policy : OverflowPolicy ) -> u64' ring_stats/src/lib.rs
echo '  -- what ring_types says the third variant is, and the predicate it ships --'
awk '/^  \/\/\/ Publish nothing and return an error, handing the decision to the caller\.$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 1 { print } /^  \/\/\/ Whether a full-ring publish under this policy loses an item without$/{ n2 = NR } n2 && NR >= n2 && NR <= n2 + 1 { print } /^  pub const fn drops_silently\( self \) -> bool$/{ n3 = NR } n3 && NR >= n3 && NR <= n3 + 7 { print }' ring_types/src/policy.rs
echo '  -- what the module comment argues --'
command grep -m1 -A3 -F '//! Drops are counted **per policy**, not in one bucket. A ring that dropped a' ring_stats/src/lib.rs
echo '  -- and the test that pins the conflation --'
command grep -m1 -A11 -F '/// would make a saturated ring look idle.' ring_stats/tests/stats_test.rs | tail -n 11
```

Live output:

```
  -- three of the seven fields, and the odd name --
  dropped_newest : AtomicU64,
  dropped_oldest : AtomicU64,
  failed : AtomicU64,
  -- the same five-line match, in both directions --
  pub fn record_drop( &self, policy : OverflowPolicy, n : u64 )
  {
    let counter = match policy
    {
      OverflowPolicy::DropNewest => &self.dropped_newest,
      OverflowPolicy::DropOldest => &self.dropped_oldest,
      OverflowPolicy::Fail => &self.failed,
    };
    counter.fetch_add( n, Ordering::Relaxed );
  }
  pub fn dropped( &self, policy : OverflowPolicy ) -> u64
  {
    let counter = match policy
    {
      OverflowPolicy::DropNewest => &self.dropped_newest,
      OverflowPolicy::DropOldest => &self.dropped_oldest,
      OverflowPolicy::Fail => &self.failed,
    };
    counter.load( Ordering::Relaxed )
  }
  -- what ring_types says the third variant is, and the predicate it ships --
  /// Publish nothing and return an error, handing the decision to the caller.
  Fail,
  /// Whether a full-ring publish under this policy loses an item without
  /// telling the caller — true for both drop variants.
  pub const fn drops_silently( self ) -> bool
  {
    match self
    {
      Self::DropNewest | Self::DropOldest => true,
      Self::Fail => false,
    }
  }
  -- what the module comment argues --
//! Drops are counted **per policy**, not in one bucket. A ring that dropped a
//! hundred newest items and one that evicted a hundred oldest ones are in
//! completely different trouble, and a single `dropped` counter cannot tell
//! them apart.
  -- and the test that pins the conflation --
#[ test ]
fn refusals_are_counted_even_though_nothing_is_lost()
{
  let stats = RingStats::new();
  for _ in 0..4 { stats.record_drop( OverflowPolicy::Fail, 1 ); }

  assert_eq!( stats.dropped( OverflowPolicy::Fail ), 4 );
  assert_eq!( stats.dropped_total(), 4 );
  assert_eq!( stats.published(), 0, "a refusal publishes nothing" );
  assert_eq!( stats.consumed(), 0 );
}
```

---

### ST11 — The Field Names Record the Distinction the Accessors Erase

Three of the seven counters are not addressed by name from outside. They are
addressed by `OverflowPolicy`, through a five-line `match` that appears twice —
once in `record_drop` returning `&counter` for a `fetch_add`, once in `dropped`
returning the same reference for a `load`. The two blocks are identical line for
line, and both are exhaustive with no `_` arm, so a fourth variant added to
`OverflowPolicy` fails to compile in two places rather than silently falling into a
default.

The field names are where the crate's own understanding shows. Two are
`dropped_newest` and `dropped_oldest`. The third is `failed` — not `dropped_failed`,
not `dropped_fail`. That break in the naming pattern is deliberate and correct:
`OverflowPolicy::Fail` means "Publish nothing and return an error, handing the
decision to the caller", and a caller holding the item it was handed back has not
had anything dropped. `ring_types` makes the same distinction structurally, twice
over: `reports_failure()` is true of exactly that one variant, and
`drops_silently()` is true of exactly the other two.

**Finding.** Everything below the accessors gets this right and everything at the
accessor level puts it back. The public reader is `dropped( OverflowPolicy::Fail )`
— a method whose name asserts a loss, reading a field whose name denies one, keyed
by a variant whose own doc says nothing was lost. There is no way to ask for the
refusal count except by asking how many were dropped.

The duplication is minor and self-correcting; the vocabulary is neither. Three
counters share an index because they share a *trigger* — the ring was full — and the
crate then treats sharing a trigger as sharing an outcome.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats
command grep -F 'refusals, not losses: the item was handed back to the caller' src/lib.rs
```

Live output:

```
  /// refusals, not losses: the item was handed back to the caller, not dropped.
```

**Disposition:** applied — `dropped`'s doc comment now states the exception the
accessor level was erasing: under `OverflowPolicy::Fail` the count is refusals,
not losses, matching the distinction `record_drop`'s own doc, `reports_failure()`
and `drops_silently()` already make everywhere else in the crate and its one
dependency.
Now prints: `refusals, not losses: the item was handed back to the caller`

---

### ST12 — The Module Comment Argues Against a Single Bucket, and the Crate Ships One

The module comment states the design position in the strongest available terms:

> Drops are counted **per policy**, not in one bucket. A ring that dropped a hundred
> newest items and one that evicted a hundred oldest ones are in completely different
> trouble, and a single `dropped` counter cannot tell them apart.

That argument is delivered, and the three counters exist because of it. Then
`dropped_total` folds all three back into exactly the single number the paragraph
says cannot tell those cases apart — and folds in the third counter too, so a hundred
refusals, from which nothing was lost at all, read the same as a hundred evictions.

**Finding.** The comment is not wrong about the design; it is describing the storage
and being read as describing the API. A caller who reads it learns that this crate
refuses to collapse drop causes, and then finds a method that collapses them, sitting
among the seven readers with no caveat on it and `#[ must_use ]` above it like the
rest.

The filter that would fix it is already written, already `const`, already
`#[ must_use ]`, and already in this crate's only dependency:
`OverflowPolicy::drops_silently()` is true for exactly the two variants that lose an
item and false for `Fail`, with a doctest asserting both. `dropped_total` folds over
`ALL` without consulting it.

The suite has taken the other side. `refusals_are_counted_even_though_nothing_is_lost`
records four refusals and then asserts `dropped_total() == 4` — a test whose name
states that nothing was lost, pinning the number that says four things were. Adding
the one-line filter would turn that assertion red. So the conflation is not merely
undocumented; it is held in place by a test that argues against it in its own title,
and any future correction has to change a test before it can change the code.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/002`](../pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md) | The same fold, as a caller reads it, and what it costs |
| [`data_structure/001`](001_seven_counters_on_one_line.md) | The layout these three fields sit in with the other four |
| [`integration/001`](../integration/001_the_write_path_and_two_callers_that_are_not_there.md) | `ring_overflow`, the only production crate that records a drop |
| [`type/002`](../type/002_seven_counters_and_one_width.md) | `OverflowPolicy` as an index, and `u64` as the value |

### Sources

| Fact | Where |
|------|-------|
| The three fields and the break in naming | Census above |
| The duplicated exhaustive `match` | `RingStats::record_drop` and `RingStats::dropped` |
| What `Fail` means | `ring_types/src/policy.rs:112-113` |
| `reports_failure()`, true of one variant | `ring_types/src/policy.rs:153-160` |
| `drops_silently()`, true of the other two | `ring_types/src/policy.rs:178-185` |
| The per-policy argument | The module comment's per-policy paragraph |
| The fold that undoes it | `RingStats::dropped_total` |
| The test pinning the conflation | `ring_stats/tests/stats_test.rs:108-118` |

### Tests

| Test | Covers |
|------|--------|
| `a_drop_lands_under_its_own_policy_only` | That the three counters are independent |
| `the_drop_total_is_the_sum_over_every_policy` | That `dropped_total` sums all three — asserting the behaviour, not questioning it |
| `refusals_are_counted_even_though_nothing_is_lost` | Asserts `dropped_total() == 4` after four refusals — pinning the conflation its own name denies |
| `distinct_policy_counters_do_not_interfere_under_contention` | The three fields under concurrent writers |
| *(to create)* | Nothing asserts that a caller can distinguish a refusal from a loss through the public API |
