# API: Producer Surface

### Scope

- **Purpose**: Specify the publishing end's operations and, as importantly, enumerate what is absent from it — since on this crate the absences are the contract.
- **Responsibility**: The operations, their error handling, and the compatibility guarantees a five-crate export list imposes on them.
- **In Scope**: `Producer`'s methods and non-methods; the shape of its refusals.
- **Out of Scope**: The draining end (→ [Consumer Surface](002_consumer_surface.md)); what happens inside the ring, which is `ring_core`'s.

### Abstract

**`Producer` is the value a system holds when it is allowed to publish and
nothing else.** Its surface is small by construction: publish, ask for room,
and nothing that could block, drain, or reach the ring directly.

Unlike the surfaces of internal crates such as
[`ring_spsc`](../../../ring_spsc/docs/api/001_producer_surface.md) — where three
candidate shapes can be left undecided because a change costs a two-crate
refactor — **this surface is on the family's five-name export list**, so every
signature here is a public contract with no absorbing indirection above it
(→ [On the Export Surface](../integration/002_on_the_export_surface.md)). The
shapes below are correspondingly more committed.

### Operations

| Operation | Shape | Returns | Blocks | Notes |
|-----------|-------|---------|--------|-------|
| Publish one item | `try_push( &mut self, record: T ) -> Result< (), T >` | Unit or the record back | **Never** | The primary operation. Returning the record in the error is what makes a refusal recoverable without a copy |
| Publish a batch | `try_push_batch( &mut self, records: &mut impl Iterator< Item = T > ) -> usize` | How many were accepted | **Never** | Partial acceptance is the normal case; a `Result` would force an all-or-nothing contract the ring cannot offer cheaply |
| Ask for room | `free_capacity( &self ) -> usize` | A lower bound | Never | **Advisory at MPSC, binding at SPSC** — see the note below |
| Ask whether the ring is full | `is_full( &self ) -> bool` | — | Never | `free_capacity() == 0`, literally — not a second reading (→ [`ring_core/docs/pitfall/001`](../../../ring_core/docs/pitfall/001_free_capacity_carries_two_contracts.md)) |

#### Three shapes this instance specified that the implementation changed

This instance was written before `ring_core`'s handles existed and before
`ring_shutdown` did. Three of its original shapes did not survive contact, and
each correction is a finding rather than a concession:

| Specified | Implemented | Why |
|---|---|---|
| `&self` receivers | **`&mut self`** | `&self` would let two threads share one `&Producer` and publish concurrently — precisely the cardinality violation `ring_spsc` cannot detect at runtime. `&mut self` makes "exactly one producer" a borrow-checker property (→ [`invariant/001`](../invariant/001_capability_follows_the_handle.md)) |
| `Result< (), Full< T > >` | **`Result< (), T >`** | A single-variant error newtype adds a name and no information. `Full`/`Closed` only earn their keep together, and `Closed` is unreachable here — the two-armed type exists already as [`ring_shutdown::Refusal`](../../../ring_shutdown/docs/type/002_refusal_carries_the_record.md) |
| `is_closed( &self ) -> bool` | **Absent** | Reading the flag means depending on `ring_shutdown`, which depends on `ring_wait` — putting a parking operation within reach of the tick path and breaking the non-parking restriction (→ [`decisions/002`](../decisions/002_why_is_closed_is_absent.md)) |

**`free_capacity`'s contract is inherited, not chosen here, and it differs by
backend.** At SPSC cardinality this handle is the only producer, so a reported
`n` cannot shrink and `n` subsequent pushes are guaranteed to succeed — it is a
binding guarantee
(→ [`ring_spsc` type/002](../../../ring_spsc/docs/type/002_free_capacity.md)).
At MPSC cardinality another producer may consume the room between the read and
the push, so the same call is a hint. **One signature, two contracts, selected
by a config field the caller may not have set** — which is a genuine hazard of
being the unifying surface over several backends, and is recorded here rather
than resolved.

#### Absent operations, and why

| Absent | Why | Consequence if added |
|--------|-----|----------------------|
| Any drain / `try_recv` / `pop` | The capability belongs to `Consumer` | [Capability Follows the Handle](../invariant/001_capability_follows_the_handle.md)'s V1 — **the one violation the acceptance criterion catches**, measured at `tests/manual/readme.md` H1 |
| `try_clone` | `ring_core::Producer` has it and it succeeds on an MPSC backend; withholding it is one of this crate's four reasons to exist | A second producer on an SPSC ring, refused at runtime rather than at compile time (→ [`decisions/001`](../decisions/001_what_this_crate_is_for.md)'s N2) |
| A blocking `push` | Nothing reachable from a handle may park | Deadlock inside a tick (W1) |
| `push_timeout( d: Duration )` | Bounded parking is parking | W2 — passes a naming audit, misses the frame deadline |
| `Clone` | One producer per ring at SPSC cardinality | **Silent data race** in `ring_spsc` (V3). Not covered by the compile-fail cases as specified |
| `Deref`, `inner()`, a public field | Any of the three restores the full backend surface | V4 — the split becomes advisory |
| A flush call | Flushing is policy, not an operation | Belongs to [`ring_flush`](../../../ring_flush/readme.md), whose triggers are the point |

**The absent list is the specification.** A reader looking for what this crate
guarantees should read this table, not the one above it — the operations are
ordinary forwarding, and the guarantees are here.

### Error Handling

| Condition | Result | Recoverable |
|-----------|--------|-------------|
| The ring is full | `Err( item )` — the record itself, handed back unwrapped | Yes, and without a copy: the caller may retry later, drop it, or route it per [`ring_overflow`](../../../ring_overflow/readme.md)'s policy |
| A batch is partially accepted | `n` where `n < len` — a bare `usize`, not wrapped in `Result` | Yes — the iterator is left positioned at the first unaccepted item |
| No producer capability | — | **Not a runtime condition.** The call does not compile |

**There is no separate "ring is closed" row.** `Closed` is unreachable from
this surface — the "Three shapes" table above and Compatibility Guarantee 4
both already say so. A refused push returns `Err( item )` whether the cause
was a full ring or a closed one, because `try_push`'s `Result< (), T >` carries
no state that could tell the two apart; that distinction exists only in
[`ring_shutdown::Refusal`](../../../ring_shutdown/docs/type/002_refusal_carries_the_record.md),
a different type this crate does not return.

**Returning the item inside the error is the design decision in this section.**
An `Err( Full )` that swallowed the value would force every caller to clone
before pushing, on every push, to survive the refusal — a per-operation cost
paid to handle a case that mostly does not occur.

**`Full` and `Closed` must stay distinct.** Collapsing them into one `Err`
loses the difference between "retry later" and "stop forever," and a caller that
cannot tell them apart will spin against a closed ring — arriving at
[Nothing Reachable From a Handle Can Park](../invariant/002_no_parking_operation_is_reachable.md)'s
W3 by a route that starts as an error-type simplification.

**No error names an internal crate.** The error type is part of the exported
surface, so a variant carrying a `ring_claim::ClaimError` would leak an internal
type across the export boundary and defeat gate G5's purpose while satisfying
its letter (it checks manifests, not type signatures).

### Compatibility Guarantees

1. **The absent list is part of the contract, and additions to it are breaking
   in the direction nobody checks.** Adding a method is normally
   backward-compatible; here, adding `drain` or `Clone` breaks guarantees other
   crates depend on while breaking no caller.
2. **`try_push` returns the item on refusal.** Changing this to a bare
   `Err( Full )` is a source break for every caller and a performance
   regression for those that adapt by cloning.
3. **`free_capacity` is a lower bound, never an exact count**, so that the
   MPSC and SPSC contracts can share one signature. Callers that need the
   binding form must document that they require an SPSC ring.
4. ~~**`Full` and `Closed` remain distinguishable variants**, not one error.~~
   **Superseded.** `Closed` is not reachable from this surface, so there is one
   refusal and it carries the record. The two-armed form lives in
   [`ring_shutdown::Refusal`](../../../ring_shutdown/docs/type/002_refusal_carries_the_record.md),
   which is where a closed ring can actually be observed.
5. **No error variant names a crate outside the five-name export list.**
6. **Version movement is lockstep** across the family's 33 crates.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_delegating_to_the_backend.md](../algorithm/002_delegating_to_the_backend.md) | What each operation does between call and return — almost nothing, deliberately |
| [../algorithm/001_splitting_a_ring_into_two_ends.md](../algorithm/001_splitting_a_ring_into_two_ends.md) | Where this value comes from |

### APIs

| File | Relationship |
|------|--------------|
| [002_consumer_surface.md](002_consumer_surface.md) | The complementary half; between them the backend's capabilities are partitioned exactly once |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_on_the_export_surface.md](../integration/002_on_the_export_surface.md) | Why guarantee 5 exists and why these shapes are committed rather than open |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | The absent-operations table, stated as an invariant with its violation costs |
| [../invariant/002_no_parking_operation_is_reachable.md](../invariant/002_no_parking_operation_is_reachable.md) | Why every operation here returns rather than waits |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_proven_by_code_that_must_not_compile.md](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md) | The criterion asserting the drain row of the absent table |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_convenience_method_undoes_the_crate.md](../pitfall/001_a_convenience_method_undoes_the_crate.md) | Each absent row as an edit that actually gets proposed, with its stated reason |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_producer.md](../type/001_producer.md) | The value this surface belongs to |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_capability_follows_the_handle.md`](../invariant/001_capability_follows_the_handle.md) | "A producer handle can publish and cannot drain" |
| [`ring_poll/readme.md`](../../../ring_poll/readme.md) | The `try_`-only shape of every operation above |
| [`ring_overflow/readme.md`](../../../ring_overflow/readme.md) | Where an `Err( Full )` is routed once returned |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `a_refused_push_hands_the_record_back_unchanged` | A refused push returns the item unchanged — guarantee 2, asserted by pushing the returned value successfully after a drain |
| **not written, and will not be** | Guarantee 4 was struck through above: `try_push` returns `Result< (), T >`, so there is no error enum with a `Full` and a `Closed` variant to discriminate. A test asserting they are distinguishable would need the type first. This row is kept rather than deleted because a reader who finds the struck-through guarantee should also find that nothing tests it |
| [`tests/ui/producer_drains.rs`](../../tests/ui/producer_drains.rs) | The absent-operations table's first row |

### HD5 — The Error Handling Table Names Three Types That Do Not Exist

The Operations table records the correction to `Result< (), T >`. The Error
Handling table below it was never brought along:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the implemented refusal shape --'
command grep -E '^  pub fn (try_push|try_push_batch)\(' ring_handle/src/lib.rs
echo '  -- and the types the error table names --'
for n in Full Closed Refusal; do
  printf '  %-8s ring_handle=%s ring_core=%s\n' "$n" \
    "$( command grep -cE "(enum|struct) $n\b" ring_handle/src/lib.rs )" \
    "$( command grep -cE "(enum|struct) $n\b" ring_core/src/lib.rs )"
done
```

Live output:

```
  -- the implemented refusal shape --
  pub fn try_push( &mut self, record : T ) -> Result< (), T >
  pub fn try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize
  -- and the types the error table names --
  Full     ring_handle=0 ring_core=0
  Closed   ring_handle=0 ring_core=0
  Refusal  ring_handle=0 ring_core=0
```

`try_push` returns `Result< (), T >` — the record itself, no wrapper. `Full`,
`Closed` and `Refusal` are declared in neither crate.

**Three of the Error Handling table's four rows describe a type that was ruled
out in the section immediately above them.** The Operations table's own
"Three shapes this instance specified that the implementation changed" block
explains exactly why `Result< (), Full< T > >` became `Result< (), T >`: a
single-variant newtype adds a name and no information. Four paragraphs later the
error table is still writing `Err( Full( item ) )`.

The *reasoning* in both sections survives the correction intact — returning the
record without a copy, and keeping "retry later" distinct from "stop forever",
are both still right. Only the spelling is stale, which is the version of this
defect most likely to be read straight past.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle
# -m1: this file is its own subject, so an unbounded match also finds this
# command line, every copy of its own output below, and the disposition line —
# each requote then compounded the previous one's
command grep -m1 -F 'no state that could tell the two apart' docs/api/001_producer_surface.md
```

Live output:

```
no state that could tell the two apart; that distinction exists only in
```

**Disposition:** applied — the Error Handling table no longer names `Full`,
`Closed`, or `Refusal`. It now states the two rows that actually exist
(`Err( item )` for a full ring, a bare `n` for a partial batch) and explains
why there is no separate "closed" row: `Closed` is unreachable from this
surface, so a full ring and a closed one return the identical `Err( item )`.
Now prints: `no state that could tell the two apart`

### HD6 — The Crate's Only Constructor Cannot Be Called by the Audience the Crate Is For

`ring_handle` is on the five-name Contract; `Split::new`'s argument type is not:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the constructor and what it takes --'
command grep -E '^use ring_core|^  pub const fn new\(' ring_handle/src/lib.rs
echo '  -- what this crate re-exports --'
printf '  pub use lines: %s\n' \
  "$( command grep -cE '^pub use ' ring_handle/src/lib.rs )"
echo '  -- the names a consumer outside the family may write --'
command grep -vE '^#|^$' bench_harness/gate/declared/ring/export_surface.txt
```

Live output:

```
  -- the constructor and what it takes --
use ring_core::Ring;
  pub const fn new( ring : Ring< T > ) -> Self
  -- what this crate re-exports --
  pub use lines: 0
  -- the names a consumer outside the family may write --
ring_factory
ring_handle
ring_tls
ring_flush
ring_types
```

`Split::new( ring : Ring< T > )` needs a `ring_core::Ring`. `ring_core` is not
on the Contract, and this crate re-exports nothing.

**So `Split::new` is `pub`, documented, and unreachable from the audience the
export surface defines.** That is not a bug — it is
[`ring_factory/docs/decisions/001`](../../../ring_factory/docs/decisions/001_the_owner_is_the_return_value.md)
working exactly as ruled: `ring_factory::build` returns a `Split`, so an outside
consumer receives one rather than constructing one, and `ring_factory`
re-exports `RingConfig` and `Registry` precisely so nothing else has to be
named.

What the api instance does not say is that this makes `Split::new` an
**intra-family** entry point wearing a public signature. A reader on the
Contract who finds it in the docs will go looking for a `Ring` they are not
permitted to name, and nothing in this instance, in the type's doc comment, or
in the export contract tells them to use `ring_factory::build` instead.
