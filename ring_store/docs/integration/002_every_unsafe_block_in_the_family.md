# Integration: Every Unsafe Block in the Family Reaches Into a Buffer

### Scope

**Purpose:** Record that the two crates which opt out of the workspace-wide
`unsafe` denial are exactly this crate's two real consumers, that all twelve
`unsafe` blocks in the family exist to reach a slot inside a `Buffer`, and what
that makes this crate's own `unsafe`-freedom worth.

**Responsibility:** The seam between a safe container and the two crates that
reach through it — where the safety obligation actually lands.

**In Scope:** `ring_mpsc/src/lib.rs:191, 594-640`;
`ring_spsc/src/lib.rs:169, 417-467`; the workspace lint table;
the family-wide `unsafe` census.

**Out of Scope:** Whether this crate *should* offer a different API that made the
wrap unnecessary is [`api/002`](../api/002_six_ways_to_reach_a_slot.md).
That `&mut self` buys the consumers nothing is
[`pattern/002`](../pattern/002_every_write_is_a_borrow.md).

---

## The Workspace Position

The root manifest denies `unsafe` for every crate that inherits the lint table,
which is all 33:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'unsafe-code' Cargo.toml   # unanchored: the members list above it grows
```

Live output:

```
unsafe-code = "deny"
```

`ring_store` satisfies it without effort. Its whole storage mechanism is a
`Box< [ S ] >` and a slice index, and the module comment says so:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '//! What is left is small enough to state completely: `capacity` slots allocated' ring_store/src/lib.rs
```

Live output:

```
//! What is left is small enough to state completely: `capacity` slots allocated
//! once, addressed by [`ring_types::SlotIndex`], with the fold from a sequence
//! delegated to `ring_index`. No `unsafe` — a `Box<[S]>` of `Default` slots is
//! allocated in one go, which is what "exactly N slots once" asks for.
```

---

### BF4 — The Two Crates That Opt Out Are This Crate's Two Consumers

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -r 'allow( unsafe_code )' ring_*/src/lib.rs | sed 's|^ring/||' | sort
```

Live output:

```
ring_mpsc/src/lib.rs:#![ allow( unsafe_code ) ]
ring_spsc/src/lib.rs:#![ allow( unsafe_code ) ]
```

Two crates out of 33. They are the same two that take `ring_store` as an
ordinary dependency ([`integration/001`](001_four_dependents_two_that_build_on_it.md)
BF1) — the correspondence is exact, in both directions.

**Finding.** The family's `unsafe` surface and this crate's consumer set are the
same two crates. That is not a coincidence of scheduling: a ring hands a producer
a `&mut S` and a consumer a `&S` for *different* sequences at the same instant,
which is the one thing a `&mut Buffer` cannot express, so any crate that builds a
ring on this storage must reach past the borrow checker to do it.

What follows is that this crate's `unsafe`-freedom is a local property with no
downstream reach. It is real — nothing in `ring_store` can be misused into
undefined behaviour on its own — and it transfers to no consumer, because every
consumer that exists had to opt out of the denial to use it.

---

### BF5 — All Twelve Unsafe Blocks Exist to Reach a Slot Inside a Buffer

```sh
cd "$(git rev-parse --show-toplevel)"
printf '  unsafe blocks family-wide:                %s\n' "$( grep -rho 'unsafe {' ring_*/src/*.rs | wc -l )"
printf '  dereferencing the cell directly:          %s\n' "$( grep -rho 'unsafe { &\(mut \)\?\*self\.slots\.at(' ring_*/src/*.rs | wc -l )"
printf '  calling slot/slot_mut, which do the above: %s\n' "$( grep -rho 'unsafe { self\.ring\.slot' ring_*/src/*.rs | wc -l )"
```

Live output:

```
  unsafe blocks family-wide:                13
  dereferencing the cell directly:          4
  calling slot/slot_mut, which do the above: 8
```

Twelve *real* blocks — the family-wide count above now reads 13, one more than
the split accounts for, because the raw `grep` also matches this document's own
quotation of the pattern in `at`'s doc comment (BF5's own "Disposition: applied"
below, in `ring_store/src/lib.rs`), not a thirteenth block in either
crate. Four dereference the cell; the other eight call the two functions that
contain those four:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn 'unsafe { &\(mut \)\?\*self\.slots\.at(' ring_*/src/*.rs | sed 's|^ring/||' | sort | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
ring_mpsc/src/lib.rs:        unsafe { &*self.slots.at(seq).get() }
ring_mpsc/src/lib.rs:        unsafe { &mut *self.slots.at(seq).get() }
ring_spsc/src/lib.rs:        unsafe { &*self.slots.at(seq).get() }
ring_spsc/src/lib.rs:        unsafe { &mut *self.slots.at(seq).get() }
```

Both consumers spell the same two functions identically, and both give
`slot_mut` a signature the borrow checker will not otherwise permit:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A12 -F '  #[ allow( clippy::mut_from_ref ) ]' ring_mpsc/src/lib.rs
```

Live output:

```
  #[ allow( clippy::mut_from_ref ) ]
  unsafe fn slot_mut( &self, seq : Seq ) -> &mut S
  {
    // SAFETY: the caller is the slot's sole owner under one of the two regimes
    // documented above (an unpublished producer claim, or a published
    // not-yet-committed consumer batch reached through `Batch::get_mut`), so no
    // other `&S`/`&mut S` to this slot is live. `at` yields `&UnsafeCell< S >`,
    // so the write permission this deref needs comes from that one slot's cell
    // and claims nothing about any other slot — which is what lets a second
    // producer write its own claimed slot, or the consumer drain a batch, at
    // the same instant.
    unsafe { &mut *self.slots.at( seq ).get() }
  }
```

**Finding.** There is no `unsafe` anywhere in this family that is not, directly
or one call away, an access to a `Buffer` slot. Twelve blocks, two crates, one
container. The whole of the family's soundness argument is the SAFETY comment on
those four sites plus the claim-exclusivity reasoning they cite.

That concentration is a good outcome and worth naming as one: a reviewer auditing
this family for undefined behaviour has one container and four call sites to
read, not a scatter across 33 crates. The cost is that the argument is stated
entirely on the consumer side. `ring_store` offers `at( &self ) -> &S`,
documents it as an ordinary shared borrow, and has no idea that with
`S = UnsafeCell< T >` both rings turn its return value into `&mut T` from
`&self` behind a `clippy::mut_from_ref` waiver.
Nothing on this crate's side records that its safety story is being reinterpreted
upstream, so a future change here — an added interior field, a `Sync` bound, a
reordering that assumed exclusive access really was exclusive — would be assessed
against a contract two crates no longer honour.

`at`'s own doc comment now says what the consumers do with its return value:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '/// An ordinary shared borrow, as far as this crate is concerned.' ring_store/src/lib.rs
```

Live output:

```
    /// An ordinary shared borrow, as far as this crate is concerned. `ring_mpsc`
    /// and `ring_spsc` read more into it: with `S = UnsafeCell< T >` both call
    /// this under a claim guaranteeing no other caller holds the same `seq`, then
    /// `unsafe { &mut *at( seq ).get() }` the result into a `&mut T` (-> BF5 in
    /// `docs/integration/002_every_unsafe_block_in_the_family.md`). This function
    /// grants nothing beyond the one shared borrow it returns; the exclusivity
```

**Disposition:** applied — added a paragraph to `at`'s doc comment in
`src/lib.rs` recording the cross-crate reinterpretation this finding
describes: with `S = UnsafeCell< T >`, `ring_mpsc` and `ring_spsc` read this
function's ordinary shared borrow as license for an unsafe interior-mutable
deref, under a claim-exclusivity contract this crate cannot see. The note
targets `at`, not `at_mut` — the SAFETY comment quoted above shows the
consumers reach the cell through `self.slots.at( seq ).get()`, the shared
accessor, not the mutable one; `at_mut` returns an ordinary `&mut S` that no
`unsafe` block anywhere touches. The crate's 17 unit tests plus 8 doctests
re-verified passing (`cargo test --all-features`, 2026-09-04). Now prints:
`grants nothing beyond the one shared borrow it returns`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/001`](001_four_dependents_two_that_build_on_it.md) | Who the two consumers are and how much of the surface they use |
| [`api/002`](../api/002_six_ways_to_reach_a_slot.md) | The by-value exit that does not exist, and the pointers its absence protects |
| [`pattern/002`](../pattern/002_every_write_is_a_borrow.md) | The `&`/`&mut` lattice this crate is built on |
| [`workaround/001`](../workaround/001_three_steps_to_avoid_two_bounds.md) | The `unsafe` denial this crate routes around in three lines |

### Sources

| Fact | Where |
|------|-------|
| Workspace-wide denial | `Cargo.toml` — `[workspace.lints.rust]` |
| The crate's own claim | `ring_store/src/lib.rs:19-22` |
| The two opt-outs | `ring_mpsc/src/lib.rs:191`, `ring_spsc/src/lib.rs:169` |
| The four cell dereferences | `ring_mpsc/src/lib.rs:601, 639`, `ring_spsc/src/lib.rs:425, 466` |
| `slot_mut`'s signature and waiver | `ring_mpsc/src/lib.rs:628-629` |

### Tests

| Test | Covers |
|------|--------|
| `two_distinct_slot_indices_never_alias` | The non-aliasing the consumers' SAFETY comments depend on |
| `distinct_indices_have_distinct_addresses` | The same, asserted on storage rather than on values |
| `ring_mpsc` — `handshake_test.rs` | The exclusivity argument, exercised under `loom` |
| *(to create)* | A `ring_store`-side note or test pinning that `at_mut`'s exclusivity is what the consumers rely on |
