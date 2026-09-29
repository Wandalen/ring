# Non-Functional Requirement: The Indirection That Is Not There

### Scope

**Purpose:** Measure what routing a publish through this crate costs against
calling the slot directly, and establish whether anything in the family would
notice if that changed.

**Responsibility:** The paired timing of both slot shapes, the assembly the two
paths compile to, the one performance claim the crate makes, and the family
benchmark's coverage of this path.

**In Scope:** `ring_event/src/lib.rs:25-31`, `:149-152`;
`ring_bench/Cargo.toml:16-41`; `ring_slot/src/lib.rs:176`, `:385`;
probes below.

**Out of Scope:** The bound count a caller pays instead is
[`api/001`](../api/001_two_traits_three_functions_one_associated_type.md). What
the extension point can put back into the path is
[`item/002`](../item/002_four_impls_and_what_the_blanket_one_does_not_claim.md).

---

## What the Crate Says About Cost, and Who Would Measure It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the one performance claim the crate makes --'
command grep -m1 -A6 -F '//! ## Why the read half is a GAT' ring_event/src/lib.rs
echo '  -- and how it describes the function that carries the cost --'
command grep -m1 -A2 -F '/// Deliberately trivial. Its value is not what it does but that there is only' ring_event/src/lib.rs
echo '  -- what the family benchmark declares --'
command grep '^ring_' ring_bench/Cargo.toml
echo '  -- and the reason it gives for the slot crate --'
command grep -m1 -A3 -F '#   ring_slot  — `ring_spsc::Ring< S >` and `ring_mpsc::Ring< S >` are generic' ring_bench/Cargo.toml
echo '  -- against every Slot implementor there is --'
command grep -r 'impl.*Slot for' --include=*.rs .
```

Live output:

```
  -- the one performance claim the crate makes --
//! ## Why the read half is a GAT
//!
//! The two shapes genuinely return different things: a `TypedSlot<T>` hands
//! back a `&T`, a `BytesSlot<N>` a `&[u8]` whose length is the payload's, not
//! the slot's. Flattening both into one concrete return type would mean
//! copying, and a `&[u8]` copied out of a slot is the allocation the whole
//! family exists to avoid. An associated type with a lifetime lets one function
  -- and how it describes the function that carries the cost --
/// Deliberately trivial. Its value is not what it does but that there is only
/// one of it: a ring's publish *path* passes through here — the step where a
/// claimed slot receives its payload — so no slot shape can acquire a publish
  -- what the family benchmark declares --
ring_factory = { path = "../ring_factory" }
ring_tls = { path = "../ring_tls" }
ring_flush = { path = "../ring_flush" }
ring_stats = { path = "../ring_stats" }
ring_spsc = { path = "../ring_spsc" }
ring_mpsc = { path = "../ring_mpsc" }
ring_core = { path = "../ring_core" }
ring_slot = { path = "../ring_slot" }
ring_types = { path = "../ring_types" }
  -- and the reason it gives for the slot crate --
#   ring_slot  — `ring_spsc::Ring< S >` and `ring_mpsc::Ring< S >` are generic
#                over `Slot`, and `TypedSlot< T >` is the only implementor this
#                benchmark exercises (the other, `BytesSlot< N >`, round-trips
#                through no path measured here — see ring_event's own
  -- against every Slot implementor there is --
ring_slot/src/lib.rs:impl< T > Slot for TypedSlot< T >
ring_slot/src/lib.rs:impl< const N : usize > Slot for BytesSlot< N >
```

## Two Million Publishes Each Way, Nine Times Over

Both variants run back-to-back inside every repetition, so drift affects both
equally. `black_box` guards the payload and the slot; the four measured
functions carry `#[ inline( never ) ]` so the loops survive optimisation.

```rust
// -ev_probe/src/bin/indirection_cost.rs
#[ inline( never ) ]
fn typed_via_crate( slot : &mut TypedSlot< u64 >, n : usize )
{
  for i in 0..n
  {
    publish_into( slot, black_box( i as u64 ) ).unwrap();
    black_box( &*slot );
  }
}

#[ inline( never ) ]
fn typed_direct( slot : &mut TypedSlot< u64 >, n : usize )
{
  for i in 0..n
  {
    slot.set( black_box( i as u64 ) );
    black_box( &*slot );
  }
}
```

Two independent release runs, nine paired repetitions each, `ROUNDS` at two
million:

```
    typed: crate / direct        median 1.000x   min 0.959x   max 1.163x
    bytes: crate / direct        median 1.012x   min 1.003x   max 1.015x
```

```
    typed: crate / direct        median 1.002x   min 0.994x   max 1.007x
    bytes: crate / direct        median 1.000x   min 0.995x   max 1.007x
```

## What the Compiler Emits for Each Path

```rust
// -ev_probe/src/bin/emitted_code.rs
#[ unsafe( no_mangle ) ]
pub extern "C" fn through_the_crate( slot : &mut TypedSlot< u64 >, value : u64 ) -> bool
{
  publish_into( slot, value ).is_ok()
}

#[ unsafe( no_mangle ) ]
pub extern "C" fn straight_to_the_slot( slot : &mut TypedSlot< u64 >, value : u64 ) -> bool
{
  slot.set( value );
  Ok::< (), RingError >( () ).is_ok()
}
```

Built with `cargo rustc --release -- --emit=asm`, the emitted file contains one
body and two names for it:

```
straight_to_the_slot:
	.cfi_startproc
	mov	w9, #1
	mov	x8, x0
	str	x9, [x0]
	mov	w0, #1
	str	x1, [x8, #8]
	ret
```

```
	.globl	through_the_crate
	.type	through_the_crate,@function
through_the_crate = straight_to_the_slot
```

---

### EV33 — The Overhead Is Not Small, It Is Absent, and the Crate Never Says So

Two independent nine-repetition runs put the typed ratio at 1.000× and 1.002×
and the byte ratio at 1.012× and 1.000×, with every spread inside a few percent
of unity — no measurable difference between publishing through this crate and
calling the slot's own method.

The assembly explains why there is nothing to measure. Two `extern "C"` functions
that differ only in whether they route through `publish_into` compile to a single
body, and the second symbol is emitted as an alias for the first:
`through_the_crate = straight_to_the_slot`. Not equivalent code — the same code,
deduplicated because the bodies came out identical.

The crate never mentions this. Its one performance statement is about the read
half — flattening `Peek::Out` into a concrete type "would mean copying, and a
`&[u8]` copied out of a slot is the allocation the whole family exists to avoid"
— which defends a design choice against a cost it would have introduced. The
write half's own account is architectural: "Deliberately trivial. Its value is
not what it does but that there is only one of it."

**Finding.** In a family whose stated purpose is avoiding allocation and copying
on a hot path, the first question a reader asks of a crate that sits between the
ring and the slot is what it costs to go through it. The answer is a linker
alias, which is about as strong as the answer can be, and it is recorded nowhere
— not in the module doc, not on `publish_into`, not in a test, not in a
benchmark. One sentence stating that the three functions are forwarding shims the
optimiser erases, and that this was measured rather than assumed, would answer
the question the "deliberately trivial" paragraph raises and leaves open.

---

### EV34 — The Family Benchmark Excludes This Crate, and Its Stated Reason for Including the Slot Crate Is Wrong

`ring_bench` is where the family measures itself. It declares nine ring crates —
`ring_factory`, `ring_tls`, `ring_flush`, `ring_stats`, `ring_spsc`, `ring_mpsc`,
`ring_core`, `ring_slot`, `ring_types` — and documents, in comments, why each of
the last three had to be added at all. `ring_event` is not among them, so no
benchmark in the workspace exercises the one path this crate exists to provide,
and the alias in the section above would stop happening without anything noticing.

The comment explaining `ring_slot`'s inclusion is where this gets worse. It reads:
"`ring_spsc::Ring< S >` and `ring_mpsc::Ring< S >` are generic over `Slot`, and
`TypedSlot< T >` is the only implementor."

`TypedSlot` is not the only implementor. `ring_slot` declares two, `TypedSlot< T >`
at `:176` and `BytesSlot< N >` at `:385`, and the second one is the entire reason
this crate exists — `ring_event`'s own module documentation states, in its
opening lines, that both must round-trip through one identical path.

**Finding.** The benchmark's model of the family has the second slot shape
missing from it, stated as fact in a manifest comment. That is consistent with
[`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md)'s
finding that `ring_core` fixes `TypedSlot< T >` eight times and never mentions
`BytesSlot` — from inside `ring_bench`, where every ring reachable through
`ring_core` is a typed-slot ring, `TypedSlot` genuinely does look like the only
implementor. It is still false about `Slot`, and it is the sentence that would
have to change first before anyone benchmarked byte-slot parity. Correcting it
costs a clause; noticing it required reading a crate the benchmark does not
depend on.

```sh
cd "$(git rev-parse --show-toplevel)"
# the comment wraps, so `this benchmark exercises` spans two lines and no
# single-line pattern can anchor on it; take the whole ring_slot paragraph
# by its opening and closing markers instead
awk '/^#   ring_slot  —/{ f = 1 } f && /^#   ring_types/{ exit } f' ring_bench/Cargo.toml
```

Live output:

```
#   ring_slot  — `ring_spsc::Ring< S >` and `ring_mpsc::Ring< S >` are generic
#                over `Slot`, and `TypedSlot< T >` is the only implementor this
#                benchmark exercises (the other, `BytesSlot< N >`, round-trips
#                through no path measured here — see ring_event's own
#                non_functional_requirement/001 § EV34). The two direct
#                candidates cannot name their own ring type without it.
```

**Disposition:** applied — `ring_bench/Cargo.toml`'s comment explaining the
`ring_slot` dependency no longer states "`TypedSlot< T >` is the only
implementor" as an unqualified fact; it now scopes that claim to what this
benchmark exercises and names `BytesSlot< N >` as the second implementor the
benchmark does not measure, cross-referencing this finding. Now prints:
`benchmark exercises`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/002`](002_a_core_only_crate_that_does_not_say_so.md) | The other property nothing in the family checks |
| [`integration/002`](../integration/002_the_ring_writes_slots_without_this_crate.md) | Why the second shape is missing from every model of the family |
| [`item/002`](../item/002_four_impls_and_what_the_blanket_one_does_not_claim.md) | What a downstream `Fill` may put back into this path |
| [`algorithm/001`](../algorithm/001_two_executable_statements_and_one_branch.md) | The two statements that compile to nothing |

### Sources

| Fact | Where |
|------|-------|
| The crate's one performance claim, about the read half | `ring_event/src/lib.rs:27-31` |
| "Deliberately trivial" | `ring_event/src/lib.rs:149-152` |
| Median 1.000×–1.012× across two independent nine-rep runs | Probe above |
| One body and two symbols in the emitted assembly | Probe above |
| The nine ring crates `ring_bench` declares | `ring_bench/Cargo.toml:16-41` |
| "`TypedSlot< T >` is the only implementor" | `ring_bench/Cargo.toml:33` |
| Two `Slot` implementors | `ring_slot/src/lib.rs:176`, `:385` |

### Tests

| Test | Covers |
|------|--------|
| `both_shapes_land_in_storage_through_the_same_two_calls` | The path whose cost nothing measures |
| `a_typed_payload_survives_storage_byte_identically` | The typed half of the timing above |
| `a_byte_payload_of_every_length_up_to_slot_size_survives_storage` | The byte half |
