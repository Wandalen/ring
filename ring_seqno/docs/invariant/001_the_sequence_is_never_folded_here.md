# Invariant: The Sequence Is Never Folded Here

### Scope

- **Purpose**: State the invariant that gives the crate its reason to exist separately from `ring_index`, and show what actually enforces it.
- **Responsibility**: Give the invariant's clauses, the mechanical check for each, and the gap between what the manual plan checks and what the invariant claims.
- **In Scope**: Absence of folding operations in `ring_seqno`.
- **Out of Scope**: What `ring_index` does with folding — that is its crate's documentation.

### The Invariant

> No function in `ring_seqno` reduces a `Seq` modulo a capacity, and no function
> here produces or consumes a `SlotIndex`.

Stated positively: every value this crate handles is an *unfolded* position, so
two positions a full lap apart differ, and a gate can tell "one lap behind" from
"caught up".

The module doc puts it as the crate's whole subject:

> **the sequence does not wrap, the slot index does**. […] Fold the sequence and
> the information is gone; the gate can no longer tell "one lap behind" from
> "caught up".

### The Clauses and What Checks Each

| # | Clause | Check | Enforced by |
|---|--------|-------|-------------|
| C1 | No `%` in any function body | grep | nothing automatic |
| C2 | No `&` mask, no `Capacity::mask` call | grep | nothing automatic |
| C3 | `SlotIndex` is never named | grep, and the import list | nothing automatic |
| C4 | `ring_index` is not a dependency | `Cargo.toml` | **the compiler** |

```sh
cd "$(git rev-parse --show-toplevel)"/ring_seqno
# C1-C3: matches outside comments. Expect exactly one line — `slowest`'s `&[ Seq ]`.
grep -nE '%|&[^&]|mask|SlotIndex' src/lib.rs | grep -vE ':\s*//' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
# C4: the only dependency is ring_types.
sed -n '/\[dependencies\]/,/^\[/p' Cargo.toml
```

Live output:

```
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
[dependencies]
ring_types = { path = "../ring_types" }

[lints]
```

The one line C1–C3 return is `pub fn slowest( cursors : &[ Seq ] )` — the `&` of
a shared reference, not a bitmask. Every other `%` and `&` in the file is inside
a `///` block.

**Only C4 is enforced.** `ring_index` is not in `Cargo.toml`, so no function here
can call the folding it owns. That is a real barrier and it is the one the crate
split was for. C1–C3 are conventions a single line could break — nothing rejects
`seq.0 % capacity.get()` written inline.

### The One Real Barrier Is Worth Stating Precisely

`ring_index` depends on `ring_types`, and so does `ring_seqno`. Neither depends on
the other:

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep 'ring_seqno\|ring_index' ring_index/Cargo.toml ring_seqno/Cargo.toml
```

Live output:

```
ring_index/Cargo.toml:name = "ring_index"
ring_seqno/Cargo.toml:name = "ring_seqno"
```

They are **siblings, not a stack.** That matters: a reader might expect
`ring_index` to be built on `ring_seqno` (fold a sequence into an index), but the
dependency does not exist in either direction. Both take `Seq` and `Capacity`
from `ring_types` and go separate ways with them.

The module doc gives the reason:

> The folding itself is `ring_index`'s job, deliberately in another crate so
> that no function can accidentally do both.

Siblings is the stronger arrangement. Had `ring_index` depended on `ring_seqno`,
nothing would stop a future `ring_index` function from taking an unfolded span
and a folded index in the same signature. As siblings, the only crate that can
name both concepts is one that depends on both, and that crate's author has to
write two `use` lines to do it.

### What M1 Actually Checks

The manual plan's M1 is the check for this invariant, and it is weaker than the
invariant:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r "lap" ring_seqno/src/lib.rs | head -20
```

Live output:

```
//! Sequence numbers and their comparison across laps.
//! gone; the gate can no longer tell "one lap behind" from "caught up".
/// How far apart two sequences are, in laps of a given capacity.
/// full lap ahead of the slowest consumer. `laps_between` is the reading that
/// use ring_seqno::laps_between;
/// assert_eq!( laps_between( Seq( 0 ), Seq( 7 ), cap ), 0 );
/// assert_eq!( laps_between( Seq( 0 ), Seq( 8 ), cap ), 1 );
/// assert_eq!( laps_between( Seq( 0 ), Seq( 17 ), cap ), 2 );
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
/// True exactly while the producer is strictly less than one lap ahead. At
/// exactly one lap the next claim would land on the consumer's current slot,
/// `docs/feature/178_sequence_barrier_and_gating_set.md` calls a lap bug.
/// assert!( !may_claim( Seq( 4 ), Seq( 0 ), cap ) ); // exactly one lap: no room
```

> **Expected:** the module doc explains comparison across laps, and at least one
> function's own doc names the lap boundary. Absence of the word `wrap`
> describing the *arithmetic*.

**That is a check on the prose, not on the code.** Every one of its expectations
is satisfied by comments. If someone added `seq.0 % capacity.get()` to a function
body tomorrow, M1 would still pass — the module doc would still explain laps,
`may_claim`'s doc would still name the boundary, and no new occurrence of `wrap`
would appear.

The 2026-08-28 Run Record reads M1 correctly and is careful about it:

> `wrap` does occur — twice, and both times *denying* it […] Nothing describes
> the arithmetic as wrapping.

So the check was run properly and its verdict is sound. The gap is in what the
check can reach, not in how it was performed. C1–C3 above are the mechanical
form M1 is missing, and they are three greps.

### The Test That Would Catch a Fold

`positions_many_laps_apart_stay_comparable` (`seq_test.rs:134-146`) is the only
assertion that fails if the invariant breaks, and it is the only one that
*could* be — it is the only test using positions more than one capacity apart in
a way where folding changes the answer:

```rust
assert_eq!( producer.0 % 8, consumer.0 % 8 );   // establishes they fold identically
assert_eq!( laps_between( consumer, producer, c ), 99 );
assert!( !may_claim( producer, consumer, c ) );
assert_eq!( free_slots( producer, consumer, c ), 0 );
```

A `ring_seqno` that folded internally would return `0` laps, `true`, and `8` — all
three assertions fail. The test is well-chosen, and it is worth noting it does
the folding itself on line 143 to prove the two inputs are a genuine trap rather
than an arbitrary pair.

`laps_are_relative_not_absolute` (`:38-44`) covers the neighbouring risk: it uses
positions near 1,000,000 so that an implementation measuring from zero rather
than between the pair is caught.

### The Clause Nobody Checks

There is a fourth thing the invariant implies that neither grep nor test covers:
**no function here may return a value that a caller could mistake for a slot
index.** `free_slots` returns `usize` — the same type `SlotIndex` wraps and the
same type `Capacity::get` returns. A caller doing

```rust
let slot = free_slots( producer, consumer, capacity );   // a count, not a position
buffer[ slot ]                                            // compiles, wrong
```

gets no complaint from the compiler. The count and the index are both `usize` in
`0..=capacity`, which overlap almost entirely.

This is not a defect in `ring_seqno` — the alternative is a newtype per quantity,
which is a family-wide decision. It is recorded because the invariant's
statement ("no `SlotIndex` is produced here") is true of the type name and not of
the representation. See [`type/001`](../type/001_what_a_span_is_measured_in.md).

### SQ22 — A Check Its Own Subject Cannot Fail

The manual check passes on the documentation rather than on the code it describes:

```
M1 greps for the folding vocabulary across the crate.
The module doc contains that vocabulary in prose (lines 7-18).
So the grep is satisfied before any function body is examined.
```

**Finding.** M1's grep is satisfied by the module doc alone; it would still pass if the folding invariant were violated in a function body.

---

### SQ23 — The Test That Does the Forbidden Thing on Purpose

The crate exists so that nothing folds a sequence. One test folds one, to prove the point:

```
seq_test.rs:143   assert_eq!( producer.0 % 8, consumer.0 % 8 );
                  // Folded, both are slot 0 — identical, and the gate
                  // would see "caught up".
```

**Finding.** `positions_many_laps_apart_stay_comparable` performs the folding the crate exists to avoid in order to show what would be lost — the only place in the family a test computes the collision deliberately.

---

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_crate_that_declares_no_type.md](../data_structure/001_the_crate_that_declares_no_type.md) | Why `Seq` and `Capacity` live upstream |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_one_dependency_and_five_declared_dependents.md](../integration/001_one_dependency_and_five_declared_dependents.md) | C4, as a manifest fact |

### Invariants

| File | Relationship |
|------|--------------|
| [002_every_reading_is_total.md](002_every_reading_is_total.md) | The same claim from the fallibility side |

### Types

| File | Relationship |
|------|--------------|
| [001_what_a_span_is_measured_in.md](../type/001_what_a_span_is_measured_in.md) | The count/index type overlap |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:7-18` | The invariant, as the crate's stated subject |
| `ring_seqno/src/lib.rs:20-23` | The doc comment that renamed "wrapping arithmetic" to comparison |
| `ring_seqno/Cargo.toml` | C4 — one dependency, and it is not `ring_index` |
| `ring_types/src/id.rs:88-92` | `SlotIndex`, and why it is a separate type |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:134-148` | The only test a fold would break |
| `tests/seq_test.rs:38-44` | Laps measured between the pair, not from zero |
| `tests/manual/readme.md` M1 | The prose check, and what it cannot reach |
