# Pattern: The Forwarding Newtype

### Scope

- **Purpose**: Identify the shape `PaddedCursor` instantiates, and record the reason it must exist — a language rule the source never names.
- **Responsibility**: Place it against the family's six other newtypes, derive the orphan-rule argument, and state what the shape costs.
- **In Scope**: `PaddedCursor` as a newtype; the family's single-field tuple structs; the `SeqCell` impl's legality.
- **Out of Scope**: The layout it carries, which is [`data_structure/001`](../data_structure/001_the_padded_cursor.md); the surface it exposes, which is [`api/001`](../api/001_the_surface_that_forwards.md).

### The Shape

A newtype whose body is empty of its own state and whose impls delegate
one-for-one to the wrapped value:

```rust
#[ derive( Debug, Default ) ]
pub struct PaddedCursor( CacheAligned< AtomicSeq > );

impl SeqCell for PaddedCursor
{
  fn load( &self, order : Ordering ) -> Seq { self.0.get().load( order ) }
  // …three more, each one line
}
```

The module documentation is explicit that it adds nothing:

> [`PaddedCursor`] is [`ring_atomic::AtomicSeq`] inside
> [`ring_align::CacheAligned`] and nothing else — no field of its own, no logic
> of its own.

**Which raises the question the source does not answer: then why does it exist?**
`CacheAligned< AtomicSeq >` is already 64 bytes, already 64-aligned, and already
holds a sequence. A type alias would spell it in one line and cost nothing.

### Against the Family's Other Newtypes

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE '^pub struct [A-Z][A-Za-z]*(< ?[A-Z] ?>)?\( ?[A-Za-z:<> ]+ ?\) ?;' \
  --include=*.rs ring_*/src/
```

Live output:

```
ring_align/src/lib.rs:pub struct CacheAligned< T >( T );
ring_cursor/src/lib.rs:pub struct PaddedCursor( CacheAligned< AtomicSeq > );
ring_poll/src/lib.rs:pub struct Budget( usize );
ring_slot/src/lib.rs:pub struct TypedSlot< T >( Option< T > );
ring_types/src/capacity.rs:pub struct Capacity( usize );
ring_types/src/id.rs:pub struct SlotIndex( pub usize );
```

| Newtype | Crate | Wraps | Field | Adds |
|---------|-------|-------|:-----:|------|
| `Capacity` | `ring_types:23` | `usize` | private | **validation** — `new` rejects non-powers-of-two |
| `SlotIndex` | `ring_types:99` | `usize` | **pub** | a name, and nothing else |
| `AtomicSeq` | `ring_atomic:166` | `AtomicU64` | private | `Seq` typing and the `SeqCell` impl |
| `CacheAligned< T >` | `ring_align:69` | `T` | private | **an alignment attribute** |
| `TypedSlot< T >` | `ring_slot:84` | `Option< T >` | private | slot semantics over an option |
| `Budget` | `ring_poll:116` | `usize` | private | poll-budget semantics |
| **`PaddedCursor`** | `ring_cursor:144` | `CacheAligned< AtomicSeq >` | private | **nothing of its own** |

**`PaddedCursor` is the only one that wraps another newtype, and the only one
that adds no capability.** Six of the seven exist to make a `usize` or a `T` mean
something specific. This one exists to relabel a composition that already means
exactly what it says.

`SlotIndex` is the near neighbour — it also adds only a name — but its field is
`pub`, so it is a label a caller can see through. `PaddedCursor`'s is private,
which makes it a boundary rather than a label.

### Why the Newtype Is Not Optional

The answer is the orphan rule, and it is worth deriving rather than asserting.

To use a cursor, a caller calls `SeqCell` methods on it. So *something* must
implement `SeqCell` for the padded type. Consider each place that impl could live:

| Where | `impl SeqCell for CacheAligned< AtomicSeq >` | Why not |
|-------|---------------------------------------------|---------|
| `ring_cursor` | **illegal** | Both the trait (`ring_atomic`'s) and the self type (`ring_align`'s, parameterised by `ring_atomic`'s) are foreign. `impl ForeignTrait for ForeignType` is rejected — **E0117** |
| `ring_align` | legal in principle | It would have to depend on `ring_atomic` to name `AtomicSeq` and `SeqCell`. It depends on neither, and doing so would put the atomic layer inside the alignment crate |
| `ring_atomic` | legal in principle | It would have to depend on `ring_align`, putting a layout concern inside the cell crate — and would make the padded form available to every consumer of `ring_atomic`, which is the opposite of the tiering |
| a blanket `impl< T : SeqCell > SeqCell for CacheAligned< T >` in `ring_align` | same problem | `SeqCell` is still foreign to `ring_align` |

**So a local type is required, and a type alias will not do** — an alias is not a
distinct type and inherits the same orphan-rule problem. A newtype is the only
construction that makes `impl SeqCell for …` legal in this crate.

The forwarding impl is therefore not boilerplate the author chose to write. It is
the price of the tiering: `ring_align` knows nothing about sequences,
`ring_atomic` knows nothing about cache lines, and the crate that combines them
must own a type to hang the combination on.

**Compiled and confirmed.** The derivation was checked rather than left as
reasoning: a throwaway crate depending on `ring_align`, `ring_atomic` and
`ring_types` — the same position `ring_cursor` would be in — attempts the impl
with all four methods fully written, so nothing but coherence can fail it:

```rust
use ring_align::CacheAligned;
use ring_atomic::{ AtomicSeq, SeqCell };

impl SeqCell for CacheAligned< AtomicSeq >
{
  fn load( &self, order : Ordering ) -> Seq { self.get().load( order ) }
  // … store, fetch_add, compare_exchange, each one delegating line
}
```

`cargo check` fails with exactly one error:

```
error[E0117]: only traits defined in the current crate can be implemented for types defined outside of the crate
   |                  `CacheAligned` is not defined in the current crate
   = note: impl doesn't have any local type before any uncovered type parameters
   = note: define and implement a trait or new type instead
```

The final note is the compiler independently prescribing the remedy this crate
applied. `PaddedCursor` **is** the "new type" rustc asks for — the impl is not a
stylistic choice, and the alternative rustc offers is the only other one
available.

Reproduce it by writing the snippet above into any crate that depends on
`ring_align` and `ring_atomic` but is neither of them.

### What the Shape Costs

| | |
|---|---|
| Source | Four one-line methods, plus the declaration |
| Runtime | Nothing — each method is a single delegating call in the same crate |
| Layout | Nothing — `size_of` and `align_of` are unchanged from the wrapped type |
| Structural | The type cannot later add an ordering policy without breaking the contract that it behaves as the unpadded cell does |
| Maintenance | Every method added to `SeqCell` must be added here too, by hand, and forgetting one is a compile error |

The last row is the honest cost and it is small: the compiler catches the
omission, so the duplication is mechanical rather than dangerous.

### When Not to Use It

| Signal | Prefer |
|--------|--------|
| The trait, the type, or both are local | A direct impl — no newtype needed |
| The wrapper needs to add state | A named struct with fields, not a tuple newtype — and see `tests/manual/readme.md` M2 on why state hidden in padding is a defect |
| Callers should see through the wrapper | A `pub` field, as `SlotIndex` does |
| The forwarding is partial — some methods change behaviour | Say so in the type's documentation. `PaddedCursor` forwards *all four* unchanged, and `padding_does_not_change_what_the_cell_does` asserts it |

### CU37 — Four Bodies, One Shape, No Opinions

```
203:    self.0.get().load( order )
208:    self.0.get().store( value, order );
213:    self.0.get().fetch_add( n, order )
219:    self.0.get().compare_exchange( current, new, success, failure )
```

No transformation, no reordering of arguments, no validation, no name change.

**Finding.** That uniformity is what makes this a pattern rather than a wrapper
with opinions: the newtype's entire behaviour is the layout it inherits plus the
vocabulary it re-exports. A fifth method added to `SeqCell` would be written the
same way by anyone who read these four, which is the property a pattern is
supposed to have.

---

### CU38 — The Pattern Rests on a Method in Another Crate Staying Free

`CacheAligned::get` is called by all four forwards and, transitively, by all
three readings. It is in `ring_align`, it is not `const`, and this crate asserts
nothing about it.

**Finding.** A change that gave `get` any work to do — a debug assertion, a
counter — would add that work to seven call paths here, and no test in this crate
would notice: every assertion is about layout or about arithmetic, none about
cost. The pattern is cheap by convention across a crate boundary, with no
contract on either side.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A4 -F '// Each forward below assumes `CacheAligned::get` stays a free, no-op' ring_cursor/src/lib.rs
```

Live output:

```
// Each forward below assumes `CacheAligned::get` stays a free, no-op
// accessor. A debug assertion or a counter added to it in `ring_align` would
// add that cost to all four methods here, and nothing in this crate's suite
// would notice — every assertion here is about layout or arithmetic, never
// about cost; the assumption is cheap by convention across the crate
```

**Disposition:** applied — the previously unstated cross-crate assumption is
now a comment directly above `impl SeqCell for PaddedCursor`, naming exactly
what this finding names: that `CacheAligned::get` stays free, that a change
to it would add cost to all seven call paths here, and that no test in this
crate's suite would notice. The crate's 23 tests (1 `allocation_test.rs` + 22
`cursor_test.rs`) plus 14 doctests re-verified passing (`cargo test
--all-features`, 2026-09-04). Now prints:
`add that cost to all four methods here`

---

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_surface_that_forwards.md](../api/001_the_surface_that_forwards.md) | The four methods this shape produces |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_padded_cursor.md](../data_structure/001_the_padded_cursor.md) | The layout the newtype carries without adding to |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_four_dependencies_all_used.md](../integration/001_four_dependencies_all_used.md) | The two dependencies whose separation forces this shape |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_the_number_64_never_appears_here.md](../invariant/002_the_number_64_never_appears_here.md) | Why the attribute is inherited through the wrapper rather than restated |

### Patterns

| File | Relationship |
|------|--------------|
| [002_one_fold_two_questions.md](002_one_fold_two_questions.md) | The crate's other anti-duplication shape, enforced by nothing |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_padded_cursor.md](../type/002_padded_cursor.md) | The promises the shape carries, and the four derives it declines |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/002_the_trait_must_travel_with_the_type.md](../workaround/002_the_trait_must_travel_with_the_type.md) | The second consequence of the same trait-scoping rules — the re-export |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:13-21, 143-144, 199-221` | The declaration, the impl, and the "nothing else" claim |
| `ring_align/src/lib.rs:69` | `CacheAligned< T >` — foreign, and parameterised by a second foreign type |
| `ring_atomic/src/lib.rs:108-151,153-166` | `SeqCell` and `AtomicSeq` — the foreign trait and the foreign payload |
| `ring_types/src/id.rs:99` | `SlotIndex( pub usize )` — the family's counter-example |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:137-160` | Every forwarded method behaves as the unpadded cell does |
| `tests/cursor_test.rs:399-408` | The newtype adds no bytes |
| `tests/manual/readme.md` M2 | The newtype adds no fields — checked by reading, since `size_of` cannot see one |
