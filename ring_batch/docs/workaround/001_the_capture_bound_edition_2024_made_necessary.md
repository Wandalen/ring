# Workaround: The Capture Bound Edition 2024 Made Necessary

### Scope

**Purpose:** Record why two of the crate's signatures carry `+ use< >`, prove
both are load-bearing, show that the identical source needs neither under the
previous edition, and identify the design choice that would remove both.

**Responsibility:** `sequences`' and `drain_order`'s capture bounds, the edition
rule that requires them, and the by-value alternative the crate's twin already
uses.

**In Scope:** `ring_batch/src/lib.rs:162`, `:358-359`;
`ring_mpsc/src/lib.rs:1195`; `ring_claim/src/lib.rs:205`.

**Out of Scope:** What `impl Iterator` erases is
[`type/002`](../type/002_the_iterator_nobody_can_name.md). The `usize`/`u64`
casts are [`workaround/002`](002_the_usize_u64_seam.md).

---

## Three Capture Bounds in the Family, Two of Them Here

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every capture bound in the family --'
command grep -r 'use<' --include=*.rs . | sed 's|ring/||'
echo '  -- against the twin, which takes self by value and needs none --'
command grep -m1 -F '  pub fn sequences( self ) -> impl Iterator< Item = Seq >' ring_claim/src/lib.rs
echo '  -- and the edition all 33 crates are on --'
command grep '^edition' Cargo.toml   # unanchored: the members list above it grows
```

Live output:

```
  -- every capture bound in the family --
ring_mpsc/src/lib.rs:  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< S >
ring_batch/src/lib.rs:  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >
ring_batch/src/lib.rs:-> impl Iterator< Item = ( Seq, SlotIndex ) > + use< >
  -- against the twin, which takes self by value and needs none --
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  -- and the edition all 33 crates are on --
edition = "2024"
```

Rust 2024 changed what a return-position `impl Trait` captures: every in-scope
lifetime, by default, whether the body uses it or not. `use< >` is the opt-out —
it names the generics the opaque type may capture, and an empty list means none.

---

### BA50 — Both Bounds Are Load-Bearing, and Neither Is Needed by the Library Itself

Three builds of the same source settle it. The library with both bounds stripped
compiles clean; the failure appears only at a caller that keeps the iterator past
the claim:

```
=== library with both bounds stripped, edition 2024 ===
  cargo check — clean

=== a caller that returns the iterator, edition 2024 ===
error[E0597]: `claim` does not live long enough
 --> tests/caller.rs:7:3
  |
6 |   let claim = BatchClaim::new( Seq( start ), n );
  |       ----- binding `claim` declared here
7 |   claim.sequences()
  |   ^^^^^------------
  |   |
  |   borrowed value does not live long enough
  |   argument requires that `claim` is borrowed for `'static`
8 | }
  | - `claim` dropped here while still borrowed

=== the identical source and caller, edition 2021 ===
test result: ok. 1 passed; 0 failed
```

**Finding.** The bound is exactly and only an edition-2024 compensation — the
same source and the same caller pass under 2021 and fail under 2024, with nothing
else changed. Both sites are independently load-bearing: stripping only
`drain_order`'s while keeping `sequences`' produces the same E0597 at the
`drain_order` call, so neither was copied from the other.

What the crate does not have is a caller that needs either. `ring_tls` — the sole
dependant — never calls `sequences` or `drain_order`
([`type/002`](../type/002_the_iterator_nobody_can_name.md) BA49), and the crate's
own test suite consumes every iterator in the expression that makes it. So the
bounds are correct, necessary for a shape that is entirely reasonable, and
currently protecting nobody.

They are also unexplained. `use< >` is two years old, appears three times in
thirty-three crates, and carries no comment at any of the three sites. A reader
who has not met the 2024 capture rules will read it as noise and a reader who
deletes it will get a clean `cargo check` and a broken downstream crate later.

---

### BA51 — A `Copy` Type Passed by Reference Is What Created the Problem

`BatchClaim` is sixteen bytes and derives `Copy`. Both signatures take it by
reference anyway — `sequences( &self )` and `drain_order( claim : &BatchClaim )` —
and a reference is a lifetime, which is what edition 2024 captures.

Making both take the value instead removes both bounds:

```
=== sequences( self ), drain_order( claim : BatchClaim ), no use<> anywhere ===
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**Finding.** With the receiver and the parameter taken by value, the opaque types
capture no lifetime because there is none in scope, the capture bounds become
unnecessary, and the caller that returns the iterator compiles. Passing a
sixteen-byte `Copy` struct by value is also cheaper than passing a pointer to it.

`ring_claim::Claim` — the same two fields, the same eight methods, three tiers up
([`pattern/001`](../pattern/001_the_range_object.md) BA38) — already writes
`pub fn sequences( self )` and consequently carries no capture bound. Two crates
built the same range type; one took `&self` and needed an edition workaround, the
other took `self` and did not.

This is not a defect in either. `&self` is the ordinary reflex for an accessor and
is right for almost every type; it is wrong here only because the type is `Copy`,
tiny, and returns an iterator. It is worth recording because the workaround and
the choice that made it necessary sit four lines apart and neither mentions the
other.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/002`](../type/002_the_iterator_nobody_can_name.md) | The other half of these two signatures |
| [`pattern/001`](../pattern/001_the_range_object.md) | The twin that took `self` by value |
| [`data_structure/001`](../data_structure/001_sixteen_bytes_that_are_not_a_buffer.md) | The sixteen bytes that make by-value viable |
| [`workaround/002`](002_the_usize_u64_seam.md) | The crate's other absorbed external constraint |

### Sources

| Fact | Where |
|------|-------|
| The two bounds | `ring_batch/src/lib.rs:162`, `:359` |
| The family's third | `ring_mpsc/src/lib.rs:1195` |
| The twin without one | `ring_claim/src/lib.rs:205` |
| Load-bearing at both sites, and edition-specific | Patched-copy builds, quoted above |
| By-value removes both | Patched-copy build, quoted above |

### Tests

| Test | Covers |
|------|--------|
| *(to create)* | Nothing in the suite returns an iterator past its claim — the exact shape both bounds exist for |
| *(to create)* | A compile-fail test would pin the bound; the crate has no `trybuild` harness |
