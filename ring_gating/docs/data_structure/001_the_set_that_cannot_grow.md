# Data Structure: The Set That Cannot Grow

### Scope

- **Purpose**: Establish that a `GatingSet`'s membership is fixed for its whole life, show that the family constructs exactly one and gives it a single consumer, and assess whether either is a problem.
- **Responsibility**: Prove both facts mechanically, trace what would have to change for a consumer to join or leave a live ring, and separate the part that is a real gap from the part that is correct-for-now.
- **In Scope**: Findings **G1** and **G2**.
- **Out of Scope**: Why the cursors are owned at all — see [`002`](002_owning_the_cursors_rather_than_borrowing_them.md).

### G1 — There Is No Mutating Method

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '&mut self|fn push|resize|insert|remove' ring_gating/src/lib.rs
# 96:    cursors.resize_with( consumers, PaddedCursor::default );
```

Live output:

```
    cursors.resize_with( consumers, PaddedCursor::default );
```

One hit, and it is inside `new`:

```rust
// ring_gating/src/lib.rs:92-98
pub fn new( capacity : Capacity, consumers : usize ) -> Self
{
  let mut cursors = Vec::with_capacity( consumers );
  cursors.resize_with( consumers, PaddedCursor::default );
  Self { cursors, capacity }
}
```

Every other method takes `&self`. `cursors()` hands out `&[ PaddedCursor ]` and
`cursor( i )` hands out `Option< &PaddedCursor >` — shared references, which is
all a consumer needs, since advancing a cursor is an atomic store through `&self`.

So the type divides cleanly: **the positions are mutable through shared
references; the membership is not mutable at all.**

That is not stated anywhere. `new`'s doc says `consumers` of zero is legal and
means ungated; nothing says the count is final. A reader who has just been told
the set "owns its cursors rather than borrowing them, so that the set and the
cursors cannot get out of sync" may reasonably expect an owner to manage
membership.

### G2 — One Construction Site, One Consumer

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'GatingSet::new' ring_*/src/*.rs | grep -vE ':\s*(///|//!|//)'
# ring_mpsc/src/lib.rs:380:      consumers : GatingSet::new( capacity, 1 ),
```

Live output:

```
ring_mpsc/src/lib.rs:      consumers : GatingSet::new( capacity, 1 ),
```

That is the whole list — **one** construction in production code, family-wide. Four
source files name the type at all, and only two of them use it:

| Crate | Relationship | Consumers |
|-------|--------------|:---------:|
| `ring_mpsc` | owns one, `GatingSet::new( capacity, 1 )` at `:380` | **1** |
| `ring_claim` | borrows one — `consumers : &'a GatingSet` at `:260` | whatever it is handed |
| `ring_barrier` | names it in prose only (`:32-37`), to say a `Barrier` borrows `&[ PaddedCursor ]` instead | — |
| `ring_consume` | names it in prose only (`:49`, `:62`), as the thing a consumer's own cursor usually ends up inside | — |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rl 'GatingSet' ring_*/src/*.rs | grep -v ring_gating
# ring_mpsc, ring_claim, ring_barrier, ring_consume
```

Live output:

```
ring_barrier/src/lib.rs
ring_claim/src/lib.rs
ring_consume/src/lib.rs
ring_mpsc/src/lib.rs
```

The last two rows are worth keeping in the table rather than dropping as
non-uses: both are *deliberate* non-uses, written down at the point where taking
a `GatingSet` would have been the obvious move.

`ring_mpsc` is multi-producer *single*-consumer, so `1` is correct there and not
a defect. The observation is about the family, not that call: **no production
code in any of the 33 crates ever builds a gating set with more than one
consumer.**

| Capability | Built in production code? | Built in some crate's tests? |
|------------|:-------------------------:|------------------------------|
| 0 consumers (ungated) | ❌ | ✅ this crate, and `ring_claim` heavily (`cap( 16_384 ), 0` ×4) |
| 1 consumer | ✅ `ring_mpsc` | ✅ throughout |
| 2 consumers | ❌ | ✅ this crate, `ring_claim:288`, `ring_barrier:437` |
| 3+ consumers | ❌ | ✅ this crate only (`:332` at 3, `:347` at 4) |

```sh
cd "$(git rev-parse --show-toplevel)"
# every multi-consumer construction outside this crate — 3, all in tests or doctests
grep -r 'GatingSet::new(' ring_*/{src,tests}/*.rs \
  | grep -v '^ring_gating/' | grep -vE ', (0|1) \)'
```

Live output:

```
ring_claim/src/lib.rs:  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 2 );
ring_barrier/tests/barrier_test.rs:  let set = GatingSet::new( cap( 8 ), 2 );
ring_claim/tests/claim_test.rs:  let consumers = GatingSet::new( cap( 8 ), 2 );
```

The two outside sites are both about *composition* rather than gating, which is
why they need more than one cursor:

- `ring_claim:288` — `the_claimer_exposes_the_gate_it_was_built_over`, asserting a
  borrowed set reports `len() == 2` through the borrower.
- `ring_barrier:437` — `a_barrier_over_a_gating_set_reads_that_set_and_not_a_copy`,
  asserting `barrier.frontier() == set.slowest()` after storing 5 and 3 into the
  two cursors. This is the sharpest external check the type has: two crates'
  independent folds over the same memory, required to agree.

The tests are good — `the_slowest_consumer_sets_the_bound…` sweeps the slow
consumer across all three indices precisely because an implementation reading
`cursors[ 0 ]` would pass a fixed-position test. What is missing is a *production
consumer* of the capability, not coverage of it.

### Why This Is Not Yet a Defect

The family's own decomposition puts a multi-consumer ring later. `ring_spsc` and
`ring_mpsc` are the two backends that exist; neither is multi-consumer by
definition. So a gating set of size > 1 has no caller because no ring that would
need one has been built yet, not because something was overlooked.

Building it now would be the YAGNI failure, not leaving it. What is worth
recording is the *shape* of the eventual requirement, because it is not obvious
that today's type can meet it:

| A future multi-consumer ring needs | `GatingSet` today |
|------------------------------------|-------------------|
| A fixed set of *n* consumers known at construction | ✅ Works unchanged |
| A consumer that joins a running ring | ❌ No method; requires `&mut self` and a `Vec` push |
| A consumer that leaves a running ring | ❌ Same, and worse — see below |

The join case is a small addition. **The leave case is not**, and it is worth
stating why before someone adds a `remove`:

- Removing a cursor removes a *bound*. A producer gated at sequence 100 by a departing consumer becomes free to advance immediately — correct only if that consumer really is finished, which the set cannot know.
- `cursor( i )` is indexed. Removing element 1 of three renumbers element 2, so every holder of index 2 now reads someone else's cursor. The `Option` return says "no such index", not "that consumer left".
- `cursors()` hands out a `&[ PaddedCursor ]` borrowed from the `Vec`. A `push` that reallocates invalidates nothing at the type level — the borrow checker prevents it — but it means `&mut self` conflicts with any live reader, which is exactly the situation a running ring is in.

The third point is the structural one: **`GatingSet`'s shared-reference reading
model and a mutable membership model are in tension**, and adding `&mut self`
methods to this type would make `cursors()` unusable while any of them could run.
A future multi-consumer ring most likely wants a different type — one whose slots
are `Option< PaddedCursor >` in a fixed-size array, so departure is a store rather
than a resize — and `GatingSet` should stay as it is.

**Recommendation:** document the immutability on the type, do not add mutators.
Two sentences on `GatingSet`'s doc comment saying membership is fixed at
construction and why. Not applied here — it is a source change, and this run
produced documentation.

### What `Vec` Buys, Given the Size Is Fixed

Given the count never changes after `new`, `Vec` is more than is needed —
`Box< [ PaddedCursor ] >` would express "a fixed-size heap array" exactly, and is
one `.into_boxed_slice()` away.

| | `Vec< PaddedCursor >` (as written) | `Box< [ PaddedCursor ] >` |
|---|---|---|
| Size of the field | 24 bytes (ptr, len, cap) | 16 bytes (ptr, len) |
| Expresses fixed size | ❌ `push` is one method call away | ✅ |
| Construction | `with_capacity` + `resize_with` | the same, plus `.into_boxed_slice()` |
| Precedent in the family | — | `ring_mpsc:373-374` does exactly this for its stamps |

`ring_mpsc` is the crate that owns a `GatingSet`, and it builds its own stamp
array as `.collect::< Vec< _ > >().into_boxed_slice()` — the conversion this type
does not do. That is a small, real inconsistency: the same author, in the same
family, chose the tighter type for the array they controlled.

Eight bytes and a signalled intent. Worth doing when the type is next touched;
not worth a change on its own.

### GT11 — No Method Can Change the Membership

```
&mut self             : 0
push / insert / remove : 0
93:  pub fn new( capacity : Capacity, consumers : usize ) -> Self
```

The consumer count is an argument to the constructor and appears nowhere else
as an input. Whatever a set is built with, it stays.

**Finding.** It has no `&mut self` method; consumers can be neither added nor removed after construction

---

### GT12 — A Growable Collection Never Grown

```
95:    let mut cursors = Vec::with_capacity( consumers );
96:    cursors.resize_with( consumers, PaddedCursor::default );
      ...and nothing else touches the Vec's length
```

`Vec` carries three words — pointer, length, capacity — and pays for the third
so that it can grow. This one is sized once and never resized.

**Finding.** `Vec::with_capacity` then `resize_with`, and nothing resizes it afterwards. A `Box< [ PaddedCursor ] >` would put the fixed length in the type and drop the capacity word; the `Vec` is a growable collection used for its allocation and never once for its growth

---


### APIs

| File | Relationship |
|------|--------------|
| [../api/001_eleven_methods_over_one_owned_vec.md](../api/001_eleven_methods_over_one_owned_vec.md) | The full surface, and the `&self`/`&mut self` split |

### Data Structures

| File | Relationship |
|------|--------------|
| [002_owning_the_cursors_rather_than_borrowing_them.md](002_owning_the_cursors_rather_than_borrowing_them.md) | Why they are owned in the first place |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_three_dependencies_and_two_dependents.md](../integration/001_three_dependencies_and_two_dependents.md) | The two crates that depend on this one, and the two that only test against it |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md](../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md) | The rule the unused multi-consumer path implements |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_set_from_construction_to_drop.md](../lifecycle/001_a_set_from_construction_to_drop.md) | Construction is the only moment membership is decided |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/002_the_multi_consumer_path_no_ring_uses.md](../workaround/002_the_multi_consumer_path_no_ring_uses.md) | G2's other half — the capability with tests and no callers |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:92-98` | `new` — the only place membership is set |
| `ring_gating/src/lib.rs:141-158` | `cursor` and `cursors`, both `&self` |
| `ring_mpsc/src/lib.rs:380` | The family's only construction, with `consumers = 1` |
| `ring_mpsc/src/lib.rs:373-374` | `.into_boxed_slice()` on the sibling array |
| `ring_claim/src/lib.rs:257` | The borrow, size unknown to the borrower |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:315-327` | One cursor per consumer, across 0..5, and no cursor past the end |
| `tests/gating_test.rs:133-147` | The minimum holds wherever the slow consumer sits — the multi-consumer path's real coverage |
| `tests/gating_test.rs:149-157` | One stalled consumer stops the producer for everyone |
