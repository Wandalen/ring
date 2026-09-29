# Workaround: The Door Ring Core Does Not Open

### Scope

- **Purpose**: Record `check_ends` as an absorbed external constraint rather than a design choice, with its cost and the condition that deletes it.
- **Responsibility**: What is being routed around, what the route costs, and what would remove the need for it.
- **In Scope**: `check_ends`; the accessor `ring_core` does not offer; the capability gap the workaround leaves open.
- **Out of Scope**: The signature itself (→ [`api/001`](../api/001_the_check_surface.md)); whether `ring_core` should change (→ [`decisions/001`](../decisions/001_four_edges_not_two.md)).

### Constraint

**`ring_core::Producer` and `ring_core::Consumer` expose no cursor.** They wrap a
`ring_spsc` or `ring_mpsc` ring, each of which holds a `CursorPair` as a private
field with no accessor, and neither wrapper forwards one.

This is a constraint from outside this crate: `ring_core` is one of the five
crates on the family's export Contract, and widening its surface for a
non-exported consumer is a decision that belongs there, not here.

### What It Blocks

The family's own entry point cannot use the family's own checker. A caller
holding a `ring_core` split — which is what `ring_factory::build` hands out, and
therefore what the Contract says a caller has — cannot call `check` and cannot
construct a `Watch`. Both take a `&CursorPair`, and there is no route from a
`ring_core` end to one.

Without a workaround, this crate would be reachable only by callers who had gone
*below* the Contract to `ring_spsc` or `ring_mpsc` directly — which is to say, by
nobody following the family's own documented path.

### The Workaround

`check_ends( capacity, &Producer, &Consumer )` — a third entry point built
entirely from what the ends *do* expose: `consumer.len()` and
`producer.free_capacity()`. If those two plus nothing else account for the whole
ring, the ends agree; if not, they do not.

```rust
let pending = consumer.len();
let free = producer.free_capacity();
if pending + free == capacity.get() { return Ok( () ); }
Err( Violation::ReadingsDisagree { pending, free, capacity : capacity.get() } )
```

### Cost

| # | Cost | Detail |
|---|---|---|
| W1 | **D1 is undetectable through this door** | Both readings are products of the saturating arithmetic that masks a consumer-ahead cursor; the two derived numbers still sum correctly for a ring `check` would reject outright |
| W2 | A quiescence precondition | The two reads are separate loads of two different moments, so a concurrent writer produces false positives — `api/001`'s B1 |
| W3 | The caller must supply the capacity | Nothing on a `ring_core` end reports it, so the one number the check is measured against comes from outside the thing being checked — which is what `ReadingsDisagree` reports when it is wrong |
| W4 | A fourth `Violation` variant | `ReadingsDisagree` exists only for this door and carries no `Seq`, making it the one variant whose payload cannot reconstruct the pair that produced it |

**W1 is the expensive one and it is not a shortcoming of the implementation.**
There is no way to detect D1 from derived readings, because deriving them is what
destroys the evidence. A better `check_ends` is not possible; a different door is.

### Deletion Condition

**`ring_core` exposing `position() -> Seq` on both ends.** At that point `check`
and `Watch` become reachable from the Contract, `check_ends` becomes a
convenience rather than the only option, and W1 stops being a capability gap.

This is `decisions/`'s Pending 1, deferred there rather than decided here — and
`ring_spsc` already offers exactly that accessor on both of its own ends, so the
change is a forward rather than an invention.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- what a ring_core end exposes --'
command grep -E '^  pub fn ' ring_core/src/lib.rs | sed 's/^/  /'
echo '-- and what ring_spsc already exposes that ring_core does not forward --'
printf 'ring_spsc position():  %s\n' "$( command grep -c 'pub fn position' ring_spsc/src/lib.rs || true )"
printf 'ring_core position():  %s\n' "$( command grep -c 'pub fn position' ring_core/src/lib.rs || true )"
echo '-- the workaround, and the two readings it is built from --'
command grep 'consumer.len()\|producer.free_capacity()' ring_debug/src/lib.rs
```

Live output:

```
-- what a ring_core end exposes --
    pub fn new( config : &RingConfig ) -> Result< Self, RingError >
    pub fn new_crossbeam( config : &RingConfig ) -> Result< Self, RingError >
    pub fn capacity( &self ) -> Capacity
    pub fn ends( &mut self ) -> Ends< '_, T >
    pub fn split( &'a mut self ) -> ( Producer< 'a, T >, Consumer< 'a, T > )
    pub fn try_push( &mut self, record : T ) -> Result< (), T >
    pub fn try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize
    pub fn try_clone( &self ) -> Option< Producer< 'a, T > >
    pub fn free_capacity( &self ) -> usize
    pub fn is_full( &self ) -> bool
    pub fn try_recv( &mut self ) -> Option< T >
    pub fn try_recv_batch( &mut self, out : &mut Vec< T > ) -> usize
    pub fn len( &self ) -> usize
    pub fn is_empty( &self ) -> bool
-- and what ring_spsc already exposes that ring_core does not forward --
ring_spsc position():  2
ring_core position():  0
-- the workaround, and the two readings it is built from --
  let pending = consumer.len();
  let free = producer.free_capacity();
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | The third entry point, and preconditions B1 and B2 |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_reaching_the_cursors_of_a_live_ring.md](../integration/001_reaching_the_cursors_of_a_live_ring.md) | The reachability problem this routes around |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_four_edges_not_two.md](../decisions/001_four_edges_not_two.md) | DB14 — the deferral that keeps this workaround alive |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `check_ends` |

### Tests

| Test | Relationship |
|------|--------------|
| `the_two_ends_of_a_live_ring_agree` | The workaround working on a healthy ring |
| `check_ends_cannot_see_the_corruption_check_can` | W1, asserted directly — the capability gap is a test, not a caveat |
| `a_ring_measured_against_the_wrong_capacity_disagrees` | W3 — the externally-supplied number being wrong is itself reportable |

### DB17 — the crate's central limitation is a test, and that is unusual enough to record

`check_ends_cannot_see_the_corruption_check_can` asserts that this crate's main
door misses this crate's most important defect. It is a passing test whose subject
is an incapability.

That shape is worth naming because the alternative shapes are all worse. A caveat
in prose drifts from the code; a `#[ ignore ]`d test is dead; an issue is
elsewhere. **A passing test that pins a limitation converts "we know about this"
into "this is still exactly as bad as we said"** — and it fails the moment the
limitation is fixed, which is the correct time to be told.

The family does this in one other place and nowhere else: `ring_bench` measures
the producer ceiling its Contract door imposes rather than asserting it. Both
crates arrived at the shape independently, and neither names it as a technique.

### DB18 — a workaround with no consumer has an unfalsifiable cost

W1 through W4 are stated as costs, and three of the four are measured. W1's real
cost is not: it is what a caller loses by using the only door available to them,
and this crate has no callers.

`ring_debug` is named by no other crate in the family — not in code, not in any
manifest. So the door that exists because the Contract path could not reach the
other two is a door nobody has walked through, and the constraint it absorbs has
never been felt by the consumer it was absorbed for.

Recorded as an observation rather than a defect. **The workaround is correct and
its motivation is real; what is unverified is the premise that a consumer would
have taken the Contract path** — and until one exists, `check_ends`'s cost is a
prediction, not a measurement, in a corpus where nearly everything else is the
other way round.

