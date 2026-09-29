# Pattern: The Half-Open Range as a Value

### Scope

- **Purpose**: Name the pattern `Claim` implements — a granted range as a plain `Copy` value with no reach into the ring — and place it among the family's seven range-shaped types.
- **Responsibility**: State the half-open convention and what it buys, then account for the three independent implementations of the value shape and for the one rule that governs when a range carries `#[ must_use ]`.
- **In Scope**: The three value ranges (`Claim`, `ring_batch::BatchClaim`, `ring_consume::Available`) and the four guard ranges that carry a borrow and a destructor instead.
- **Out of Scope**: Why this crate cannot use the guard shape — see [`decisions/002`](../decisions/002_must_use_without_drop.md).

### The Convention

`src/lib.rs:80-82` states it on the type:

> Half-open: `start..end`, so an empty claim and a one-slot claim are not the
> same value, and `end` is directly the sequence the producer cursor now sits
> at.

Two justifications, and the second is the load-bearing one. Under a closed
convention (`start..=last`) a zero-width range has no representation — `last`
would have to be `start - 1`, which underflows at `Seq::ZERO`. But the clause
that earns the convention its place is `end` *being* the next cursor value:
`claim`'s `compare_exchange( current, next, … )` uses exactly that number, so
the type's accessor and the algorithm's arithmetic are the same expression.

Three consequences follow mechanically, and each shows up as a test:

| Consequence | Where asserted |
|-------------|----------------|
| `end` is exclusive — `contains( end )` is false | doctest `:184`, labelled `"half-open"` |
| adjacent ranges do not overlap | `adjacent_claims_do_not_overlap:99` |
| an empty range contains nothing and overlaps nothing | `an_empty_claim_contains_nothing…:56` |

`ring_consume` adopts the convention and cites this crate for it
(`ring_consume/src/lib.rs:85-86`):

> Half-open, like `ring_claim::Claim`, and for the same reason: `end` is
> directly the sequence to commit once the run has been read.

That is the only cross-crate citation of a convention anywhere in the family,
and it is exact: consumer-side, `end` is the sequence to commit, the mirror of
the producer-side clause it names.

### CL41 — The Value Shape Is Implemented Three Times, and No Two Agree on How

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -r -A6 '^pub struct' ring_*/src/*.rs | command grep -B1 'start : Seq' | command grep -E 'pub struct|start'
command grep -r -i 'half-open' ring_*/src/*.rs
```

Live output:

```
ring_batch/src/lib.rs-  start : Seq,
ring_claim/src/lib.rs-  start : Seq,
ring_consume/src/lib.rs-  start : Seq,
ring_mpsc/src/lib.rs-  start : Seq,
ring_spsc/src/lib.rs-  start : Seq,
ring_claim/src/lib.rs:/// Half-open: `start..end`, so an empty claim and a one-slot claim are not the
ring_claim/src/lib.rs:  /// assert!( !claim.contains( Seq( 6 ) ), "half-open" );
ring_consume/src/lib.rs:/// Half-open, like `ring_claim::Claim`, and for the same reason: `end` is
```

Five types carry a `start : Seq` field. Three are plain values:

| | `Claim` | `BatchClaim` | `Available` |
|--|---------|--------------|-------------|
| Crate / tier | `ring_claim` / 5 | `ring_batch` / 2 | `ring_consume` / 5 |
| Line | 96 | 56 | 99 |
| Width field | `len : usize` | `count : usize` | **`len : u64`** |
| Receiver | `self` | **`&self`** | `self` |
| Derives | `Debug, Clone, Copy, PartialEq, Eq` | identical | identical |
| Size | 16 bytes | 16 bytes | 16 bytes |
| Methods | 8 | 8 | **6** |
| `const fn` | 7 of 8 | 7 of 8 | 5 of 6 |
| `contains` / `overlaps` | ✔ ✔ | ✔ ✔ | **✘ ✘** |
| Type-level `#[ must_use ]` | **messaged** | none | none |

Same concept, same size, three different answers on nearly every axis, and not
one of the three files mentions that the other two exist — `ring_consume:85` is
a citation of the *convention*, not of the type.

**Two of the differences are defensible.**

`Available`'s missing `contains` and `overlaps` are the right call. Those two
predicates exist to let a test assert that no two producers hold one sequence
([`invariant/001`](../invariant/001_no_two_producers_hold_one_sequence.md)) —
exclusivity is a producer-side property, and a consumer that reads a run twice
has not broken anything. The consumer-side type correctly ships the four
accessors and the iterator and stops.

`Available`'s `len : u64` against the other two's `usize` follows the same
logic: it is compared against `Seq`'s inner `u64` in `Consumer::available_up_to(
max : u64 )`, so `usize` would put a cast at the boundary instead of inside it.

**One is not.** `BatchClaim`'s `&self` receiver on a 16-byte `Copy` type passes
an 8-byte pointer to reach 16 bytes of data — strictly worse, in the crate two
tiers *closer* to the primitive, and the only one of the three to do it. It is
also what forces `overlaps( &self, other : &Self )`, so the argument type
diverges as a consequence rather than a choice.

The `const` gap runs the other way and is the interesting one: `BatchClaim`
reaches 7 of 8 by comparing `Seq`'s public `.0` directly, where `Claim` compares
`Seq` values through `PartialOrd` and cannot be `const`. That trade — `const`
against respecting the newtype — is examined in
[`item/001`](../item/001_the_eight_readings_of_a_range.md).

### CL42 — Annotation Tracks the Cost of Dropping, for Five of Seven, and the Two Gaps Are Both Twins of an Annotated Type

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -rn 'impl.*Drop for' ring_*/src/*.rs | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
command grep -rn -B2 'pub struct \(Claim\|BatchClaim\|Available\|Reservation\|Reserved\|Batch\)' \
  ring_*/src/*.rs | grep -E 'must_use|pub struct' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
ring_mpsc/src/lib.rs:impl< S > Drop for Reserved< '_, S >
ring_mpsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Reservation< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_batch/src/lib.rs:pub struct BatchClaim
ring_claim/src/lib.rs-95-#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
ring_claim/src/lib.rs:pub struct Claim
ring_claim/src/lib.rs:pub struct Claimer< 'a >
ring_consume/src/lib.rs:pub struct Available
ring_mpsc/src/lib.rs:pub struct Reserved< 'a, S >
ring_mpsc/src/lib.rs:pub struct Batch< 'a, S >
ring_spsc/src/lib.rs-731-#[ must_use = "a reservation publishes on drop; dropping it immediately publishes an unwritten slot" ]
ring_spsc/src/lib.rs:pub struct Reservation< 'a, S >
ring_spsc/src/lib.rs-960-#[ must_use = "a batch commits on drop; dropping it immediately discards the records it covers" ]
ring_spsc/src/lib.rs:pub struct Batch< 'a, S >
```

The other four range types are guards — a borrow, a position, and a destructor:

| Type | Crate | Tier | Line | `Drop` does | `must_use` |
|------|-------|-----:|-----:|-------------|------------|
| `Reservation< 'a, S >` | `ring_spsc` | 6 | 733 | publishes (`:788`) | **messaged** |
| `Batch< 'a, S >` | `ring_spsc` | 6 | 962 | commits (`:1130`) | **messaged** |
| `Reserved< 'a, S >` | `ring_mpsc` | 6 | 933 | publishes (`:985`) | none — **argued** |
| `Batch< 'a, S >` | `ring_mpsc` | 6 | 1143 | commits (`:1257`) | none |

Across all seven, one rule explains the annotations, and it is not "value types
get one":

| Type | What dropping it costs | Annotated | Absence argued |
|------|------------------------|:---------:|:--------------:|
| `Claim` | a permanent stall — every consumer stops | ✔ | — |
| `Reservation` (spsc) | publishes an unwritten slot | ✔ | — |
| `Batch` (spsc) | discards the records it covers | ✔ | — |
| `Available` | **nothing** — the run stays available | ✘ | n/a |
| `Reserved` (mpsc) | one empty record, a defined outcome | ✘ | ✔ **five lines** |
| **`Batch` (mpsc)** | **discards the records it covers** | **✘** | **✘** |
| **`BatchClaim`** | **a permanent stall** | **✘** | **✘** |

`ring_mpsc:927-931` is the proof that the rule is deliberate rather than
accidental — it spends five lines justifying an *absence*:

> **A guard dropped without a write publishes an empty slot, not a torn one.**
> The slot was left `Default` by the consumer that drained it, so a panic
> between claim and write costs one empty record — an observable, defined
> outcome rather than undefined behaviour. That is why no completion flag is
> tracked: there is nothing for it to prevent.

So the family reasons about this explicitly when it chooses to, which makes the
two unannotated-and-unargued rows harder to read as intentional. Both are twins
of an annotated type:

- **`ring_mpsc::Batch` against `ring_spsc::Batch`** — identical fields
  (`ring`, `start`, `len`), identical Drop semantics (commit, releasing the
  slots), same tier. `ring_spsc:960` carries *"a batch commits on drop; dropping
  it immediately discards the records it covers"*. `ring_mpsc:1142` carries
  `#[ derive( Debug ) ]` and nothing else. The sentence that describes the
  hazard is already written, in the sibling crate, and applies verbatim.
- **`ring_batch::BatchClaim` against `Claim`** — the sharper of the two, because
  the cost is the highest on the table. `BatchClaim`'s seven annotated *methods*
  mean discarding the result of `start()` warns while discarding **the claim
  itself** does not, which inverts the priority. And `BatchClaim` has no
  destructor either, so the lint is the only mechanism available — the
  combination this crate documents as its second decision
  ([`decisions/002`](../decisions/002_must_use_without_drop.md)), minus the one
  half that combination rests on.

With `claim_gated`'s missing single-producer clause
([`decisions/001`](../decisions/001_compare_exchange_rather_than_fetch_add.md)),
`ring_batch` now carries two documentation gaps that `ring_claim` closed for its
own equivalent, three tiers up, where a `ring_batch` reader has no reason to
look.

### The Split Is by Tier, and Nothing Says So

| Shape | Types | Tiers |
|-------|------:|-------|
| Value — `Copy`, 16 bytes, no destructor | 3 | 2, 5, 5 |
| Guard — borrow + generic + `Drop` | 4 | 6, 6, 6, 6 |

Clean, and it is not about ergonomics — it is about what the type can reach. A
guard publishes on drop, publishing requires a ring, and reaching a ring
requires a borrow and a slot type. Every type that *can* hold a ring does; every
type that cannot is a bare value with a lint attached.

`ring_spsc:617-620` argues for the guard while naming this crate's shape as the
alternative it rejected:

> The guard shape rather than a bare `claim`/`publish` pair, because an early
> return between the two wedges the ring permanently and no runtime check can
> distinguish "claimed and about to publish" from "claimed and abandoned" …
> Making the publish the drop makes the case unreachable, including on unwind.

Read without the tier column that is a criticism of `ring_claim`. With it, it is
a Tier 6 crate correctly taking a shape that is not available at Tier 5 — a
`Claimer` has no ring to borrow, which is the whole point of it being a
primitive. The rule is consistent across all seven types and is stated nowhere.

### What the Value Shape Buys

Recorded because the guard shape is argued twice in the family and the value
shape is argued nowhere:

| Property | Consequence |
|----------|-------------|
| `Copy` | claims can be stored, compared, and passed freely; `no_two_producers_are_ever_granted_the_same_sequence` collects `Vec< Vec< Claim > >` — 2,000 per thread, 8,000 total — and flattens after the join |
| No borrow | those claims outlive the `thread::scope` that produced them, which is what makes the partition assertion expressible at all |
| No destructor | dropping is free and cannot panic, so a producer's unwind path has nothing to run |
| No generic | `Claim` is one type, not one per slot type — so the exhaustive 900-pair test constructs both sides from literals, with no ring in scope |

The second and fourth are load-bearing for the test suite specifically. A guard
cannot be collected across a scope boundary, and a generic range type would make
`Claim::new( Seq( a_start ), a_len )` inside a quadruple-nested loop
substantially more awkward
([`item/001`](../item/001_the_eight_readings_of_a_range.md)).

### Patterns

| File | Relationship |
|------|--------------|
| [001_retrying_against_a_moving_target.md](001_retrying_against_a_moving_target.md) | The loop that produces one of these |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md) | The sixteen bytes all three value types occupy |
| [../data_structure/002_the_borrow_that_is_half_the_type.md](../data_structure/002_the_borrow_that_is_half_the_type.md) | The borrow the guard shape carries and this one does not |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_must_use_without_drop.md](../decisions/002_must_use_without_drop.md) | Why Tier 5 cannot adopt the guard shape |
| [../decisions/001_compare_exchange_rather_than_fetch_add.md](../decisions/001_compare_exchange_rather_than_fetch_add.md) | `ring_batch`'s other undocumented gap |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_eight_readings_of_a_range.md](../item/001_the_eight_readings_of_a_range.md) | The eight methods, and where `BatchClaim` bought two more `const` |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_no_two_producers_hold_one_sequence.md](../invariant/001_no_two_producers_hold_one_sequence.md) | Why `contains`/`overlaps` are producer-side only |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:78-100` | The convention, stated on the type, with the messaged annotation |
| `ring_batch/src/lib.rs:50-168` | The twin — same eight names, no type-level annotation |
| `ring_consume/src/lib.rs:69-185` | The third value range, and the family's one convention citation |
| `ring_spsc/src/lib.rs:617-620, 679-681, 890-899` | The guard shape, argued, naming this crate's alternative |
| `ring_mpsc/src/lib.rs:920-931, 1137-1142` | An argued absence, and an unargued one |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:55` — `a_claim_is_half_open` | The convention, asserted directly |
| `tests/claim_test.rs:99` — `adjacent_claims_do_not_overlap` | Its first consequence |
| `tests/claim_test.rs:69` — `an_empty_claim_contains_nothing…` | Its second |
| `tests/claim_test.rs:320` — `no_two_producers_are_ever_granted_the_same_sequence` | What `Copy` and no-borrow make expressible |
