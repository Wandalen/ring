# Pitfall: The Variant a Default Build Cannot Reach

### Scope

**Purpose:** Record that `Resolution::EvictedOldest` is unreachable in production in
both build configurations, and what that costs the properties stated about it.

**Responsibility:** The default build's rejection of `DropOldest`, the crossbeam
build's early return, and what `would_resolve` can therefore actually receive.

**In Scope:** `ring_core/Cargo.toml:9`;
`ring_core/src/lib.rs:165-168`, `:196`, `:396-404`, `:411`.

**Out of Scope:** The counting of a refusal is
[`pitfall/002`](002_counting_a_refusal_through_the_drop_counter.md). The invariant
this variant is the sole witness for is
[`invariant/002`](../invariant/002_no_resolution_overwrites_unread_data_silently.md).

---

## Both Configurations, and What Each Does First

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the default feature set --'
command grep '^default = ' ring_core/Cargo.toml
echo '  -- what the default constructor does with DropOldest --'
command grep -m1 -A4 -F '    if config.overflow() == OverflowPolicy::DropOldest' ring_core/src/lib.rs
echo '  -- the only constructor that accepts it --'
command grep 'pub fn new_crossbeam' ring_core/src/lib.rs
echo '  -- and what that path does before it could resolve --'
command grep -m1 -B5 -A3 -F '          let _evicted = queue.force_push( record );' ring_core/src/lib.rs
echo '  -- the consumer already said so --'
command grep -m1 -F '  /// `DropOldest` cannot arrive here — [`Ring::new`] refuses it, and' ring_core/src/lib.rs
echo '  -- and now the crate that owns the variant says so too --'
command grep -m1 -A2 -F '  /// **No shipping configuration produces this.**' ring_overflow/src/lib.rs
```

Live output:

```
  -- the default feature set --
default = []
  -- what the default constructor does with DropOldest --
    if config.overflow() == OverflowPolicy::DropOldest
    {
      return Err( RingError::PolicyUnsupported );
    }

  -- the only constructor that accepts it --
  pub fn new_crossbeam( config : &RingConfig ) -> Result< Self, RingError >
  -- and what that path does before it could resolve --
      #[ cfg( feature = "crossbeam" ) ]
      ProducerInner::Crossbeam( queue ) =>
      {
        if self.overflow == OverflowPolicy::DropOldest
        {
          let _evicted = queue.force_push( record );
          return Ok( () );
        }
        queue.push( record )
  -- the consumer already said so --
  /// `DropOldest` cannot arrive here — [`Ring::new`] refuses it, and
  -- and now the crate that owns the variant says so too --
  /// **No shipping configuration produces this.** It requires
  /// [`OverflowPolicy::DropOldest`], which the default build rejects at
  /// construction with `RingError::PolicyUnsupported`, and which the `crossbeam`
```

---

### OV29 — One of Three Variants Is Unreachable in Both Build Configurations

`EvictedOldest` is produced by exactly one arm, for exactly one policy:
`OverflowPolicy::DropOldest`. Neither shipping configuration lets that policy reach
`would_resolve`.

With default features — `default = []`, so `crossbeam` is off — `Ring::new` rejects
`DropOldest` outright at construction with `RingError::PolicyUnsupported`. A ring
configured that way does not exist, so nothing downstream can resolve for it.

With `crossbeam` on, `Ring::new_crossbeam` accepts the policy, and the publish path
then handles it *before* the resolution site: the `Crossbeam` arm tests
`self.overflow == OverflowPolicy::DropOldest`, calls `force_push`, and returns
`Ok( () )` — leaving the `would_resolve` call at `:400` unreached for that policy.

**Finding.** So `would_resolve` sees `DropNewest` or `Fail` in production and never
`DropOldest`, and `EvictedOldest` is a variant no shipping build constructs. The
eviction it names does happen under `crossbeam` — `force_push` performs it — but it
is performed by `crossbeam` and reported by an early return, not by this crate's
type.

`ring_core` already documented half of this ("`DropOldest` cannot arrive here")
for its own `match`. Nothing in `ring_overflow` recorded that one of its three
exported variants is production-dead, and the crate that owns the variant was the
one that could not see why.

The declaration now carries it. `EvictedOldest`'s doc comment names both
configurations and what each does instead, and says why the variant is kept rather
than removed: it is the only value for which `accepted_incoming()` is true, so
deleting it would collapse that predicate to a constant `false` and take the
crate's safety criterion with it.

**Disposition:** applied — a doc comment added at `Resolution::EvictedOldest` in
`ring_overflow/src/lib.rs` recording that the default build rejects
`DropOldest` at construction with `RingError::PolicyUnsupported`, that the
`crossbeam` build handles it with `force_push` and returns before the resolution
site, and why the variant stays. Documentation only — no behaviour changed, and
the 73-test suite plus 20 doctests pass unchanged. Now prints: `  /// **No shipping configuration produces this.**`

---

### OV30 — The Unreachable Variant Is the Sole Witness for Three Stated Properties

`EvictedOldest` is not an idle variant. It is the only value for which
`accepted_incoming()` is true, which makes it:

- the sole satisfier of the antecedent in
  `no_resolution_overwrites_unread_data_silently`, the crate's safety criterion;
- the only case distinguishing `lost_an_item` from `accepted_incoming`, since
  without it the two predicates would partition the outcomes identically;
- one of the two arms `resolve_agrees_with_would_resolve` covers.

**Finding.** The crate's central safety property is therefore demonstrated
entirely on a configuration that no production build accepts. That does not make
the property false — the suite exercises it honestly, and if `DropOldest` were ever
enabled the guarantee would hold. It makes the property untested against anything
that ships.

The compounding risk was in `invariant/002` § OV24: the safety assertion sat behind
`if resolution.accepted_incoming()`, with nothing asserting the body ran. So the one
witness holding that test non-vacuous was also the one variant with no production
path — and if `DropOldest` had been removed from `OverflowPolicy::ALL` on the
grounds that no backend supports it by default, the safety test would have gone
green and empty in the same commit.

Both halves are now closed. The loop counts its satisfactions and asserts the
count, so that removal turns the test red instead of hollow — it was run to
confirm, not assumed. And `Resolution::EvictedOldest` now states which
configuration produces it, so the next reader weighing that removal finds the
reason it would be wrong at the declaration rather than two crates away.

**Disposition:** applied — the witness guard in
`ring_overflow/tests/overflow_test.rs` (§ OV24 above) and the declaration
comment in `ring_overflow/src/lib.rs` (§ OV29 above) together. What remains
true and undispositioned is the underlying fact this finding names: the safety
property is still demonstrated only on a configuration no shipping build accepts.
That is a property of `ring_core`'s backend support, not of this crate, and it
cannot be fixed here — what could be fixed here was the silence about it. Now prints: `  /// **No shipping configuration produces this.**`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/002`](../invariant/002_no_resolution_overwrites_unread_data_silently.md) | The test this variant holds up |
| [`algorithm/002`](../algorithm/002_where_the_third_outcome_goes.md) | The consumer's handling of the outcome |
| [`decisions/001`](../decisions/001_the_fourth_variant_that_is_not_there.md) | Why `accepted_incoming` means what it means |
| [`integration/001`](../integration/001_one_consumer_one_import_one_site.md) | The one call site both paths route around |

### Sources

| Fact | Where |
|------|-------|
| `crossbeam` off by default | `ring_core/Cargo.toml:9` |
| `DropOldest` rejected at construction | `ring_core/src/lib.rs:165-168` |
| The one constructor accepting it | `ring_core/src/lib.rs:196` |
| The early return that precedes resolution | `ring_core/src/lib.rs:396-404` |
| `ring_core`'s own note | `ring_core/src/lib.rs:348` |
| The arm producing the variant | `ring_overflow/src/lib.rs:234` |

### Tests

| Test | Covers |
|------|--------|
| `no_resolution_overwrites_unread_data_silently` | The property this variant solely witnesses |
| `the_two_readings_partition_the_outcomes` | That it is the only `accepted_incoming` case |
| `resolve_agrees_with_would_resolve` | Its arm, in both halves |
| *(to create)* | Nothing records or tests which build configuration can produce it |
