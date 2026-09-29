# Workaround: The Multi-Consumer Path No Ring Uses

### Scope

- **Purpose**: Record that this crate's plural shape is absorbed on behalf of a ring that does not exist yet, and price that absorption against the singular shape the family already has.
- **Responsibility**: Show what the plural costs at N = 1, show the allocation-free alternative already in production next door, and give the two conditions that close this entry.
- **In Scope**: The gap between what the type supports and what anything constructs.
- **Out of Scope**: Whether membership should be mutable — see [`data_structure/001`](../data_structure/001_the_set_that_cannot_grow.md).

### The Constraint

This crate's `GatingSet` specifies a gating **set** — plural by name and by
contract, because a ring with several independent consumers must be gated by
the slowest of them. The crate implements that faithfully.

Nothing constructs one:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'GatingSet::new' ring_*/src/*.rs
# ring_mpsc/src/lib.rs:380:      consumers : GatingSet::new( capacity, 1 ),
```

Live output:

```
ring_claim/src/lib.rs:/// let consumers = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
ring_claim/src/lib.rs:  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 1 );
ring_claim/src/lib.rs:  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 1 );
ring_claim/src/lib.rs:  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 2 );
ring_claim/src/lib.rs:  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 1 );
ring_claim/src/lib.rs:  /// let consumers = GatingSet::new( ring_types::Capacity::new( 4 ).unwrap(), 1 );
ring_claim/src/lib.rs:  /// let consumers = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
ring_claim/src/lib.rs:  /// let consumers = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
ring_gating/src/lib.rs:/// let set = GatingSet::new( Capacity::new( 8 ).unwrap(), 2 );
ring_gating/src/lib.rs:  /// let ungated = GatingSet::new( Capacity::new( 4 ).unwrap(), 0 );
ring_gating/src/lib.rs:  /// assert_eq!( GatingSet::new( Capacity::new( 2 ).unwrap(), 3 ).len(), 3 );
ring_gating/src/lib.rs:  /// assert!( GatingSet::new( Capacity::new( 2 ).unwrap(), 0 ).is_empty() );
ring_gating/src/lib.rs:  /// let set = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
ring_gating/src/lib.rs:  /// assert_eq!( GatingSet::new( Capacity::new( 4 ).unwrap(), 3 ).cursors().len(), 3 );
ring_gating/src/lib.rs:  /// assert_eq!( GatingSet::new( Capacity::new( 16 ).unwrap(), 1 ).capacity().get(), 16 );
ring_gating/src/lib.rs:  /// let set = GatingSet::new( Capacity::new( 8 ).unwrap(), 3 );
ring_gating/src/lib.rs:  /// let set = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
ring_gating/src/lib.rs:  /// let set = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
ring_gating/src/lib.rs:  /// let set = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
ring_gating/src/lib.rs:  /// let set = GatingSet::new( Capacity::new( 8 ).unwrap(), 1 );
ring_mpsc/src/lib.rs:      consumers : GatingSet::new( capacity, 1 ),
```

One production construction in all 33 crates, with a literal `1`. Every
multi-consumer construction in the repository is in a test or a doctest
([`data_structure/001`](../data_structure/001_the_set_that_cannot_grow.md) § G2).

So the crate absorbs a plurality for a consumer that has not been written, and
the production path pays for it on every gating read.

### G19 — The Other Ring Does Not Pay It

The family has two rings. Both have exactly one consumer. They gate through
different mechanisms:

| | `ring_mpsc` | `ring_spsc` |
|--|-------------|-------------|
| Gating state | `GatingSet` + `Claimer` | `CursorPair` |
| Consumers | 1 | 1 |
| Per gating read | one load, `min`, `map_or`, `free_slots` | two loads, `distance_to` |
| Allocations per read | **0** — was **1** until commit `b7e075ca` | **0** |

```rust
// ring_spsc/src/lib.rs — occupancy, stripped of docs
let produced = self.ring.cursors.producer().load( OWN );
let consumed = self.ring.cursors.consumer().load( GATING );
consumed.distance_to( produced )
```

```rust
// ring_cursor::slowest in ring_cursor/src/lib.rs — what ring_mpsc reaches instead
cursors.iter().map( | c | c.load( GATING ) ).min()
```

The two structures are the same three fields:

| `CursorPair` | `Claimer` + `GatingSet` |
|--------------|-------------------------|
| `producer : PaddedCursor` | `Claimer::cursor : PaddedCursor` |
| `consumer : PaddedCursor` | `GatingSet::cursors : Vec< PaddedCursor >` — length 1 |
| `capacity : Capacity` | `GatingSet::capacity : Capacity` |

**Identical content, one heap allocation, one lifetime parameter, and a slice
walk per read apart.** That list used to be longer by a per-read allocation; the
rest of it is unchanged, which is the point. Removing the allocation narrowed
the gap between the two paths without closing it, because the gap was never
mainly about the allocation. The difference is not what the ring needs; it is
which crate the ring was built on.

### What the Plural Costs at N = 1

| Cost | At N = 1 |
|------|---------:|
| A slice walk per gating read | Folding one element |
| A `Vec< PaddedCursor >` held for the set's whole life | Holding one cursor |
| An `Option< Seq >` from the fold | Never `None` — a set with one consumer always has a minimum |
| `map_or( capacity, … )` in `headroom` | The default arm is unreachable |
| `Option` in `slowest` and `limit` | Two `unwrap`/`expect` sites in tests that would not exist |
| The empty-set decision ([`decisions/001`](../decisions/001_capacity_for_an_empty_set.md)) | Argued at length, exercised only by tests |

Four of the five vanish at N = 1. The fifth — the empty-set behaviour — is the
one that would still matter, because `consumers = 0` is a shape a `CursorPair`
cannot express at all.

### Why It Is Still Right to Keep

| Reason | Weight |
|--------|-------:|
| The type is specified as a set, and the crate implements it faithfully | Decisive |
| The fold is shared with `ring_barrier`, whose plural case *is* reachable | Strong |
| A multi-consumer ring is the family's named next step | Moderate |
| Deleting it would move the cost, not remove it | Moderate |

The second is the one that carries real weight today. `ring_cursor::slowest` is
called from both `ring_gating` and `ring_barrier`, and `ring_barrier`'s side genuinely folds
over several dependencies — `ring_consume::Consumer` holds a `Barrier< 'a >` in
production source, and a barrier over three dependencies is a doctest in
`ring_barrier` rather than a hypothetical. So the shared fold is not dead code;
only this crate's *use* of it is single-element.

That is why this is a workaround entry and not a defect: **the plural path is
correct, specified, and shared — it is merely unexercised from here**, and the
cost of the general shape lands on the one caller that does not need it.

### The Condition for Deletion

| Route | Closes this entry | Cost |
|-------|-------------------|------|
| A multi-consumer ring lands | ✅ — the constraint becomes real and the entry is history | The ring's own work |
| The family rules out multi-consumer rings, and `ring_mpsc` moves to `CursorPair` | ✅ — the crate is deleted, not just this entry | Loses the `consumers = 0` shape and `ring_barrier`'s sibling |
| ~~The fold stops allocating~~ — **happened, `b7e075ca`** | ❌ — closed [`non_functional_requirement/001`](../non_functional_requirement/001_every_gating_read_allocates_nothing.md), not this | One `ring_cursor` change, no API break |

The third was worth separating out precisely because it was tempting, and it is
now the one that has actually been taken: removing the allocation made the plural
path nearly free, which removed the *cost* recorded here without resolving the
*question*. The set is still a set no ring constructs plurally, and this entry is
still open — which is exactly what separating the two predicted, and is the
reason it was worth separating.

Between the first two: the second is a real option and nobody has taken it, so
this entry stays open. Recording it is the point — a plural shape with a single
caller is the kind of thing that reads as obviously necessary five years later,
after the reason it was general has been forgotten.

### GT58 — One Consumer Each, Two Mechanisms

```
ring_spsc : CursorPair, two atomic loads, no allocation
ring_mpsc : GatingSet with consumers = 1, one Vec held for the set's life,
            a fold over one — plus a Vec per read, until b7e075ca
```

Both shipping rings have exactly one consumer. They reach that one consumer's
position by different means, and only one of the two pays for plurality. What it
pays has fallen: a heap allocation on every read became a pointer chase to a
one-element buffer allocated once at construction.

**Finding.** Each has exactly one consumer and they gate through different mechanisms: `ring_spsc` reads a `CursorPair` in two loads, `ring_mpsc` folds a one-element `Vec` it holds for the set's whole life — a per-read allocation until `b7e075ca`, a pointer chase since

---

### GT59 — The Expensive Spelling of a Type That Exists

```
Claimer< 'a > { cursor : PaddedCursor, consumers : &'a GatingSet }
  + GatingSet { cursors : Vec< PaddedCursor >, capacity : Capacity }
CursorPair   { producer : PaddedCursor, consumer : PaddedCursor, capacity : Capacity }
```

At the only width anything ships with, the multi-consumer machinery holds
precisely the fields `CursorPair` already holds — one heap allocation and one
lifetime parameter apart.

**Finding.** Together they hold exactly `CursorPair`'s three fields, one heap allocation and one lifetime parameter apart — so the multi-consumer machinery, at the only width anything ships with, is a more expensive spelling of a type the family already has

---

### GT60 — The Plurality Is Exercised Only by Tests

```
production constructions with consumers > 1 : 0
tests constructing multi-consumer sets      : ring_barrier, ring_claim,
                                              ring_gating, ring_publish
```

The fold, the allocation and the minimum-across-a-set rule are the whole reason
this crate is separate from `ring_cursor`. Nothing that ships reaches them.

**Finding.** No shipping ring uses it. `ring_spsc` constructs no `GatingSet` at all and `ring_mpsc` constructs one with `consumers = 1`, so the fold, the allocation and the minimum-across-the-set rule are exercised only by tests

---

### GT61 — The Assumption Lives in a String

```
ring_mpsc/src/lib.rs:550:  fn consumer_cursor( &self ) -> &PaddedCursor
                       553:      .cursor( 0 )
                       554:      .expect( "a gating set built with one consumer has cursor 0" )
```

The one production read of a cursor out of a set states the single-consumer
assumption in an `expect` message. It is the right message, and it is checked at
run time rather than expressed in a type.

**Finding.** `ring_mpsc::consumer_cursor` calls `.cursor( 0 ).expect( "a gating set built with one consumer has cursor 0" )`. The single-consumer assumption is recorded in an `expect` message rather than in a type, and this is the only place in shipping code where a cursor is taken out of a set

---


### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_set_that_cannot_grow.md](../data_structure/001_the_set_that_cannot_grow.md) | G2 — the construction census this entry rests on |
| [../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md](../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md) | The `Vec` the plural shape requires |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_capacity_for_an_empty_set.md](../decisions/001_capacity_for_an_empty_set.md) | The N = 0 shape a `CursorPair` cannot express |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_other_half_of_feature_178.md](../integration/002_the_other_half_of_feature_178.md) | `ring_barrier`, whose plural case is reachable |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_gating_read_allocates_nothing.md](../non_functional_requirement/001_every_gating_read_allocates_nothing.md) | The per-read cost, and the change that removed it |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_predicate_the_quantity_and_the_reason.md](../pattern/002_the_predicate_the_quantity_and_the_reason.md) | `CursorPair`'s two rungs against this crate's three |

### Workarounds

| File | Relationship |
|------|--------------|
| [001_the_manual_check_that_names_a_foreign_method.md](001_the_manual_check_that_names_a_foreign_method.md) | The other absorbed constraint |

### Sources

| File | Relationship |
|------|--------------|
| `ring_mpsc/src/lib.rs:380` | The one production construction, with its literal `1` |
| `ring_spsc/src/lib.rs:609-612` | The allocation-free alternative, in production |
| `ring_cursor/src/lib.rs:247-252` | `CursorPair`'s three fields |
| `ring_claim/src/lib.rs:257-261` | `Claimer`'s two, which complete the same three |
| `ring_consume/src/lib.rs:210-214` | The production holder of a `Barrier`, whose plural case is real |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:133-147` | The plural path, exercised at three positions |
| `tests/gating_test.rs:149-157` | The plural path's whole point, at N = 2 |
| `ring_claim/tests/claim_test.rs:277` | A multi-consumer set built where no ring builds one |
