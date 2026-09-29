# Integration: The Other Half (`ring_barrier`)

### Scope

- **Purpose**: Set `GatingSet` beside `ring_barrier::Barrier` — the two crates that implement one feature from opposite sides — and draw out what the symmetry explains.
- **Responsibility**: Give the method-for-method correspondence, name the three places the symmetry breaks, and trace each break to a decision rather than an oversight.
- **In Scope**: The relationship between the two crates.
- **Out of Scope**: `ring_barrier`'s own internals, which that crate documents.

### One Structure, Two Crates

The structure a producer consults before claiming: the set of consumer cursors
that gate it, and the minimum position across that set. With one consumer it is
a single read; with several it is a minimum, and the slowest consumer is what
bounds the producer. Making the gating set explicit is what lets a ring gain a
second consumer without the producer logic changing.

`ring_gating` is the producer's half; `ring_barrier` is the consumer's.
`ring_barrier` states the division in its own module doc, and adds the part
that matters:

> What they *do* share is the fold over a slice of cursors, which is
> `ring_cursor::slowest` and lives in neither of them.

### G9 — Two of Three Dependencies Shared, and the Third Is the Split

| | `ring_gating` | `ring_barrier` |
|---|---|---|
| Tier | 4 | 5 |
| Dep 1 | `ring_types` | `ring_types` |
| Dep 2 | `ring_cursor` | `ring_cursor` |
| Dep 3 | **`ring_seqno`** | **`ring_wait`** |

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^\[dependencies\]/,/^\[/p' ring_{gating,barrier}/Cargo.toml
grep -m1 -oE 'Tier [0-9]+' ring_{gating,barrier}/src/lib.rs
```

Live output:

```
[dependencies]
ring_types = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }
ring_seqno = { path = "../ring_seqno" }

[lints]
[dependencies]
ring_types = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }
ring_wait = { path = "../ring_wait" }

# Tests only. Three of them assert the relationship between this crate's answers
# and `ring_gating`'s over one set of cursors — see the test file's own header on
# why a stand-in would assert nothing. The library itself never names it.
[dev-dependencies]
ring_gating/src/lib.rs:Tier 4
ring_barrier/src/lib.rs:Tier 5
```

The two shared edges are the shared substance — the vocabulary, and the fold.
The third edge is the entire difference in job:

| Crate | Third dep | What it buys | Consequence |
|-------|-----------|--------------|-------------|
| `ring_gating` | `ring_seqno` | `free_slots` — a saturating subtraction against a capacity | Can compute a bound; **cannot wait** |
| `ring_barrier` | `ring_wait` | `wait_until` — a spin/park budget | Can wait; **has no capacity to bound against** |

A producer must not block inside the gate — it has a claim loop of its own to
run. A consumer with nothing to read has nothing else to do. So one crate got
arithmetic and the other got a wait loop, and neither took both.

### The Method Correspondence

| Question | `GatingSet` | `Barrier` |
|----------|-------------|-----------|
| Where is the rearmost cursor? | `slowest() -> Option< Seq >` | `frontier() -> Option< Seq >` |
| How many? | `headroom( producer ) -> usize` | `available( from ) -> u64` |
| May I? | `admits( producer, count : usize ) -> bool` | `admits( from, count : u64 ) -> bool` |
| …and if not, why? | `check( … ) -> Result< (), RingError >` | — |
| Where will I stop? | `limit() -> Option< Seq >` | — |
| Wait until I may | — | `wait_for( from, count, kind, spins ) -> Result< Seq, RingError >` |

Both name their predicate `admits`, and both spell it `count <= <quantity>` over
their own quantity method. That is the correspondence working: the same question,
asked from two sides, gets the same name and the same shape.

### Three Places the Symmetry Breaks

#### 1. The empty fold resolves opposite ways

| Crate | Empty → | Line |
|-------|--------:|------|
| `GatingSet::headroom` | `capacity` | `ring_gating:224` |
| `Barrier::available` | `0` | `ring_barrier:218` |

```rust
// ring_barrier/src/lib.rs:216-219
pub fn available( &self, from : Seq ) -> u64
{
  self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
}
```

Same `map_or`, opposite identity. A producer with no consumers is unconstrained;
a consumer with no dependencies has nothing available. Both are the *safe*
answer for their own side, which is why `ring_cursor::slowest` returns an
`Option` and refuses to pick — argued in
[`decisions/001`](../decisions/001_capacity_for_an_empty_set.md).

#### 2. One owns its cursors, the other borrows a bare slice

`Barrier::over` takes `&'a [ PaddedCursor ]`, and `ring_barrier` explains at
length why it is not a `GatingSet`:

> Taking the aggregate instead would have meant a publisher's cursor could never
> be depended on at all, since there is no way to move an existing cursor into a
> set that owns its own. That is not a hypothetical: it is what made the
> four-operation handshake in `ring_publish/tests/handshake_test.rs` unwireable
> until this signature changed.

**A consumer's dependencies are wherever they happen to live**; a producer's
gating set is a thing it owns. So the asymmetry is not a style difference — the
borrowing signature is what makes the family composable at all, and it was
arrived at by a wiring failure rather than by design taste.

The `ring_barrier` test that closes the loop builds one of each over the same
memory:

```rust
// ring_barrier/tests/barrier_test.rs:437-449
fn a_barrier_over_a_gating_set_reads_that_set_and_not_a_copy()
{
  let set = GatingSet::new( cap( 8 ), 2 );
  let barrier = Barrier::over( set.cursors() );

  set.cursor( 0 ).unwrap().store( Seq( 5 ), Ordering::Release );
  set.cursor( 1 ).unwrap().store( Seq( 3 ), Ordering::Release );

  assert_eq!( barrier.frontier(), set.slowest() );
  assert_eq!( barrier.frontier(), Some( Seq( 3 ) ) );
}
```

Two crates' independent folds over one set of cursors, required to agree. It is
the only external test that constrains `GatingSet::slowest` at all, and it is one
of only three multi-consumer constructions outside this crate.

#### 3. G10 — Owning the cursors costs two `const fn`s

| | `GatingSet` | `Barrier` |
|---|:---:|:---:|
| Public methods | 11 | 9 |
| `const fn` | 1 | 4 |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -cE '^\s*pub const fn ' ring_{gating,barrier}/src/lib.rs
# ring_gating/src/lib.rs:1
# ring_barrier/src/lib.rs:4
```

Live output:

```
ring_gating/src/lib.rs:1
ring_barrier/src/lib.rs:4
```

`Barrier`'s four are `over`, `dependencies`, `len` and `is_empty` — every one of
them either returns the borrowed slice unchanged or calls a slice method, and
slice methods are const-stable. `GatingSet`'s counterparts hold a `Vec`, and two
of them therefore cannot be const at all:

| Method | `Barrier` | `GatingSet` | Why |
|--------|:---------:|:-----------:|-----|
| `len` | `const` | not | Could be — `Vec::len` is const-stable, the attribute is simply absent |
| `is_empty` | `const` | not | Same |
| `dependencies` / `cursors` | `const` | **cannot be** | `&self.v` deref-coerces `Vec → [ T ]`, and `Deref` is not a const trait |
| `cursor` | not | **cannot be** | `Vec::get` is `[ T ]::get` through the same coercion |

Compiled against a two-field stand-in on `rustc 1.97.1`:

```
error: `Deref` is not yet stable as a const trait
error[E0658]: cannot perform conditionally-const deref coercion on `Vec<u32>` in constant functions
  = note: see issue #143874 <https://github.com/rust-lang/rust/issues/143874>
```

So the ownership decision has a cost nobody chose: **two methods that are `const`
in the borrowing sibling cannot be here, and two more are non-const by omission.**
Nothing depends on any of the four today. Recorded because the difference reads
as a deliberate contrast between the two crates and is not one — it is one
language limitation plus one oversight. The `const`-ness census from this crate's
side is in [`api/001`](../api/001_eleven_methods_over_one_owned_vec.md).

### What the Comparison Settles

| Question | Answer |
|----------|--------|
| Should `GatingSet` gain a `wait_for`? | No — a producer must not block inside the gate; that is what the `ring_wait` edge marks |
| Should `Barrier` gain a `check`? | Its `wait_for` already returns a `Result`, and `RingError::Empty` is its only refusal — there is nothing to distinguish |
| Should either take the other's type? | No, and `ring_barrier`'s doc records the wiring failure that proved it |
| Is the shared fold worth its indirection? | Yes — it is the one thing both halves must agree on, and one test asserts they do |

### GT21 — The Split Is Legible in the Manifests

```
ring_gating : ring_types, ring_cursor, ring_seqno
ring_barrier : ring_types, ring_cursor, ring_wait
```

Two crates implementing one feature share two dependencies and differ in the
third. The one they differ on is exactly the thing that separates them.

**Finding.** They share two of three dependencies; the third is `ring_seqno` here and `ring_wait` there, which is exactly the split between computing a bound and waiting on one

---

### GT22 — Peers on Paper, Parent and Child in the Graph

```
GatingSet / Barrier : gating set and sequence barrier, co-equal
Cargo.toml          : ring_barrier -> ring_gating   ( dev-dependencies )
                       ring_gating  -> ring_barrier   does not exist
```

The design pairs two halves of one mechanism. The build graph describes a
direction, and only the build graph is checkable.

**Finding.** `ring_barrier` is also a dependent of this crate, and the edge runs one way. The two halves are peers by design and parent-and-child in the build graph, and only the second relationship is checkable from the source

---

### GT63 — What Owning Costs in `const`

```
ring_gating  : 1 const fn of 11 methods
ring_barrier : 4 const fn of  9 methods
```

The two halves differ in one structural decision — own the cursors or borrow
them — and the difference shows up somewhere neither instance would look for
it.

**Finding.** Owning a `Vec` instead of borrowing a slice costs `const fn`s the sibling gets for free: 1 of 11 methods is `const` here, 4 of 9 in `ring_barrier`. The ownership decision that makes this half safe to share is also what puts most of its surface out of reach of a `const` context

---



### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_check_orders_its_two_refusals.md](../algorithm/002_check_orders_its_two_refusals.md) | The zero-count asymmetry between the two halves |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_eleven_methods_over_one_owned_vec.md](../api/001_eleven_methods_over_one_owned_vec.md) | G5 — the `const fn` census from this side |
| [../api/002_the_reading_that_returns_a_position.md](../api/002_the_reading_that_returns_a_position.md) | `limit` and `frontier`, the two position-valued readings |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md](../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md) | The ownership decision this comparison prices |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_capacity_for_an_empty_set.md](../decisions/001_capacity_for_an_empty_set.md) | The opposite identity, argued from this side |

### Integrations

| File | Relationship |
|------|--------------|
| [001_three_dependencies_and_two_dependents.md](001_three_dependencies_and_two_dependents.md) | The dev-dependency edge that makes the shared test possible |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:30-44` | Why the dependencies are a bare slice |
| `ring_barrier/src/lib.rs:216-219` | `available` — the opposite identity |
| `ring_barrier/src/lib.rs:282-287` | `wait_for` — the capability this crate lacks |
| `ring_barrier/Cargo.toml` | `ring_wait` where this crate has `ring_seqno` |

### Tests

| File | Relationship |
|------|--------------|
| `ring_barrier/tests/barrier_test.rs:437-449` | The two folds, required to agree |
| `ring_barrier/tests/barrier_test.rs:235-246` | A dependency-free barrier, the mirror of an ungated set |
| `tests/gating_test.rs:277-287` | The zero-count case, where the two halves differ |
