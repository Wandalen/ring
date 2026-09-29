# Data Structure: Sixteen Bytes That Are Not a Buffer

### Scope

**Purpose:** Record the only structure this crate defines — two words, `Copy`,
no allocation, no pointer — and the storage dependency its manifest deliberately
does not have.

**Responsibility:** `BatchClaim`'s fields, its measured layout, and the edge
`ring_batch` declines to take.

**In Scope:** `ring_batch/src/lib.rs:55-60`;
`ring_batch/Cargo.toml:8-12`.

**Out of Scope:** The identical structure in `ring_claim` is
[`data_structure/002`](002_the_struct_ring_claim_wrote_again.md). What the
iterators cost is
[`non_functional_requirement/002`](../non_functional_requirement/002_sixteen_bytes_and_no_allocation.md).

---

## Everything the Crate Stores

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- everything the crate stores --'
command grep -m1 -A5 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]' ring_batch/src/lib.rs
echo '  -- what it depends on, and what it does not --'
command grep -E '^ring_' ring_batch/Cargo.toml
echo '  -- every crate above Tier 1 that names a storage crate --'
command grep -rl '^ring_store = \|^ring_slot = ' --include=Cargo.toml . | sed 's|ring/||;s|/Cargo.toml||' | sort | tr '\n' ' '; echo
```

Live output:

```
  -- everything the crate stores --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub struct BatchClaim
{
  start : Seq,
  count : usize,
}
  -- what it depends on, and what it does not --
ring_types = { path = "../ring_types" }
ring_seqno = { path = "../ring_seqno" }
ring_atomic = { path = "../ring_atomic" }
ring_index = { path = "../ring_index" }
  -- every crate above Tier 1 that names a storage crate --
ring_bench ring_store ring_core ring_event ring_mpsc ring_spsc ring_tls 
```

Two fields, four dependencies, and no edge to anything that holds bytes.

---

### BA9 — Two Words, No Niche, and Nothing to Drop

Measured in a release build:

```
--- the two range objects, measured ---
                                   ring_batch::BatchClaim   ring_claim::Claim
  size_of                                  16                  16
  align_of                                  8                   8
  size_of Option< _ >                      24                  24
  is Copy                                 yes                 yes
```

**Finding.** `Option< BatchClaim >` is 24 bytes, not 16 — the struct has no
niche, because both fields use their full range. `Seq( u64 )` has no invalid
bit pattern and neither does `count : usize`; every one of the 2^128 possible
bit patterns is a well-formed `BatchClaim`. That is a consequence of the design
rather than an oversight: an empty claim (`count == 0`) is a legal, meaningful
state that the crate depends on, so there is no spare encoding for `None` to
borrow.

The eight bytes matter more than they look. `ring_tls::Flush` stores a
`BatchClaim` by value at `ring_tls/src/lib.rs:296`, so every in-flight flush carries the
full 16 rather than a pointer, and a `Result< BatchClaim, RingError >` is the
same 24 bytes an `Option` would be. Nothing here is heap-allocated and nothing
has a destructor, which is why `claim` can hand a range across a thread boundary
by copying two words.

---

### BA10 — The One Structural Decision, Stated in the Module Comment and Visible in the Manifest

Seven crates above Tier 1 name a storage crate in their manifest. `ring_batch`
is not one of them, and says why:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '//! A [`BatchClaim`] is a *range*, not a buffer. It says which sequences the' ring_batch/src/lib.rs
```

Live output:

```
//! A [`BatchClaim`] is a *range*, not a buffer. It says which sequences the
//! caller owns; what goes in them is `ring_store`'s and `ring_event`'s
//! business. That split is why this crate needs no storage dependency.
```

**Finding.** This is the crate's only architectural decision and the only one
with a comment attached, and the manifest agrees with it — a rare case in this
family where a stated boundary is checkable from `Cargo.toml`.

What it buys is that a claim is meaningful with no ring in existence.
`BatchClaim::new( Seq( 10 ), 3 )` is a valid value; the test suite constructs
claims directly in ten of its 21 tests without touching a cursor at all, and all
eight accessors are exercised that way. The type is a statement about
sequence arithmetic, and sequence arithmetic does not need memory.

The cost is that nothing can check the claim against the ring it is for. A
`BatchClaim` carries no capacity, no cursor identity, and no ring reference, so
`drain_order( &claim, capacity )` will fold a claim taken from one ring against
another ring's capacity without complaint — the same shape as
`ring_mpsc`'s hand-written fold recorded in `ring_index`'s corpus as IX19, and
here it is the deliberate consequence of the boundary rather than an accident.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](002_the_struct_ring_claim_wrote_again.md) | The same sixteen bytes, defined again one tier up |
| [`non_functional_requirement/002`](../non_functional_requirement/002_sixteen_bytes_and_no_allocation.md) | What the iterators over this structure cost |
| [`type/001`](../type/001_the_ring_that_can_gate_against_itself.md) | The generic parameters that let a claim meet the wrong cursor |
| [`lifecycle/001`](../lifecycle/001_no_lifecycle_and_no_rollback.md) | Why a structure with no `Drop` cannot release anything |

### Sources

| Fact | Where |
|------|-------|
| The struct | `ring_batch/src/lib.rs:55-60` |
| The four dependencies | `ring_batch/Cargo.toml:8-12` |
| The stated boundary | `ring_batch/src/lib.rs:28-30` |
| Sizes and alignment | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `claims_are_copied_not_moved_and_compare_by_value` | `Copy` and `PartialEq`, and the `Debug` field name |
| `a_claim_reports_its_own_extent` | Both fields read back through accessors |
| `an_empty_claim_is_a_success_not_a_failure` | The state that costs the type its niche |
| *(to create)* | That a claim folded against a foreign capacity is not detectable — the boundary's cost, asserted nowhere |
