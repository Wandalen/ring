# Pattern: The Owned Set with Shared Readers

### Scope

- **Purpose**: Name the ownership shape `GatingSet` sits in — one owner, several `&self` readers, mutation through the leaf — and show what each participant may and may not do.
- **Responsibility**: Give the three roles, the two ways the family reaches them, and the constraint the shape imposes on its owner.
- **In Scope**: Who holds the set and who reads it.
- **Out of Scope**: Why the field is a `Vec` rather than a `Box< [ T ] >` — see [`data_structure/002`](../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md).

### The Shape

```
        ring_mpsc::Ring           owns  ──►  GatingSet { cursors : Vec< PaddedCursor >, capacity }
                                                 ▲            ▲
        ring_claim::Claimer   borrows  ──────────┘            │
          (&'a GatingSet, reads headroom)                     │
                                                              │
        each consumer          stores through  ───────────────┘
          (&PaddedCursor from cursor( i ), interior mutability)
```

Three roles, and only the first is an owner:

| Role | Holds | May | May not |
|------|-------|-----|---------|
| Owner | `GatingSet` by value | Construct it, hand out borrows, drop it | Add or remove a consumer — there is no `&mut self` method |
| Gate reader | `&GatingSet` | Call any of the eleven methods | Keep the borrow past the owner's life |
| Consumer | `&PaddedCursor` | `store` its own position | Read the set, or reach another consumer's cursor |

The third row is the pattern's payload. **A consumer mutates the set's contents
holding only a shared reference, and can reach exactly one cursor to do it.**
That is the whole reason `cursor( index )` returns `Option< &PaddedCursor >`
rather than a slice or a `&mut`.

### The Two Reachings

| Route | Written | Used in |
|-------|---------|---------|
| Borrow | `Claimer::new( &shared.consumers )` | `ring_mpsc:577`, the production path |
| `Arc` | `Arc::new( GatingSet::new( capacity, 1 ) )` | `ring_publish/tests/handshake_test.rs:86,177` |

Both are the same shape at different lifetimes. The borrow is cheaper and forces
the owner to outlive every reader lexically, which `ring_mpsc` records as a
constraint on its own API:

> Two steps rather than one because `Claimer` borrows the `GatingSet` it gates on
>
> — `ring_mpsc/src/lib.rs:559`

`Ring::ends` cannot return a `Claimer` and the ring together in one value without
a self-reference, so it returns an `Ends` that holds both. That is the price of
the borrowing route, paid once, at the one place that owns a set.

The `Arc` route appears only in tests, and only where a set must cross a thread
boundary and outlive the scope that made it. Nothing in production uses it — the
family's one owner is a `Ring`, which the threads share by reference already.

### What the Pattern Rules Out

| Alternative | Why not |
|-------------|---------|
| `&mut self` to add a consumer | Would require exclusive access to a value every producer and consumer holds a shared reference to |
| `Mutex< Vec< PaddedCursor > >` | A lock on the producer's hot path, to guard a `Vec` that never changes |
| Each consumer owning its own cursor | Then the producer has no set to fold over — the whole point is that the cursors are collocated |
| `Clone` on `GatingSet` | A cloned set forks the cursors; the producer would gate on positions no consumer updates |

The last row is the interesting one, because `GatingSet` derives only `Debug` —
`Clone` is absent with no comment saying it was refused. It looks like an
omission a careless `#[ derive( Clone ) ]` could undo. It is not:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -B2 'pub struct PaddedCursor' ring_cursor/src/lib.rs
# 143:#[ derive( Debug, Default ) ]
# 144:pub struct PaddedCursor( CacheAligned< AtomicSeq > );
```

Live output:

```
/// ```
#[derive(Debug, Default)]
pub struct PaddedCursor(CacheAligned<AtomicSeq>);
```

`AtomicU64` is not `Clone`, so `AtomicSeq` is not, so `PaddedCursor` is not, so
`Vec< PaddedCursor >` is not — a `derive( Clone )` on `GatingSet` fails to
compile. **The pattern is defended at the leaf**, exactly as its thread-safety is
([`type/002`](../type/002_send_and_sync_without_unsafe.md)): a property this
crate never states, inherited from four crates down, and not overridable here
because `unsafe-code = "deny"` leaves no manual route.

### The Constraint on the Owner

Because membership is fixed at construction, **the owner must know its consumer
count before it can build a ring.** `ring_mpsc` does — it is multi-producer
*single*-consumer by definition, so it passes `1` literally:

```rust
// ring_mpsc/src/lib.rs:380
consumers : GatingSet::new( capacity, 1 ),
```

A future multi-consumer ring inherits the same constraint: its consumer count
becomes a constructor argument, not a runtime registration. Whether that is the
right shape for a ring that gains consumers dynamically is the open question in
[`data_structure/001`](../data_structure/001_the_set_that_cannot_grow.md), and it
is a question about this pattern rather than about the type.

### GT42 — The Two Halves Chose Opposite Ownership

```
ring_gating : cursors : Vec< PaddedCursor >     -- owns
ring_barrier: cursors : &'a [ PaddedCursor ]    -- borrows
```

One feature, two crates, opposite decisions — and both are right. A producer's
gate outlives every consumer it watches; a consumer's barrier does not.

**Finding.** This half owns a `Vec< PaddedCursor >` and lends `&PaddedCursor`; `ring_barrier` borrows a slice it does not own. The two halves of one feature took opposite ownership decisions, each right for its side — a producer's gate outlives every consumer, a consumer's barrier does not

---

### GT43 — The Producer Is Not a Type

```
headroom( &self, producer : Seq )
admits  ( &self, producer : Seq, count : usize )
check   ( &self, producer : Seq, count : usize )
```

"Owned set with shared readers" names two roles. The set is one of them. The
other is a `u64` in a newtype, passed in by whoever is calling — the pattern's
second half has no representation in the code.

**Finding.** Every method takes the producer as a bare `Seq` argument. There is no producer type, no handle, and no way for the set to tell one caller from another — so "shared readers" is enforced by which reference a caller happens to hold, and the pattern names a role the code does not represent

---


### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_set_that_cannot_grow.md](../data_structure/001_the_set_that_cannot_grow.md) | The fixed-membership consequence, and its open question |
| [../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md](../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md) | The field choice underneath the pattern |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_eleven_methods_over_one_owned_vec.md](../api/001_eleven_methods_over_one_owned_vec.md) | G1 — the surface that makes the three roles possible |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_three_dependencies_and_two_dependents.md](../integration/001_three_dependencies_and_two_dependents.md) | The owner and the borrower, in their crates |

### Patterns

| File | Relationship |
|------|--------------|
| [002_the_predicate_the_quantity_and_the_reason.md](002_the_predicate_the_quantity_and_the_reason.md) | The API shape layered over this structural one |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_send_and_sync_without_unsafe.md](../type/002_send_and_sync_without_unsafe.md) | Why a shared reference is enough to store through |

### Sources

| File | Relationship |
|------|--------------|
| `ring_mpsc/src/lib.rs:337, 380, 577` | The owner |
| `ring_mpsc/src/lib.rs:559` | The lifetime constraint the borrow imposes |
| `ring_claim/src/lib.rs:257,273` | The borrower |
| `ring_publish/tests/handshake_test.rs:86,177` | The `Arc` route |
| `ring_cursor/src/lib.rs:143-144` | `PaddedCursor`, not `Clone` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:394-428` | Owner and reader in two threads, over a borrow |
| `ring_claim/tests/claim_test.rs:273-282` | A borrowed set read back through the borrower |
| `ring_publish/tests/handshake_test.rs:86` | The `Arc` route across threads |
