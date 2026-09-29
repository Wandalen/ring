# Invariant: What Survives a Refusal, and What Does Not

### Scope

**Purpose:** Measure the two boundaries `invariant/001`'s R1–R3 do not reach —
whether a refusal leaves the map itself untouched, and what an emptied registry
still owns after `len()` reports nothing.

**Responsibility:** The map's length and capacity across a refusal and across a
full removal; what the registry's eight methods can report about itself; and
which of R1–R3 the measurements confirm and which they leave with a blind spot.

**In Scope:** `ring_registry/docs/invariant/001_one_name_one_ring.md:14-16`,
`:33`, `:48`, `:58`; `ring_registry/src/lib.rs:105-235`.

**Out of Scope:** The invariants themselves and their enforcement are
[`invariant/001`](001_one_name_one_ring.md). The allocation the refusal does make
is [`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md).
The silent-drop route through `remove` is
[`pitfall/002`](../pitfall/002_the_remove_that_drops_a_ring_without_a_word.md).

---

## What the Invariants Say, and What the Registry Can Report

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
i=ring_registry/docs/invariant/001_one_name_one_ring.md
echo '  -- the three standing invariants, and the accounting one --'
command grep '^| R[0-9] |' "$i" | cut -c1-96 | sed 's/^/    /'
echo '  -- the witness the accounting invariant relies on --'
# Anchored at column 0: `invariant/001` quotes its own tables inside indented
# regenerate output further down, and an unanchored search finds those too.
command grep '^| E4 . .HashMap::len\|^\*\*E4 has no drift' "$i" | cut -c1-96 | sed 's/^/    /'
echo '  -- what a refused registration is asserted to leave unchanged --'
command grep 'V2 . A refused registration mutates' "$i" | cut -c1-96 | sed 's/^/    /'
echo '  -- every quantity the registry can report about itself --'
awk -v n1="$( command grep -n -m1 -F 'impl< T > Registry< T >' ring_registry/src/lib.rs | cut -d: -f1 )" -v n2="$(( $( command grep -n -m1 -F '    self.rings.keys().map( String::as_str )' ring_registry/src/lib.rs | cut -d: -f1 ) + 2 ))" 'NR >= n1 && NR <= n2 && /pub fn/ { print "    src/lib.rs:" NR ":" $0 }' ring_registry/src/lib.rs
echo '  -- and whether any of them is about storage rather than count --'
printf '    methods naming capacity, reserve, shrink or storage: %s\n' \
  "$( command grep -c -e 'pub fn capacity\|pub fn reserve\|pub fn shrink' ring_registry/src/lib.rs || true )"
```

Live output:

```
  -- the three standing invariants, and the accounting one --
    | R1 | Each live name maps to exactly one ring | Two rings answering to one name; a name resolvi
    | R2 | Each registered ring is owned by exactly one registry | A ring reachable from two places,
    | R3 | `len()` equals the number of live names, which equals the number of owned rings | A regis
  -- the witness the accounting invariant relies on --
    | E4 | `HashMap::len` is the only source of `len()` | R3 | None — there is no second counter t
    **E4 has no drift because there is no second number.** `len()` forwards to the
  -- what a refused registration is asserted to leave unchanged --
    | V2 | A refused registration mutates the map (R3) | `a_second_registration_under_a_live_name_is
  -- every quantity the registry can report about itself --
    src/lib.rs:105:  pub fn new() -> Self
    src/lib.rs:157:  pub fn register
    src/lib.rs:189:  pub fn get_mut( &mut self, name : &str ) -> Option< &mut Split< T > >
    src/lib.rs:199:  pub fn remove( &mut self, name : &str ) -> Option< Split< T > >
    src/lib.rs:206:  pub fn contains( &self, name : &str ) -> bool
    src/lib.rs:213:  pub fn len( &self ) -> usize
    src/lib.rs:220:  pub fn is_empty( &self ) -> bool
    src/lib.rs:232:  pub fn names( &self ) -> impl Iterator< Item = &str >
  -- and whether any of them is about storage rather than count --
    methods naming capacity, reserve, shrink or storage: 0
```

## Length and Capacity Across Both Boundaries

`Registry` exposes no capacity, so the measurement runs against the same
`HashMap< String, V >` it wraps, with `V` the width of a `Split< T >`, under a
counting global allocator:

```rust
// src/bin/refusal_totality.rs
let mut m : HashMap< String, V > = HashMap::new();
for i in 0 .. 8 { m.insert( format!( "ring_{i}" ), [ 0; 48 ] ); }

// A refusal: the key is present, so the Entry match takes the occupied arm.
reset();
let name = "ring_3".to_string();
let refused = match m.entry( name )
{
  Entry::Occupied( o ) => Err( o.key().clone() ),
  Entry::Vacant( v ) => { v.insert( [ 0; 48 ] ); Ok( () ) },
};
```

```
    after 8 registrations       len  8  capacity 14
    after a refusal             len  8  capacity 14   allocations 2, 12 bytes, refused true
    after removing all eight    len  0  capacity 14   remove allocated 0, 0 bytes
    is_empty() reports true, and the table still holds 14 slots
```

---

### RG23 — The Refusal Is Total on the Map, Which Is Stronger Than What V2 Asserts

V2 detects "a refused registration mutates the map" through
`a_second_registration_under_a_live_name_is_refused`, which asserts `len() == 1`.
Length is a coarse witness: a refusal that replaced one ring with another, or
rehashed the table, or reserved for a slot it did not use, would all leave the
length untouched.

Measured, none of that happens. Across a refusal on a map holding eight names,
length stays 8 and capacity stays 14 — the table is not grown, not rehashed, not
touched. The only allocations are the two 6-byte `String`s of
[RG1](../algorithm/001_two_branches_and_what_the_refusal_costs.md): the name
materialized from `impl Into< String >` and the key cloned back out of the
occupied entry. Neither belongs to the map. Combined with the drop-counter test
that already establishes no ring is dropped, the refusal is total on the
registry's own state: the caller comes away with an error and their ring, and the
registry is exactly what it was.

That is a better property than V2 claims and it is worth claiming, because it is
what makes the refusal *retryable*. A caller can register under a different name
immediately, in the same expression chain, with no reasoning about what the
failed attempt left behind. Nothing in the invariant table says so.

**Finding.** Recorded as an invariant the crate holds and does not state. R1–R3
are all about what a registry contains; none is about what an operation
*preserves*. One row — a refused registration leaves the map identical, allocating
only for the name it reports — states the transactional property directly, and it
is checkable with `capacity()` rather than only `len()`, which is what makes it
stronger than V2.

---

### RG24 — `is_empty()` Reports Nothing While the Registry Still Holds Fourteen Slots

R3 is the accounting invariant: "`len()` equals the number of live names, which
equals the number of owned rings." E4 explains why it cannot drift — "`len()`
forwards to the map… there is no second counter." Both are true.

There is a second *quantity*, and it is not a counter. After eight registrations
and eight removals, `len()` is 0, `is_empty()` is `true`, and the map still holds
a fourteen-slot table. `remove` releases the ring — measured, it allocates
nothing and the drop test proves ownership transfers — but it never releases the
storage. All eight of the registry's methods report counts, names, or rings;
`command grep` finds zero methods naming capacity, reserve or shrink. So the
registry can be asked what it holds and cannot be asked what it occupies, and
there is no way to give the table back short of dropping the whole registry.

The scale is small and the consequence is not a leak — the memory is reachable,
accounted, and freed on drop. What it is, is a fourth number in a crate whose
central document is about accounting, present in no invariant and reportable by
no method. A long-lived registry that cycles many short-lived names — the
`removing_a_name_frees_it_for_reuse` pattern, repeated — grows a table it never
shrinks, and `is_empty()` will keep saying `true`.

**Finding.** Recorded as an unaccounted resource rather than a defect. R3 should
say what it is about: the *count* accounting is exact and the *storage*
accounting is unstated. Whether to expose it is a separate question with a real
answer either way — `HashMap` offers `shrink_to_fit`, and a registry that never
holds more than a handful of names does not need it, which is the same argument
[`decisions/002`](../decisions/002_three_pending_questions_and_the_one_consumer.md)
makes about the map type. The version worth writing down is the observation, not
the method.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](001_one_name_one_ring.md) | R1–R3, E4 and V2 — the statements this measures |
| [`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md) | The two allocations a refusal does make |
| [`pitfall/002`](../pitfall/002_the_remove_that_drops_a_ring_without_a_word.md) | The other thing `remove` leaves behind |
| [`api/001`](../api/001_the_registry_surface.md) | The eight methods, and what none of them reports |
| [`decisions/002`](../decisions/002_three_pending_questions_and_the_one_consumer.md) | The population at which none of this matters |

### Sources

| Fact | Where |
|------|-------|
| R1–R3 | `.../invariant/001_one_name_one_ring.md:14-16` |
| E4, the single-counter argument | `.../invariant/001_one_name_one_ring.md:33`, `:48` |
| V2 and its `len()` witness | `.../invariant/001_one_name_one_ring.md:58` |
| The eight methods, none about storage | Census above |
| Capacity unchanged across a refusal | Probe above, two runs |
| Capacity 14 retained at `len() == 0` | Probe above, two runs |

### Tests

| Test | Covers |
|------|--------|
| `a_second_registration_under_a_live_name_is_refused` | V2, with `len()` as the witness |
| `a_refused_registration_does_not_drop_the_ring_already_there` | The other half of the refusal's totality |
| `removing_a_name_frees_it_for_reuse` | The cycle that grows a table nothing shrinks |
