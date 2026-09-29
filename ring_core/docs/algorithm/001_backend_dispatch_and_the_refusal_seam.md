# Algorithm: Backend Dispatch and the Refusal Seam

### Scope

- **Purpose**: Specify how one `try_push` reaches three differently-shaped backends while honouring one contract, and record the seam where that nearly failed.
- **Responsibility**: The dispatch mechanism, the per-backend push procedure, the overflow-policy resolution applied after all three, and the ordering constraint the MPSC arm must respect.
- **In Scope**: `Producer::try_push` and `Producer::try_push_batch`.
- **Out of Scope**: Each backend's internal publish protocol (→ [`ring_spsc` algorithm/001](../../../ring_spsc/docs/algorithm/001_uncontended_claim_and_publish.md), [`ring_mpsc` algorithm/001](../../../ring_mpsc/docs/algorithm/001_claim_then_publish.md)); the drain side (→ [`algorithm/002`](002_uniform_drain_over_three_shapes.md)).

### Abstract

One `try_push` reaches three differently-shaped backends and honours one contract:
the record comes back intact on refusal, at every backend. Two of the three offer
that naturally; the third consumes the value and cannot give it back, and closing
that gap is the seam where this crate's one shipped bug lived.

### Algorithm

Dispatch on the private enum, run the backend's own publish shape, then resolve
the overflow policy once — after the match, never inside an arm.

#### Dispatch

Dispatch is an enum `match`, not a trait object. `ProducerInner` has one
variant per backend, and the crossbeam variant is `#[ cfg( feature = ... ) ]`,
so in the default build the match has two arms and the third does not exist.

**A trait would have been the obvious choice and is the wrong one here**, for
two reasons that are specific to this crate rather than general:

1. The three backends do not share a method set. `ring_spsc` and `ring_mpsc`
   publish through a slot reservation; `ArrayQueue` has only `push( value )`.
   A trait would have to be written to the *intersection*, which is the
   value-shaped surface — so the trait buys nothing the enum does not, and
   costs a vtable on the publish path.
2. The backend is fixed at construction and never changes. Dynamic dispatch
   pays for a choice that is already made.

The cost of the enum is that adding a backend is an exhaustive-match change
across every method rather than a new impl. That is the right trade at three
backends and the wrong one at ten; it is worth revisiting only if a fourth
appears.

#### Procedure — `try_push`

**Input:** a record `T`, by value.
**Output:** `Ok( () )`, or `Err( record )` with the record handed back intact.

| Step | SPSC | MPSC | crossbeam |
|---|---|---|---|
| 1 | `claim()` a reservation | `claim()` a reservation | — |
| 2 | on `Err`, return the record | on `Err`, return the record | — |
| 3 | `set( record )` into the slot | `set( record )` into the slot | `push( record )`, or `force_push` under `DropOldest` |
| 4 | guard's `Drop` publishes | guard's `Drop` publishes | already published |
| 5 | apply the overflow policy to any refusal | same | same |

Step 5 is shared and runs after the backend arm: a refusal is passed to
`ring_overflow::would_resolve`, and under `DropNewest` the record is dropped and
`Ok( () )` returned instead. So the policy is resolved in exactly one place for
all three backends, and a backend arm never has to know which policy is in
force — except `DropOldest`, which crossbeam alone implements and which it
handles in step 3 because eviction has to happen *instead of* the push rather
than after it.

#### The refusal seam

Steps 1–2 at MPSC are the whole point of this instance, and the crate shipped
one build without them.

`ring_mpsc::Producer::push` has signature `push( self, record : T ) ->
Result< Seq, RingError >`. **It takes the record by value.** On a full ring the
record is consumed inside it and there is nothing left to return — while this
crate's `try_push` promises `Result< (), T >`, which requires handing it back.

The two signatures are incompatible, and the incompatibility is invisible at
the call site: `producer.push( record )` compiles. The only way to make the
result *type-check* is to panic on the `Err`, which is what the first
implementation did:

```rust
// The shipped bug. Panics on any full multi-producer ring.
Ok( producer.push( record ).map( | _ | () ).expect( "mpsc refusal carries no record" ) )
```

The fix is to move the refusal **ahead of** the move:

```rust
match producer.claim()
{
  Ok( mut reserved ) => { reserved.set( record ); Ok( () ) }
  Err( _ ) => Err( record ),          // record never entered the backend
}
```

`claim()` returns a reservation without consuming anything, so a full ring is
detected while the record is still the caller's.

**What makes this worth a document rather than a comment** is how it was found,
and what did not find it. Measured, under the buggy version restored as a
mutation:

| Suite | Result |
|---|---|
| `core_test` (21 tests) | 4 fail |
| doc tests (7 tests) | **0 fail — all pass** |

Every doc example on this crate would have shipped against a `try_push` that
panics on any full multi-producer ring, because `Producer::try_push`'s own
example — like all seven — builds an SPSC ring. The single-backend example is
the right shape for documentation and the wrong shape for verification, and the
division of labour between them is stated in
`tests/manual/readme.md` C2 so that the doc tests are never cited as evidence
that the backends work.

#### Procedure — `try_push_batch`

Pull from the iterator, `try_push` each, stop at the first refusal, return the
count accepted.

**The refused record has already been taken from the iterator when the push
fails**, so a caller resuming from the same iterator resumes *after* the
refusal, not at it. That is a real edge — a caller who assumes otherwise
silently drops one record per refusal — and it is asserted rather than
described: `a_partial_batch_push_reports_its_count_and_consumes_the_refused_record`
checks the iterator's position, not only the count.

The alternative — peeking, or pushing the record back — would require either a
`Peekable` bound the caller may not want or a buffer this crate has no place
to put. Reporting the position honestly is cheaper than either.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The contract these procedures implement |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_three_way_storage_enum.md](../data_structure/001_three_way_storage_enum.md) | The enum dispatched over, and why it is not a trait |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_uniform_delivery_across_backends.md](../invariant/002_uniform_delivery_across_backends.md) | "A refused record comes back" — the property the seam preserves |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_mpsc/docs/api/001_producer_publish_surface.md`](../../../ring_mpsc/docs/api/001_producer_publish_surface.md) | `push`'s by-value signature, which the seam works around |
| [`ring_overflow/readme.md`](../../../ring_overflow/readme.md) | `would_resolve`, the shared step 5 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `a_refused_record_comes_back_on_every_backend` — the seam, at every backend rather than only the one the doc example uses |
| `tests/core_test.rs` | `a_partial_batch_push_reports_its_count_and_consumes_the_refused_record` — the iterator position |
| `tests/core_test.rs` | `drop_newest_discards_the_incoming_record_without_an_error` — step 5 |
| `tests/manual/readme.md` | C1 — the mutation, and the 4-versus-0 measurement |

### CO1 — The One Wildcard Arm Was on the Policy Enum, Where It Cost the Most

**What was found.** The dispatch discipline described above was real and had
exactly one hole. Every match on `Storage`, `EndsInner`, `ProducerInner` and
`ConsumerInner` named its arms; adding a backend broke the build in five places.
One did not — the fold of `ring_overflow::Resolution` down to *kept* or *handed
back* spelled its second case `_ =>`, so a `Resolution` variant added tomorrow
(say one meaning "block until room") would take that arm, come back to the
caller as a refusal, and compile without a warning.

**The asymmetry was the finding, not the wildcard.** Backends got compile
errors; policies got a default. Nothing marked that difference at the call site,
and the crate's own module documentation presented exhaustive matching as a
property of the file rather than of four of its five matches. Naming the two
remaining variants costs one line and moves the fifth match into the same class
as the other four:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
printf 'wildcard arms: %s\n' "$( grep -cE '^ +_ =>' src/lib.rs )"
command grep -m1 -B2 -A4 -F '      Err( record ) => match would_resolve( self.overflow )' src/lib.rs
```

Live output:

```
wildcard arms: 0
    {
      Ok( () ) => Ok( () ),
      Err( record ) => match would_resolve( self.overflow )
      {
        Resolution::DroppedIncoming => Ok( () ),
        Resolution::EvictedOldest | Resolution::Refused => Err( record ),
      },
```

`would_resolve` returns a `ring_overflow::Resolution`. One of its variants means
"the incoming record is gone" and the other two mean "hand it back", which is a
partition of three named things — not a case and a remainder — and the match now
spells it that way. The file has no wildcard arm left at all.

**What this does not fix is the reason the wildcard was tempting.** `Resolution`
belongs to `ring_overflow`, so a variant added there is a build error here only
for as long as somebody keeps choosing to make it one; nothing in either crate
records that this match is the seam where a new policy has to be considered. The
compile error is the notice, and it now exists.

**Disposition:** applied — the fold names `Resolution::EvictedOldest | Resolution::Refused` instead of `_`, so a fourth `Resolution` variant fails this crate's build exactly as a fourth backend already did, and the file's exhaustive-matching claim is now true of all five of its matches rather than four. Now prints: `wildcard arms: 0`

### CO2 — The Refusal Seam Is Three Different Shapes Underneath

The uniform `Result< (), T >` hides three unrelated refusal conventions:

| Backend | Its own refusal | What the arm must do |
|---------|-----------------|----------------------|
| `ring_spsc` | Returns the record | Pass it through |
| `ring_mpsc` | `Err( RingError )`, record consumed | Claim first, so the record was never moved (→ [`../pattern/002`](../pattern/002_claim_before_move_so_a_refusal_can_hand_the_record_back.md)) |
| `crossbeam` | `Result< (), T >` | Already the target shape |

Only the middle one needs work, and it is the reason `pattern/002` exists as a
named pattern rather than a comment. Recording the other two here keeps that
pattern's applicability condition honest — it is needed for one arm of three,
not because wrapping generally demands it.
