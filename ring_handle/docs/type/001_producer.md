# Type: Producer

### Scope

- **Purpose**: Define the publishing handle as a value — what it is, what it is not, and the trait implementations that are as much a part of its definition as its fields.
- **Responsibility**: The definition and the validation rules governing it, including the derives it must never acquire.
- **In Scope**: `Producer< T >`'s identity, representation, and auto-trait status.
- **Out of Scope**: Its methods (→ [Producer Surface](../api/001_producer_surface.md)); the shared backend's own layout, which is `ring_core`'s.

### Definition

**A `Producer< T >` is the exclusive right to publish into one ring, expressed
as an owned value.**

The phrasing is chosen carefully. It is not "a reference to a ring with
publishing methods" — that would be a view, and views are copyable. It is a
*right*: something one party holds and another therefore does not, which is why
its non-`Clone` status is not an optimization but the type's meaning.

| Property | Value |
|----------|-------|
| Kind | Struct, generic over the item type `T` |
| Fields | One — a reference to the shared backend (→ [Two Handles Over One Backend](../data_structure/001_two_handles_over_one_backend.md)) |
| Size | One pointer |
| Constructed by | The split, and nothing else (→ [Splitting a Ring Into Two Ends](../algorithm/001_splitting_a_ring_into_two_ends.md)) |
| Constructed how many times per ring | Exactly once |
| Exported | **Yes** — reached through `ring_handle`, one of the five Contract names |

**The trait implementations are part of the definition, not an afterthought:**

| Trait | Status | Why |
|-------|--------|-----|
| `Send` | **Required** | The pair must move to two threads — [Send Without Sync](../non_functional_requirement/002_send_without_sync.md)'s N1 |
| `Sync` | **Deliberately not granted** | Two threads holding `&Producer` is a second producer by another name |
| `Clone` | **Forbidden** | A cloned right is not exclusive; at SPSC cardinality this is a data race |
| `Copy` | **Forbidden** | Same, more so — a `Copy` right is duplicated by passing it |
| `Debug` | Permitted | Must not print the backend's contents; a ring's worth of items in a panic message is not a debug aid |
| `Deref` | **Forbidden** | Restores every backend capability, including drain |
| `Default` | **Meaningless** | There is no ring to publish into; a `Default` impl would have to invent one |
| `Drop` | Permitted, must not flush | A flushing `Drop` makes the type's behaviour depend on when it is dropped |

**`Sync`'s absence is the subtlest entry.** `Send` and `Sync` are auto traits:
a type acquires them by having members that have them, without anyone choosing.
So `Producer` becoming `Sync` is not an edit anyone makes — it is something
that *happens* when the backend field's type changes, and the diff that causes
it need not mention `Producer` at all.

### Validation

| # | Rule | Enforced by | Detected when |
|---|------|-------------|---------------|
| U1 | Exactly one `Producer` exists per ring | The split returning it by value, and `!Clone` | Compile time — for the copying route only. A *moved* handle passed between threads sequentially satisfies this and still breaks `ring_spsc`'s invariant (→ [`ring_spsc` invariant/001](../../../ring_spsc/docs/invariant/001_exactly_one_producer_one_consumer.md)) |
| U2 | No drain-shaped method exists on it | Absence, plus `tests/ui/producer_drains.rs` | Compile time; the case is the acceptance criterion |
| U3 | It is `Send` | The backend field's own bounds | Compile time, via a static assertion |
| U4 | It is not `Sync` | `tests/ui/producer_shared_across_threads.rs` | Compile time. The property is inherited from `ring_spsc`'s `PhantomData< Cell< () > >` two crates down, so the edit that would break it is not in this crate's diff — which is why a case rather than review |
| U5 | It is not `Clone` and not `Copy` | `tests/ui/producer_clones.rs` | Compile time. A case this crate added beyond the acceptance table's two, because a `#[derive(Clone)]` here is a data race rather than a compile error downstream |
| U6 | It is exactly the size of the `ring_core` handle it wraps (24 bytes, not one pointer — a discriminant, a reference, and an `OverflowPolicy`) | `the_wrapper_costs_nothing` | A `size_of` equality catches an added field. The absolute figure was specified as 8 and is wrong; the equality is the claim that survives `ring_core` changing its own layout |
| U7 | No public route reaches the backend | Absence | **Not detected** — no case is specified against accessors |

**U7 alone is undetected, and that is this type's real exposure.** U4 and U5
were on that list until compile-fail cases closed them; U7 stays because a case
can name a method that must not exist and cannot name *any route* that must not
exist. U1 and
U2 — the properties the acceptance criterion covers — are the ones a reader
would guess are at risk; the three that are actually unguarded are auto-trait
drift, a one-line derive, and an accessor
(→ [A Convenience Method Undoes the Crate](../pitfall/001_a_convenience_method_undoes_the_crate.md)).

**U1's parenthetical is the honest limit of this type's guarantee.** Non-`Clone`
prevents *duplication*. It does not prevent a single handle being moved from
thread A to thread B and used from both in sequence — which the compiler accepts
because it is one value, and which `ring_spsc` cannot survive without an
external happens-before edge. No type-level mechanism closes that; it is stated
where it is enforceable, in `ring_spsc`'s own invariant, and pointed at from
here so the gap is not mistaken for coverage.

**U6 is the cheapest rule in the table and it guards a property nothing else
does.** Every forbidden addition that adds *state* — a counter, a closed flag, a
batch buffer — changes the size. One assertion covers all of them without
naming any.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | What this value can do, and the absent-operations list U2 and U7 protect |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_splitting_a_ring_into_two_ends.md](../algorithm/001_splitting_a_ring_into_two_ends.md) | The only construction site — U1's mechanism |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) | The single field, and the three candidate shapes that determine U3 and U4 |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | U2 and U7 as its V1 and V4; U5 as its V3 |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_send_without_sync.md](../non_functional_requirement/002_send_without_sync.md) | U3 and U4 as measurable criteria |
| [../non_functional_requirement/001_proven_by_code_that_must_not_compile.md](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md) | U2's detector, and the gap U4/U5/U7 fall into |

### Types

| File | Relationship |
|------|--------------|
| [002_consumer.md](002_consumer.md) | The complementary right; the two partition the backend's capabilities exactly |

### Sources

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | "A producer handle can publish and cannot drain" — this crate's own invariant, restated as this type's definition |
| [../integration/002_on_the_export_surface.md](../integration/002_on_the_export_surface.md) | Places this crate, not this type itself, on the five-crate export list |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `both_handles_are_send` | `assert_send::< Producer< '_, u32 > >()` — U3 |
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `the_wrapper_costs_nothing` | U6, as an equality with the *wrapped* handle rather than with `usize`. The original form asserted one pointer and would have failed on first run: a `Producer` is 24 bytes. The equality survives a `ring_core` layout change; the absolute figure would have been re-pinned to whatever was measured, which tests nothing |
| [`tests/ui/producer_clones.rs`](../../tests/ui/producer_clones.rs) | U5 — a compile-fail case, and still not in the acceptance criterion. Writing a stricter local test needs no amendment to the shared table: a criterion the crate exceeds is not a criterion the crate has changed |
| [`tests/ui/producer_shared_across_threads.rs`](../../tests/ui/producer_shared_across_threads.rs) | U4 — pins `Sync` status rather than leaving it to auto-trait drift. It has to be a compile-fail case: Rust has no negative trait bound, so `!Sync` is not expressible as a static assertion |

### HD41 — The Definition Table Says One Pointer, the Validation Table Says Three Times That, and the Suite Pins Neither

Both type instances state this handle's size twice, thirty rows apart, and
disagree with themselves:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the Definition tables --'
command grep -H '^| Size |' ring_handle/docs/type/00[12]_*.md | sed 's|^|    |'
echo '  -- the Validation tables in the same two files --'
command grep -H -E '^\| (U6|C6) \|' ring_handle/docs/type/00[12]_*.md | cut -c1-150 | sed 's|^|    |'
echo '  -- and what the suite actually asserts --'
command grep 'size_of' ring_handle/tests/handle_test.rs | sed 's|^|    |'
```

Live output:

```
  -- the Definition tables --
    ring_handle/docs/type/001_producer.md:| Size | One pointer |
    ring_handle/docs/type/002_consumer.md:| Size | One pointer |
  -- the Validation tables in the same two files --
    ring_handle/docs/type/001_producer.md:| U6 | It is exactly the size of the `ring_core` handle it wraps (24 bytes, not one pointer — a discrimin
    ring_handle/docs/type/002_consumer.md:| C6 | It is exactly the size of the `ring_core` handle it wraps (16 bytes — a discriminant and a referen
  -- and what the suite actually asserts --
      use core::mem::size_of;
        size_of::< ring_handle::Producer< '_, u32 > >(),
        size_of::< ring_core::Producer< '_, u32 > >(),
        size_of::< ring_handle::Consumer< '_, u32 > >(),
        size_of::< ring_core::Consumer< '_, u32 > >(),
    /// `the_wrapper_costs_nothing`'s `size_of` equality.
```

"One pointer" is 8 bytes on this host. U6 says 24 and gives the reason — a
discriminant, a reference and an `OverflowPolicy`, because the wrapped
`ring_core::Producer` is a backend enum plus a policy field. C6 says 16 for the
consumer, which has no policy field. The Definition rows were written from the
crate's shape — one field, therefore one pointer — and the inference is wrong
because the one field is not a pointer.

**The suite cannot arbitrate.** `the_wrapper_costs_nothing` asserts
`size_of::< ring_handle::Producer >() == size_of::< ring_core::Producer >()`.
That is deliberately an equality between the wrapper and the wrapped, chosen so
it survives `ring_core` changing its own layout — which means no absolute
figure in either file is checked by anything. The 8 is wrong, the 24 and 16
happen to be right, and the assertion that runs would pass on all three.

The Definition table is the row a reader reaches first and the one nothing
tests. [`data_structure/001`](../data_structure/001_two_handles_over_one_backend.md)'s
HD12 records the same figure's history from the design side — it was specified
as one pointer, measured at 24, and that starting figure was left standing.

### HD42 — The Export Row Claims Membership on a List of Crates, Not Types, and Its Sibling Row Gets It Right

The Definition table's last row places this type on the family Contract. The
Contract is a list of crate names:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the five names the Contract actually lists --'
sed -n '/^ring_/p' bench_harness/gate/declared/ring/export_surface.txt | sed 's|^|    |'
echo '  -- the five names this crate exports --'
command grep -oE '^pub struct [A-Za-z]+' ring_handle/src/lib.rs | sed 's|^|    |'
echo '  -- the two Exported rows, matched as table rows --'
command grep -H '^| Exported |' ring_handle/docs/type/00[12]_*.md | cut -c1-140 | sed 's|^|    |'
```

Live output:

```
  -- the five names the Contract actually lists --
    ring_factory
    ring_handle
    ring_tls
    ring_flush
    ring_types
  -- the five names this crate exports --
    pub struct Split
    pub struct Ends
    pub struct Producer
    pub struct Consumer
    pub struct Drain
  -- the two Exported rows, matched as table rows --
    ring_handle/docs/type/001_producer.md:| Exported | **Yes** — reached through `ring_handle`, one of the five Contract names |
    ring_handle/docs/type/002_consumer.md:| Exported | **Yes** — reached through `ring_handle`, one of the five Contract names |
```

`Producer` is not one of the five names on the Contract. The five are crates —
which `ring_*` a consumer outside `ring_*` may name as a dependency —
and `ring_handle` is one of them; the types it exports are reached *through*
that membership, not listed alongside it.

**The coincidence that makes the error easy is arithmetic.** This crate exports
exactly five type names and the Contract carries exactly five crate names. Two
five-element lists, disjoint, one row apart in a reader's attention.

[`type/002`](002_consumer.md)'s row says it correctly — "reached through
`ring_handle`, one of the five Contract names" — so the distinction was
understood when the sibling file was written and lost in this one. The
consequence is not cosmetic: the Contract file's own header says widening it
widens the family's export Contract and requires a decision, and a row claiming a
*type* sits on that list invites the reading that adding a sixth type is the
same kind of act as adding a sixth crate. It is not
(→ [`integration/002`](../integration/002_on_the_export_surface.md)'s HD23 on
what the file is checked against).

**Disposition:** applied — the Definition table's Exported row now reads
"reached through `ring_handle`, one of the five Contract names," matching
`type/002_consumer.md`'s already-correct phrasing instead of claiming
`Producer` itself sits on the five-crate Contract list.
Now prints: `one of the five Contract names`
