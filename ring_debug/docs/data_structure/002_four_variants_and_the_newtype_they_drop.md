# Data Structure: Four Variants and the Newtype They Drop

### Scope

- **Purpose**: Describe `Violation`'s four payloads, and record what each keeps and what each discards on the way out.
- **Responsibility**: Field-by-field layout of the reported value, and the type-level information lost at the boundary.
- **In Scope**: The four variants' fields and their types; the `Capacity`-to-`usize` unwrap; what `ReadingsDisagree` carries that the others do not.
- **Out of Scope**: Which entry point produces which variant (→ [`api/001`](../api/001_the_check_surface.md)); the semantics of each defect (→ [`invariant/001`](../invariant/001_cursor_invariants_over_a_live_ring.md)); the enum's openness to extension (→ [`item/002`](../item/002_what_the_crate_does_not_declare.md)).

### Abstract

Four struct variants, eleven fields between them, and one design rule applied
consistently: **a `Violation` carries the numbers it was derived from, so a
caller never has to re-read the thing that was already wrong.** Re-reading is
precisely what a diagnostic must not do — the second read is a different moment,
and a fault that has since been overwritten reports as healthy.

Applied without exception. It is also where the crate's one type-level
concession lives.

### Data Structures

| Variant | Fields | Produced by |
|---|---|---|
| `ConsumerAheadOfProducer` | `producer : Seq`, `consumer : Seq` | `check`, `Watch::new`, `Watch::observe` |
| `ProducerLappedConsumer` | `producer : Seq`, `consumer : Seq`, `capacity : usize` | `check`, `Watch::new`, `Watch::observe` |
| `CursorWentBackwards` | `cursor : Cursor`, `was : Seq`, `now : Seq` | `Watch::observe` only |
| `ReadingsDisagree` | `pending : usize`, `free : usize`, `capacity : usize` | `check_ends` only |

**Three carry `Seq`, one carries none.** `ReadingsDisagree` is the odd variant in
every way that matters: it is the only one whose numbers are *derived* rather than
read — `pending` is `consumer.len()` and `free` is `producer.free_capacity()`,
each a computed value from the far side of a `ring_core` boundary that does not
expose cursors at all. It is also the only variant whose three numbers cannot be
used to reconstruct the cursor pair that produced them.

That asymmetry is not a flaw in the variant; it is the shape of the only
information available at that door, and it is why `check_ends` cannot see D1 (→
[`integration/001`](../integration/001_reaching_the_cursors_of_a_live_ring.md)).

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
S=ring_debug/src/lib.rs
echo '-- the enum as declared --'
command grep -m1 -A2 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]' $S
echo '-- every field of every variant --'
command grep -m1 -A60 -F 'pub enum Violation' $S | command grep -E '^\s+(producer|consumer|capacity|cursor|was|now|pending|free) :'
echo '-- what the payloads are typed as --'
printf 'fields typed Seq:       %s\n' "$( command grep -m1 -A60 -F 'pub enum Violation' $S | command grep -cE ': Seq,?$' )"
printf 'fields typed usize:     %s\n' "$( command grep -m1 -A60 -F 'pub enum Violation' $S | command grep -cE ': usize,?$' )"
printf 'fields typed Capacity:  %s\n' "$( command grep -m1 -A60 -F 'pub enum Violation' $S | command grep -cE ': Capacity,?$' || true )"
echo '-- and where the newtype is unwrapped --'
command grep 'capacity : capacity.get()\|capacity : capacity' $S
echo '-- is any payload a reference? a lifetime on the enum would show here --'
command grep -cE '^pub enum Violation<' $S || true
```

Live output:

```
-- the enum as declared --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub enum Violation
{
-- every field of every variant --
    producer : Seq,
    consumer : Seq,
    producer : Seq,
    consumer : Seq,
    capacity : usize,
    cursor : Cursor,
    was : Seq,
    now : Seq,
    pending : usize,
    free : usize,
    capacity : usize,
-- what the payloads are typed as --
fields typed Seq:       6
fields typed usize:     4
fields typed Capacity:  0
-- and where the newtype is unwrapped --
      Violation::ProducerLappedConsumer { producer, consumer, capacity : capacity.get() }
  Err( Violation::ReadingsDisagree { pending, free, capacity : capacity.get() } )
-- is any payload a reference? a lifetime on the enum would show here --
0
```

### Types

| File | Relationship |
|------|--------------|
| [../type/001_violation.md](../type/001_violation.md) | The same enum as a *type* — its traits and its role in the surface |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_cursor_invariants_over_a_live_ring.md](../invariant/001_cursor_invariants_over_a_live_ring.md) | D1–D3, one per variant, and the fourth that is not a D |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_reaching_the_cursors_of_a_live_ring.md](../integration/001_reaching_the_cursors_of_a_live_ring.md) | Why `ReadingsDisagree` carries derived numbers rather than cursors |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The enum and its two `impl` blocks |

### Tests

| Test | Relationship |
|------|--------------|
| `a_violation_reports_the_numbers_it_was_derived_from` | DB8 — the rule, asserted directly |
| `a_ring_measured_against_the_wrong_capacity_disagrees` | DB7 — a `usize` capacity that is wrong is exactly what `ReadingsDisagree` is for |
| `a_backwards_consumer_is_named_as_the_consumer` | The one variant whose payload includes no number from the ring's own arithmetic |

### DB7 — `Capacity` is validated on the way in and `usize` on the way out


`check` takes a `&CursorPair`, whose `capacity()` is a `Capacity` — the family's
validated newtype, constructible only through `Capacity::new`, which rejects zero
and non-powers-of-two. `check_ends` takes a `Capacity` directly. `Watch` stores
one.

Not one of the four variants holds a `Capacity`. Both variants that report a
capacity hold a bare `usize`, unwrapped via `capacity.get()` at the two
construction sites.

**This is deliberate for `ReadingsDisagree` and incidental for
`ProducerLappedConsumer`, and the two cases have not been distinguished.** For
`ReadingsDisagree` the `usize` is right: the whole point of that variant is that
the *stated* capacity and the *observed* one disagree, and typing the stated one
as validated would suggest a guarantee the variant exists to report the absence
of. For `ProducerLappedConsumer` the capacity is simply the ring's real, already
validated capacity, and the unwrap loses that for no gain — a downstream tool
matching on the variant gets a number it must re-validate to pass anywhere else
in the family.

The consequence is small and real: `Violation` is the one public type in this
crate that names no other family type except `Seq` and `Cursor`, so a caller
reacting to a violation cannot hand its capacity back to any family constructor
without going through `Capacity::new` and handling a failure that cannot happen.

### DB8 — every payload is a copy taken at detection time, and this is the crate's load-bearing rule


Eleven fields, and all eleven are values captured at the moment the comparison
failed. No variant holds a reference, a handle, or an index the caller would have
to re-resolve. `Violation` derives `Copy`.

**This is the one design rule in the crate that would be expensive to lose**, and
unlike DB5 and DB6 it is applied deliberately and completely. A `Violation`
carrying `&CursorPair` instead of the two sequences would be smaller and would
compile — and would report whatever the pair says when the caller finally looks,
which for a live ring is a different pair state and quite possibly a healthy one.
The bug it prevents is the one where a diagnostic tool prints "no violation
detected" about a `Violation` value it is holding in its hand.

The rule also has a cost worth naming: `ProducerLappedConsumer` carries three
numbers where two would identify the fault, because the third is needed to
*explain* it. The variants are sized for the reader rather than for the
comparison, and `ReadingsDisagree` carries a `capacity` that its own two other
fields must sum to — a redundancy on purpose.
