# Pattern: Proof by Monomorphisation

### Scope

**Purpose:** Establish how this crate's suite proves the identical-path claim —
by writing one generic body and instantiating it at both shapes rather than by
asserting the same thing twice — and mark exactly where that proof stops.

**Responsibility:** The two generic test helpers, the three tests that call them,
the thirteen that name a shape, and the case where one source body returns
opposite answers.

**In Scope:** `ring_event/tests/event_test.rs:24-43`, `:53`, `:60`, `:67`;
`ring_event/src/lib.rs:7-9`; probe below.

**Out of Scope:** The state that makes the divergence possible is
[`pitfall/001`](../pitfall/001_a_zero_length_payload_reads_as_nothing.md). The
shape of the traits themselves is
[`pattern/001`](001_a_trait_on_the_varying_side_and_one_function_over_it.md).

---

## The Mechanism, and How Far It Reaches

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the helper the suite calls the whole point --'
command grep -m1 -B1 -A18 -F '/// The whole point: one body, no shape named, used by both shapes below.' ring_event/tests/event_test.rs
echo '  -- which tests reach one, against how many tests there are --'
awk '/^fn /{ f = $0; hit = 0 } /round_trip\(|land_and_read\(/{ if ( f != "" && !hit ) { print "  " f; hit = 1 } }' ring_event/tests/event_test.rs
command grep -c '^#\[ test \]' ring_event/tests/event_test.rs || true
echo '  -- and the shapes the remaining tests name explicitly --'
command grep -c 'TypedSlot::<\|BytesSlot::<' ring_event/tests/event_test.rs || true
```

Live output:

```
  -- the helper the suite calls the whole point --

/// The whole point: one body, no shape named, used by both shapes below.
fn round_trip< S, P >( slot : &mut S, payload : P ) -> Option< S::Out< '_ > >
where
  S : Peek,
  P : Fill< S >,
{
  publish_into( slot, payload ).expect( "the payload fits" );
  drain_from( slot )
}

/// The same, but through real storage addressed by a sequence — the shape the
/// ring itself uses.
fn land_and_read< S, P >( buffer : &mut Buffer< S >, seq : Seq, payload : P ) -> bool
where
  S : Peek + Slot + Default,
  P : Fill< S >,
{
  publish_into( buffer.at_mut( seq ), payload ).is_ok() && drain_from( buffer.at( seq ) ).is_some()
}
  -- which tests reach one, against how many tests there are --
  fn one_generic_body_round_trips_a_typed_slot()
  fn the_same_generic_body_round_trips_a_bytes_slot()
  fn both_shapes_land_in_storage_through_the_same_two_calls()
17
  -- and the shapes the remaining tests name explicitly --
13
```

## The Same Body, Two Answers

*The probe below is frozen at authoring time — the scratch crate that ran it
has since been swept — and no drift was found: `round_trip`'s signature and
the `Peek`/`Fill` bounds it carries
(`ring_event/tests/event_test.rs:24-33`) are unchanged, and the three
printed booleans are plain `bool` values, uninvolved with `BytesSlot`'s
since-rewritten `Debug`/`PartialEq`. The recorded output should still
reproduce; it just cannot be re-run to confirm.*

```rust
// -ev_probe/src/bin/generic_body_diverges.rs
// The suite's own helper, verbatim: one body, no shape named.
fn round_trip< S, P >( slot : &mut S, payload : P ) -> Option< S::Out< '_ > >
where
  S : Peek,
  P : Fill< S >,
{
  publish_into( slot, payload ).expect( "the payload fits" );
  drain_from( slot )
}

round_trip( &mut typed, 0u32 ).is_some()
round_trip( &mut bytes, &b"ab"[ .. ] ).is_some()
round_trip( &mut empty, &b""[ .. ] ).is_some()
```

```
    typed, one payload    true
    bytes, two payload    true
    bytes, zero payload   false
```

---

### EV39 — The Parity Claim Is Discharged by the Type Checker, Not by Assertions

This crate's own module documentation asks that both slot shapes "round-trip through the **identical**
claim/publish/drain path", and the ordinary way to test that is to write the
round-trip twice, once per shape, and check the results match. That proves the
two paths *agree*; it does not prove they are one path, because the two bodies
are two bodies.

The suite does something better. `round_trip< S, P >` is written once, names no
shape, and carries a doc comment saying so — "The whole point: one body, no shape
named, used by both shapes below." `one_generic_body_round_trips_a_typed_slot`
instantiates it at `TypedSlot`, `the_same_generic_body_round_trips_a_bytes_slot`
at `BytesSlot`, and the test names carry the argument. `land_and_read< S, P >`
repeats the trick one level up, through `ring_store::Buffer< S >`, which is the
storage the ring itself uses.

So the "identical" in that wording is discharged structurally: if the
body compiles at both instantiations, there is exactly one source path, and no
amount of later editing can give one shape a path of its own without the shared
signature changing.

**Finding.** This is the strongest form the claim can take from inside the crate,
and it costs three tests where a conventional suite would need six. It is also
the reason [`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md)'s
finding is easy to miss from here: the proof is genuine and complete about the
source, so reading this suite leaves a strong impression that the claim is
satisfied, while the unrealised half lives one crate away in `ring_core`'s
monomorphisation. What the pattern proves and what the feature asks are the same
sentence about different scopes, and nothing in the suite marks the boundary.

---

### EV40 — One Source Body Is Not One Behaviour, and the Suite Knows It

Monomorphisation shares source, not conduct. The compiler emits one instantiation
per `(S, P)` pair, each calling that pair's own `Fill` and `Peek` impls, and those
impls are where the shapes genuinely differ.

The probe runs the suite's own helper at three instantiations. The typed one
returns `Some` for its payload. The byte one returns `Some` for a two-byte
payload and `None` for a zero-byte payload — the same source body, the same
generic call, opposite answers, and no typed payload produces the `None`
because `TypedSlot::set` always leaves the slot occupied.

The suite is not naive about this. Only three of its seventeen tests go through
a generic helper; the other fourteen name a concrete shape, eleven of them by
writing `TypedSlot::<` or `BytesSlot::<` directly, and those are exactly the
tests that cover the divergences —
`a_zero_length_publish_is_indistinguishable_from_unpublished_and_says_so`,
`a_byte_payload_longer_than_the_slot_is_refused_and_changes_nothing`,
`a_typed_publish_cannot_fail`.

**Finding.** The division of labour is right and it is nowhere stated. Three
generic tests prove the path is shared; thirteen concrete tests cover the places
sharing a path does not make two shapes the same. A reader who takes the generic
helper's doc comment at face value — "the whole point" — will read the concrete
tests as ordinary coverage rather than as the other half of the argument. One
sentence above the concrete section, saying that these tests exist precisely
because a shared path does not imply shared behaviour, would make the suite
self-describing; the section headers already divide the file, they just do not
say why.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/001`](001_a_trait_on_the_varying_side_and_one_function_over_it.md) | The shape this exploits |
| [`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md) | Where the claim this proves stops being true |
| [`pitfall/001`](../pitfall/001_a_zero_length_payload_reads_as_nothing.md) | The divergence the probe triggers |
| [`invariant/001`](../invariant/001_two_readings_of_one_emptiness.md) | Another property held per shape rather than per path |

### Sources

| Fact | Where |
|------|-------|
| The generic helper and its doc comment | `ring_event/tests/event_test.rs:24-33` |
| The storage-level helper, with four bounds | `ring_event/tests/event_test.rs:35-43` |
| Three tests through a helper, seventeen in total | Census above |
| Eleven tests naming a shape explicitly, over thirteen lines | Census above |
| This crate's "identical" wording | `ring_event/src/lib.rs:7-9` |
| One body returning opposite answers | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `one_generic_body_round_trips_a_typed_slot` | The first instantiation of the shared body |
| `the_same_generic_body_round_trips_a_bytes_slot` | The second |
| `both_shapes_land_in_storage_through_the_same_two_calls` | The same trick through real storage |
| `a_zero_length_publish_is_indistinguishable_from_unpublished_and_says_so` | The divergence the shared body cannot cover |
