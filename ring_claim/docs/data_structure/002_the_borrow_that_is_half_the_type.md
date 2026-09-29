# Data Structure: The Borrow That Is Half the Type

### Scope

- **Purpose**: Account for `Claimer`'s second field — an `&'a GatingSet` rather than an owned one — and trace what that single decision did to the public API of the one crate that builds a ring on it.
- **Responsibility**: State why the field cannot be owned, show the three places `ring_mpsc` documents the borrow as the cause of its own shape, and place `Claimer` among the family's lifetime-carrying types.
- **In Scope**: The `'a`, the self-reference it avoids, and its downstream effects.
- **Out of Scope**: The 56 bytes it costs — see [`data_structure/001`](001_sixteen_bytes_and_one_hundred_twenty_eight.md).

### Why It Cannot Be Owned

```rust
pub struct Claimer< 'a >
{
  cursor : PaddedCursor,
  consumers : &'a GatingSet,
}
```

A `GatingSet` holds the consumer cursors. A ring holds both the gating set and
the claimer. If the claimer owned its gating set, a ring would hold the gating
set *twice* — once directly, for consumers to advance, and once inside the
claimer, for producers to read — and the two copies would diverge immediately:
consumers would release slots into a set no producer ever reads.

The alternative to a borrow is therefore not "own it", it is "make the ring
self-referential", and `ring_mpsc:559-562` says exactly that:

> Two steps rather than one because [`Claimer`] borrows the [`GatingSet`] it
> checks headroom against, and a `Ring` holding both would be
> self-referential. `&mut self` is what makes the claim cursor unique: there is
> no moment at which two `Ends` name one ring.

### CL17 — The Borrow Forced a Type to Exist in Another Crate

`ring_mpsc` has an `Ends` type. So do `ring_core` and `ring_handle`. Only one of
the three documents a reason:

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -r -B4 '^pub struct Ends' ring_*/src/*.rs
```

Live output:

```
ring_core/src/lib.rs-}
ring_core/src/lib.rs-
ring_core/src/lib.rs-/// A borrow of a [`Ring`], from which the two ends are taken.
ring_core/src/lib.rs-#[ derive( Debug ) ]
ring_core/src/lib.rs:pub struct Ends< 'a, T >
--
ring_handle/src/lib.rs-}
ring_handle/src/lib.rs-
ring_handle/src/lib.rs-/// The two ends, before they are separated.
ring_handle/src/lib.rs-#[ derive( Debug ) ]
ring_handle/src/lib.rs:pub struct Ends< 'a, T >
--
ring_mpsc/src/lib.rs-/// Holds the [`Claimer`] — and therefore the claim cursor — that every producer
ring_mpsc/src/lib.rs-/// shares. It exists as a separate type only because `Claimer` borrows the
ring_mpsc/src/lib.rs-/// [`GatingSet`] inside the ring; see [`Ring::ends`].
ring_mpsc/src/lib.rs-#[ derive( Debug ) ]
ring_mpsc/src/lib.rs:pub struct Ends< 'a, S >
```

| Crate | Doc comment on `Ends` |
|-------|------------------------|
| `ring_core:238` | "A borrow of a [`Ring`], from which the two ends are taken." |
| `ring_handle:92` | "The two ends, before they are separated." |
| **`ring_mpsc:657-659`** | **"Holds the [`Claimer`] — and therefore the claim cursor — that every producer shares. It exists as a separate type only because `Claimer` borrows the [`GatingSet`] inside the ring; see [`Ring::ends`]."** |

The first two describe what the type *is*. The third states a cause, and the
cause is this crate's field. `ring_mpsc::Ends` is not a design its author wanted
— it is the smallest structure that can hold a `Claimer` and the `&Ring` it
borrows from without either outliving the other:

```rust
pub fn ends( &mut self ) -> Ends< '_, S >     // ring_mpsc:573
{
  let shared : &Self = self;
  Ends { ring : shared, claimer : Claimer::new( &shared.consumers ) }
}
```

The `let shared : &Self = self;` line is the whole trick: reborrow `&mut self`
as `&self`, then hand two copies of that shared borrow into one struct. It
compiles only because both fields borrow from the same shorter-lived `&self`,
and the `&mut` receiver is what guarantees no second `Ends` exists concurrently.

`ring_mpsc` names the borrow as its cause in **three** separate doc comments —
`:293`, `:559-562`, `:657-659` — which is more prose than this crate spends on
the field itself. A field with one line of documentation here has three
paragraphs of consequences one crate away.

### CL18 — Four of the Family's 23 Lifetime-Carrying Types Have No Type Parameter

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
# anchor on any lifetime, not on `'a` — `ring_handle::Drain< 'c, 'a, T >`
# names its first one `'c` and is missed by the narrower pattern
command grep -rE "^pub (struct|enum) [A-Za-z_]+< *'" ring_*/src/*.rs
```

Live output:

```
ring_barrier/src/lib.rs:pub struct Barrier< 'a >
ring_claim/src/lib.rs:pub struct Claimer< 'a >
ring_consume/src/lib.rs:pub struct Consumer< 'a >
ring_core/src/lib.rs:pub struct Ends< 'a, T >
ring_core/src/lib.rs:pub struct Producer< 'a, T >
ring_core/src/lib.rs:pub struct Consumer< 'a, T >
ring_flush/src/lib.rs:pub struct Flusher< 'a, T >
ring_handle/src/lib.rs:pub struct Ends< 'a, T >
ring_handle/src/lib.rs:pub struct Producer< 'a, T >
ring_handle/src/lib.rs:pub struct Consumer< 'a, T >
ring_handle/src/lib.rs:pub struct Drain< 'c, 'a, T >
ring_mpsc/src/lib.rs:pub struct Ends< 'a, S >
ring_mpsc/src/lib.rs:pub struct Producer< 'a, S >
ring_mpsc/src/lib.rs:pub struct Reserved< 'a, S >
ring_mpsc/src/lib.rs:pub struct Consumer< 'a, S >
ring_mpsc/src/lib.rs:pub struct Batch< 'a, S >
ring_shutdown/src/lib.rs:pub struct Stopped< 'a >
ring_shutdown/src/lib.rs:pub struct Guarded< 'a, T >
ring_spsc/src/lib.rs:pub struct Producer< 'a, S >
ring_spsc/src/lib.rs:pub struct Reservation< 'a, S >
ring_spsc/src/lib.rs:pub struct Consumer< 'a, S >
ring_spsc/src/lib.rs:pub struct Batch< 'a, S >
ring_tls/src/lib.rs:pub struct Flush< 'a, T >
```

Twenty-three public types carry a lifetime. Nineteen of them also carry a type
parameter — `Producer< 'a, T >`, `Reserved< 'a, S >`, `Batch< 'a, S >` — because
they borrow a ring *of something*. Four do not:

| Type | Borrows | What it is |
|------|---------|------------|
| `ring_barrier::Barrier< 'a >` | a slice of cursors | a read-only view over dependencies |
| `ring_consume::Consumer< 'a >` | a published cursor | the drain half of the handshake |
| `ring_shutdown::Stopped< 'a >` | a flag | a lifecycle observation |
| **`ring_claim::Claimer< 'a >`** | **a `GatingSet`** | **the claim half of the handshake** |

The four that borrow without a type parameter are the four that operate on
*sequences* rather than on *payloads*. That is the family's Tier 5 boundary drawn
in the type system: below it, everything is `Seq` arithmetic and cursors and no
crate knows what a record is; above it, `ring_mpsc` and `ring_spsc` add the `S`
and the slots.

`Claimer` is the only one of the four whose borrow is documented as having
reshaped a caller's API. The other three are borrowed *by* higher crates without
forcing an intermediate type, because none of them is stored alongside the thing
it borrows from — `Barrier::over( &slice )` is built at the call site and
dropped there, and `ring_consume::Consumer` borrows a cursor that lives outside
the consumer. `Claimer` is the one that a ring wants to *keep*, which is
precisely when a borrow becomes a structural problem rather than a call-site
one.

### What the Borrow Buys

It is worth stating the counterfactual, since the cost is visible and the
benefit is not:

| If `Claimer` owned a `GatingSet` | Consequence |
|----------------------------------|-------------|
| `ring_mpsc::Ends` unnecessary | one fewer public type, `ends()` one step |
| the ring holds two gating sets | consumers advance one, producers read the other |
| back-pressure stops working | producers never see a slot released — every ring wedges at capacity |
| detectable by | a test that fills a ring, drains it, and claims again |

The last row is the reassuring one: the failure is total and immediate, not
subtle. `a_consumer_advancing_releases_exactly_that_many_slots`
(`tests/claim_test.rs:172`) is the test that would fail, and it is a
single-threaded test with no timing dependency. The design that avoids a
self-reference is also the design whose alternative fails loudly.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seventeen_items_and_nothing_that_drops_silently.md](../api/001_seventeen_items_and_nothing_that_drops_silently.md) | `consumers()`, the accessor that hands the borrow back out |

### Data Structures

| File | Relationship |
|------|--------------|
| [001_sixteen_bytes_and_one_hundred_twenty_eight.md](001_sixteen_bytes_and_one_hundred_twenty_eight.md) | What the 8-byte field costs in bytes |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependents_that_split_one_feature.md](../integration/001_two_dependents_that_split_one_feature.md) | The dependent whose API this field shaped |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_claimer_over_a_rings_life.md](../lifecycle/002_the_claimer_over_a_rings_life.md) | What the lifetime does and does not constrain about ordering |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_the_lifetime_on_the_claimer.md](../type/002_the_lifetime_on_the_claimer.md) | `'a` as a type-level obligation, against the family's other 22 |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:238-261` | The type, its doc, and the borrowed field |
| `ring_claim/src/lib.rs:319-333` | `consumers()`, returning `&'a GatingSet` rather than `&GatingSet` |
| `ring_mpsc/src/lib.rs:293` | The first of three references to the borrow as a cause |
| `ring_mpsc/src/lib.rs:557-578` | `Ring::ends`, the reborrow trick, and its stated reason |
| `ring_mpsc/src/lib.rs:655-665` | `Ends`, and the sentence naming this field |
| `ring_core/src/lib.rs:256-263` | An `Ends` with a different reason |
| `ring_handle/src/lib.rs:92-98` | And a third |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:172` — `a_consumer_advancing_releases_exactly_that_many_slots` | The test the owned-set alternative would fail |
| `tests/claim_test.rs:288` — `the_claimer_exposes_the_gate_it_was_built_over` | The borrow, asserted to be the same set that was passed in |
