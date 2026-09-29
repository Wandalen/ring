# Lifecycle: The Teardown Order Nothing Promises

### Scope

**Purpose:** Ask the question L6's drop-counter test cannot answer — in what
order the rings go — and record that the answer is unspecified, varies between
registries in one process, and is warned about nowhere.

**Responsibility:** What the teardown transition guarantees and what it leaves
open; where the crate's one ordering warning is attached; what the drop test
asserts; and who writes the `Drop` that actually runs.

**In Scope:** `ring_registry/src/lib.rs:225-229`;
`ring_registry/docs/lifecycle/001_a_ring_from_registration_to_drop.md:29`,
`:46`, `:102`; `ring_registry/tests/registry_test.rs:223`, `:227`.

**Out of Scope:** The stages themselves are
[`lifecycle/001`](001_a_ring_from_registration_to_drop.md). `names()`'s own order
hazard is
[`algorithm/002`](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md).
The early teardown a discarded `remove` performs is
[`pitfall/002`](../pitfall/002_the_remove_that_drops_a_ring_without_a_word.md).

---

## What Teardown Claims, and Who Supplies It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
l=ring_registry/docs/lifecycle/001_a_ring_from_registration_to_drop.md
echo '  -- what the teardown transition is claimed to guarantee --'
# Anchored at column 0: `lifecycle/001` quotes its own rows inside indented
# regenerate output further down, and an unanchored search finds those copies.
command grep '^| L6 | K2 | K5\|^\*\*L6 is transitive\|^| Registry dropped with rings' "$l" | cut -c1-96 | sed 's/^/    /'
echo '  -- every place the crate warns about ordering, and what it is attached to --'
command grep 'unspecified\|varies between\|order' ring_registry/src/lib.rs | cut -c1-96 | sed 's/^/    /'
echo '  -- what the teardown test actually asserts --'
command grep 'assert_eq!( DROPS' ring_registry/tests/registry_test.rs | cut -c1-88 | sed 's/^/    /'
echo '  -- who writes the Drop that runs at teardown --'
printf '    impl Drop in ring_registry/src: %s\n' \
  "$( command grep -rc 'impl.*Drop for' --include=*.rs ring_registry/src/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
printf '    of 33 ring crates, lib.rs files writing one: %s\n' \
  "$( command grep -rln 'impl.*Drop for' --include=lib.rs ring_*/src/ | wc -l )"
command grep -r 'impl.*Drop for' --include=lib.rs ring_*/src/ | sed 's|ring/||' | cut -c1-72 | sed 's/^/    /'
```

Live output:

```
  -- what the teardown transition is claimed to guarantee --
    | L6 | K2 | K5 | The registry is dropped | Dropped by the registry |
    **L6 is transitive and is the acceptance criterion's third clause.** The registry
    | Registry dropped with rings in it | The registry, transitively | Each `Split` once — no leak
  -- every place the crate warns about ordering, and what it is attached to --
    /// `dropping_the_registry_drops_every_record_still_in_every_ring`. Drop order across rings is u
      /// Every live name, in no particular order.
      /// `HashMap` iteration order is unspecified and varies between one map and
      /// an expected list must sort or collect into a set first. An ordered map
  -- what the teardown test actually asserts --
      assert_eq!( DROPS.load( Ordering::SeqCst ), 0, "something was dropped on the way in" )
      assert_eq!( DROPS.load( Ordering::SeqCst ), 13, "4 + 7 + 2 records were not all droppe
      assert_eq!( DROPS.load( Ordering::SeqCst ), 0, "the registry dropped a ring it no long
      assert_eq!( DROPS.load( Ordering::SeqCst ), 5 );
      assert_eq!( DROPS.load( Ordering::SeqCst ), 0, "nothing dropped on the way in" );
  -- who writes the Drop that runs at teardown --
    impl Drop in ring_registry/src: 0
    of 33 ring crates, lib.rs files writing one: 2
    ring_mpsc/src/lib.rs:impl< S > Drop for Reserved< '_, S >
    ring_mpsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
    ring_spsc/src/lib.rs:impl< S > Drop for Reservation< '_, S >
    ring_spsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
```

## The Order, Recorded

Five rings, registered under five names in a fixed order, one record each,
carrying the name of the ring it sits in. The registry is dropped; the records'
own `Drop` records the sequence. Built and dropped five times in one process:

```rust
// src/bin/teardown_order.rs
fn teardown_order() -> Vec< &'static str >
{
  let mut registry = Registry::new();
  for n in NAMES { registry.register( n, one_record( n ) ).unwrap(); }
  LOG.lock().unwrap().clear();
  drop( registry );
  LOG.lock().unwrap().clone()
}
```

```
    registry 0, dropped: delta gamma alpha epsilon beta
    registry 1, dropped: epsilon beta gamma delta alpha
    registry 2, dropped: gamma epsilon alpha delta beta
    registry 3, dropped: alpha gamma beta delta epsilon
    registry 4, dropped: alpha gamma epsilon delta beta
    five registries built identically, five drops: 5 distinct orders
    the registration order was:            alpha beta gamma delta epsilon
```

Five distinct out of five, in both runs. Across the ten registries built between
the two runs, not one tore down in registration order.

---

### RG31 — The Records Are Destroyed in an Order Nothing Specifies, and the Crate's Only Ordering Warning Is Attached to the Method Teardown Never Calls

`names`'s doc is where this crate says what it knows about `HashMap` ordering:
"Every live name, in no particular order… `HashMap` iteration order is unspecified
and varies between runs, so a caller that compares must sort or collect into a set
first." Correct, if understated
([RG3](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md)), and
scoped to `names()`.

Teardown inherits the same nondeterminism and never touches `names()`. Dropping
a `HashMap` walks its buckets, so the rings — and every record still unread in
them — are destroyed in bucket order. Measured, five registries built by one
function from one fixed name list produce five different teardown orders, and
across ten samples none matched registration order. A caller reaches this without
writing an iteration, without calling any method at all: the hazard fires on the
closing brace.

Whether that matters is entirely a question about `T`, and in this family `Drop`
is not an idle trait. The two crates that write one — `ring_mpsc` and
`ring_spsc`, four impls between them — use it to *publish*: `Reserved::drop`
performs "the one `Release` store the whole protocol turns on". A record type
whose destructor flushes, closes, decrements or logs is exactly the kind this
family builds, and for such a `T`, "every record is dropped" and "the records are
dropped in a defined order" are different guarantees. The crate makes the first
and is silent on the second.

**Finding.** Recorded as an undocumented hazard on the one transition the crate
calls unconditional. The cheap repair is one sentence — on `Registry`'s own type
doc, not on `names`, since that is where a reader looking for teardown behaviour
goes — saying the drop order across rings is unspecified and varies, and that a
caller needing a defined order must `remove` in that order first. The expensive
one is
[RG4](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md)'s
`BTreeMap`, which would make teardown order registration-independent but
*defined*, and delete this hazard along with the `names()` one. It is worth noting
that the same data-structure change closes both, because neither alone would
justify it and together they are the second and third reason.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_registry
command grep 'Drop order across rings' src/lib.rs
```

Live output:

```
/// `dropping_the_registry_drops_every_record_still_in_every_ring`. Drop order across rings is unspecified and varies — a caller needing one must `remove` them in that order first.
```

**Disposition:** applied — took the cheap repair, on `Registry`'s own type doc
rather than on `names`. The sentence was appended to the doc comment's existing
last line rather than inserted as a new one, so `remove` and every method after
it in `src/lib.rs` kept its line number — verified by re-checking `get_mut`
(189), `remove` (199), `contains` (206) and `names` (232) all unchanged, which
matters because this file is cited by line number from over a dozen other
`ring_registry` doc files. The expensive repair (RG4's `BTreeMap`) is left
alone; nothing here forecloses it. Full crate suite re-verified passing after
this and every other `src/lib.rs` edit made against this crate this pass
(`cargo test --all-features -p ring_registry`, 2026-09-04): 15 integration
tests, 1 doctest, 0 failures. Now prints:
`Drop order across rings is unspecified and varies — a caller needing one must`

---

### RG32 — The Registry Writes No `Drop`, So Every Teardown Property It States Belongs to Someone Else

`lifecycle/001` is confident about L6: "The registry drops each `Split< T >`, each
`Split` drops its `Ring< T >`, and each `Ring` drops the records still unread in
it", and the cleanup table promises "Each `Split` once — no leak, no double drop".
Both are true. Neither is this crate's code. `command grep` finds zero
`impl Drop` in `ring_registry/src`; the destructor that runs is
`HashMap`'s, then `Split`'s, then `Ring`'s.

That is the right design — a registry that wrote its own `Drop` would be adding a
step to a chain that already works — and it has one consequence the file does not
draw. The properties the crate can claim about teardown are exactly the
properties `HashMap` guarantees, and completeness is on that list while ordering
is explicitly not. So RG31's gap is not an oversight in the documentation of a
behaviour the crate chose; it is the shape of a behaviour the crate inherited
whole, including the part `std` declines to promise.

The test mirrors it precisely. `dropping_the_registry_drops_every_record_still_in_every_ring`
asserts `DROPS == 0` before and `DROPS == 13` after, for rings of 4, 7 and 2
records. A total is the one observation that cannot detect a reordering: every
permutation of those thirteen drops sums to thirteen. The test is a good guard for
the property it was written for and structurally blind to the one beside it.

**Finding.** Recorded as an inherited-guarantee boundary the file should draw
rather than a defect. Two lines in `lifecycle/001`'s Dependencies table — which
already carries "Nothing supplies the drop" — naming *which* teardown properties
come with `HashMap` and which do not, put RG31's hazard where a reader meets it.
And it locates the only real lever: the crate cannot fix ordering by writing a
`Drop`, because it would still be iterating the same unordered map
([RG8](../api/002_the_receiver_split_and_the_sweep_it_forbids.md) — there is no
accessor that would let it impose one). The lever is the map type, which is
[`data_structure/002`](../data_structure/002_the_only_map_in_thirty_three_crates.md)'s
subject and, on this evidence, one argument stronger than it was.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/001`](001_a_ring_from_registration_to_drop.md) | L6 and the cleanup table — the claims this tests |
| [`algorithm/002`](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md) | The documented half of the same nondeterminism |
| [`data_structure/002`](../data_structure/002_the_only_map_in_thirty_three_crates.md) | The map choice both hazards trace back to |
| [`api/002`](../api/002_the_receiver_split_and_the_sweep_it_forbids.md) | Why the crate could not impose an order even with a `Drop` |
| [`pitfall/002`](../pitfall/002_the_remove_that_drops_a_ring_without_a_word.md) | The teardown that happens early, and silently |

### Sources

| Fact | Where |
|------|-------|
| L6's transitive claim | `.../lifecycle/001_a_ring_from_registration_to_drop.md:29`, `:46`, `:102` |
| The crate's only ordering warning, on `names` | `ring_registry/src/lib.rs:225-229` |
| The drop test asserts totals | `ring_registry/tests/registry_test.rs:223`, `:227` |
| Zero `Drop` impls in `src` | Census above |
| Two crates of 33 write one, four impls | Census above |
| Five distinct teardown orders, twice | Probe above, two runs |

### Tests

| Test | Covers |
|------|--------|
| `dropping_the_registry_drops_every_record_still_in_every_ring` | L6's total, and nothing about its order |
| `a_removed_ring_carries_its_records_to_its_new_owner` | The one teardown the caller does control |
| `names_lists_every_live_name` | The documented half of the same nondeterminism |
