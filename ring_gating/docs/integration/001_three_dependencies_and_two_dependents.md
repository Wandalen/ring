# Integration: Three Dependencies and Two Dependents

### Scope

- **Purpose**: Account for every edge into and out of this crate, and establish what each one is actually carrying.
- **Responsibility**: Name what the crate takes from each of its three dependencies, what its two real dependents do with it, and why the other two dependents are `[dev-dependencies]`.
- **In Scope**: The `Cargo.toml` graph and the imports that justify it.
- **Out of Scope**: `ring_barrier` as a design counterpart — see [`002`](002_the_other_half_of_feature_178.md).

### Downward: Three Dependencies, Four Import Sites

```toml
# ring_gating/Cargo.toml
[dependencies]
ring_types  = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }
ring_seqno    = { path = "../ring_seqno" }
```

Every use of all three, with doc lines stripped:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs \
  | grep -E 'ring_types|ring_cursor|ring_seqno'
# 42:use ring_cursor::PaddedCursor;
# 43:use ring_types::{ Capacity, RingError, Seq };
# 199:    ring_cursor::slowest( &self.cursors )
# 226:      ring_seqno::free_slots( producer, slowest, self.capacity )
```

Live output:

```
use ring_cursor::PaddedCursor;
use ring_types::{Capacity, RingError, Seq};
        ring_cursor::slowest(&self.cursors)
            ring_seqno::free_slots(producer, slowest, self.capacity)
```

**Four lines.** That is the crate's entire coupling to everything below it.

| Dependency | Taken | Used at | If it were dropped |
|------------|-------|---------|--------------------|
| `ring_types` | `Capacity`, `RingError`, `Seq` | Every signature | The crate has no vocabulary — three types would have to be redeclared |
| `ring_cursor` | `PaddedCursor`, `slowest` | The field type, and the fold | The crate would perform its own atomic loads, and would have to name an ordering — [`invariant/002`](../invariant/002_this_crate_names_no_ordering.md) |
| `ring_seqno` | `free_slots` | One line in `headroom` | The crate would own a saturating subtraction and a `u64 → usize` narrowing |

Each dependency is load-bearing for exactly one thing, and the middle row is the
one the crate's own manual checks defend: `ring_cursor::slowest` is what keeps
`Ordering::` out of this file entirely.

`ring_seqno` is the thinnest edge — one call, one line — and the most consequential
to remove, because `free_slots` is where the family decides that distance
saturates rather than signs. Five steps of the chain and what each adds are traced
in [`algorithm/001`](../algorithm/001_headroom_in_two_delegations.md).

### Upward: Four Dependents, Two of Them Real

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rl 'ring_gating *=' */Cargo.toml | grep -v ring_gating/
# ring_barrier  ring_claim  ring_mpsc  ring_publish
```

Live output:

```
ring_barrier/Cargo.toml
ring_claim/Cargo.toml
ring_mpsc/Cargo.toml
ring_publish/Cargo.toml
```

| Crate | Section | Relationship |
|-------|---------|--------------|
| `ring_mpsc` | `[dependencies]` | Owns one — `consumers : GatingSet` |
| `ring_claim` | `[dependencies]` | Borrows one — `consumers : &'a GatingSet` |
| `ring_barrier` | `[dev-dependencies]` | Tests against one; the library never names it |
| `ring_publish` | `[dev-dependencies]` | Builds one to drive its handshake tests |

#### `ring_mpsc` — the owner

```rust
// ring_mpsc/src/lib.rs, doc lines stripped
36:  consumers : GatingSet,
68:      consumers : GatingSet::new( capacity, 1 ),
84:    self.consumers.capacity()
137:    self.consumers
146:    Ends { ring : shared, claimer : Claimer::new( &shared.consumers ) }
```

Five lines, and the shape they describe is the whole reason the type is owned
rather than borrowed: the ring holds the set, and hands a borrow of it to a
`Claimer` at `:146`. `ring_mpsc`'s own doc names the constraint —

> Two steps rather than one because `Claimer` borrows the `GatingSet` it gates
> on
>
> — `ring_mpsc/src/lib.rs:559`

— which is the lifetime consequence of `ring_claim` taking `&'a GatingSet`
instead of an `Arc`. See
[`data_structure/002`](../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md).

#### `ring_claim` — the borrower

`Claimer` stores `consumers : &'a GatingSet` (`:253`) and reads through it in
three places: `headroom()` at `:353`, and the loop conditions of `claim` and
`claim_up_to` at `:404` and `:449`. It never calls `check`, `admits` or `limit` —
see [`decisions/002`](../decisions/002_a_result_rather_than_a_bool.md).

#### The two dev-dependents

`ring_barrier` states its reason in the manifest rather than leaving it implicit:

```toml
# Tests only. Three of them assert the relationship between this crate's answers
# and `ring_gating`'s over one set of cursors — see the test file's own header on
# why a stand-in would assert nothing. The library itself never names it.
[dev-dependencies]
```

That is the right shape for a dev-dependency comment: it says what the tests do
with it, and it says what would be lost by faking it. The library's own module
doc makes the same point from the other side — *"A `Barrier` borrows
`&[PaddedCursor]`, not a `ring_gating::GatingSet`"* (`:32`).

`ring_publish` carries a heavier dev set — `ring_claim`, `ring_consume`,
`ring_barrier` and `ring_gating` together — because its handshake tests assemble
the whole producer/consumer round trip. Eleven `GatingSet::new` calls appear
there, every one with `consumers = 1`. Its header names the reason the real type
is used:

> the **consumer position** lives in the producer's `GatingSet`, which is what
>
> — `ring_publish/tests/handshake_test.rs:23`

### What the Shape Says

| Observation | Reading |
|-------------|---------|
| 3 down, 4 up, 4 import lines | A joint, not a component |
| 2 of 4 dependents are dev-only | The type is a better test fixture than it is a dependency |
| No dependent calls `check` | The crate's richest method is unreached — [`decisions/002`](../decisions/002_a_result_rather_than_a_bool.md) |
| No dependent constructs `consumers > 1` in production | The multi-consumer path has tests and no callers — [`data_structure/001`](../data_structure/001_the_set_that_cannot_grow.md) |

The second row is the one worth sitting with. Both dev-dependents pull this crate
in specifically because a stand-in would weaken what their tests assert —
`ring_barrier` wants the *same cursors* two folds disagree or agree over, and
`ring_publish` wants the *real* position a producer gates on. That is a real use,
and it is not a use the dependency graph's `[dependencies]` section records.

### GT19 — Two of the Four Dependents Say Why in a Comment

```
ring_barrier/Cargo.toml:13: # Tests only. Three of them assert the relationship
ring_barrier/Cargo.toml:15: # why a stand-in would assert nothing. The library
ring_barrier/Cargo.toml:16: [dev-dependencies]
```

A dependent count is a weak signal on its own — it does not distinguish a
library edge from a test edge. Here one of the manifests closes that gap
itself.

**Finding.** Two of them are `[dev-dependencies]`, and `ring_barrier`'s `Cargo.toml` says why in a comment rather than leaving it to be inferred — "the library itself never names it"

---

### GT20 — One Edge for One Function

```
226:      ring_seqno::free_slots( producer, slowest, self.capacity )
      ...the only ring_seqno item this crate names
      ring_cursor -> ring_seqno as well, so the edge is also transitive
```

Three declared dependencies, and the narrowest of them is reached from a single
line for a single function that would arrive anyway through a sibling.

**Finding.** Declared directly and also reachable through `ring_cursor`, which depends on it too. The direct edge exists for one function, `free_slots`, called from one line of `headroom` — the narrowest justification for a manifest entry anywhere in this crate

---


### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_set_that_cannot_grow.md](../data_structure/001_the_set_that_cannot_grow.md) | The four source files that name the type, and the two that decline to use it |
| [../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md](../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md) | The ownership choice `ring_mpsc` and `ring_claim` sit on either side of |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_result_rather_than_a_bool.md](../decisions/002_a_result_rather_than_a_bool.md) | What `ring_claim` does instead of calling `check` |

### Integrations

| File | Relationship |
|------|--------------|
| [002_the_other_half_of_feature_178.md](002_the_other_half_of_feature_178.md) | The dev-dependent that is also the design counterpart |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_this_crate_names_no_ordering.md](../invariant/002_this_crate_names_no_ordering.md) | What the `ring_cursor` edge is protecting |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/Cargo.toml` | Three dependencies |
| `ring_mpsc/src/lib.rs:337, 380, 577` | The owner's field, construction and hand-off |
| `ring_claim/src/lib.rs:257` | The borrow |
| `ring_barrier/Cargo.toml` § `[dev-dependencies]` | The comment that explains a test-only edge |
| `ring_publish/Cargo.toml` § `[dev-dependencies]` | Four ring crates assembled for one round-trip test |

### Tests

| File | Relationship |
|------|--------------|
| `ring_barrier/tests/barrier_test.rs:399-412` | Two folds over one set of cursors, required to agree |
| `ring_claim/tests/claim_test.rs:273-282` | A borrowed set read back through the borrower |
| `ring_publish/tests/handshake_test.rs:86,177` | The set behind an `Arc`, driven across threads |
