# Type: The Iterator Nobody Can Name

### Scope

**Purpose:** Record what `impl Iterator` hides on the crate's two sequence-yielding
functions, which capability is actually lost, and how consistently the family
made the same choice.

**Responsibility:** `sequences`' and `drain_order`'s return types, the traits the
underlying concrete type implements, and the four `sequences` methods across the
family.

**In Scope:** `ring_batch/src/lib.rs:162-165`, `:358-362`.

**Out of Scope:** The `+ use< >` capture bound is
[`workaround/001`](../workaround/001_the_capture_bound_edition_2024_made_necessary.md).
Laziness as a cost property is
[`non_functional_requirement/002`](../non_functional_requirement/002_sixteen_bytes_and_no_allocation.md).

---

## What the Signature Says and What the Body Builds

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the caller is promised --'
command grep -m1 -A3 -F '  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >' ring_batch/src/lib.rs
command grep -m1 -A4 -F 'pub fn drain_order( claim : &BatchClaim, capacity : Capacity )' ring_batch/src/lib.rs
echo '  -- every impl Iterator return in the family --'
command grep -r -e '-> impl Iterator' --include=lib.rs ring_*/src/ | sed 's|ring/||'
```

Live output:

```
  -- what the caller is promised --
  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >
  {
    ( self.start.0..self.end().0 ).map( Seq )
  }
pub fn drain_order( claim : &BatchClaim, capacity : Capacity )
-> impl Iterator< Item = ( Seq, SlotIndex ) > + use< >
{
  claim.sequences().map( move | seq | ( seq, of( seq, capacity ) ) )
}
  -- every impl Iterator return in the family --
ring_batch/src/lib.rs:  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >
ring_batch/src/lib.rs:-> impl Iterator< Item = ( Seq, SlotIndex ) > + use< >
ring_claim/src/lib.rs:  pub fn sequences( self ) -> impl Iterator< Item = Seq >
ring_consume/src/lib.rs:  pub fn sequences( self ) -> impl Iterator< Item = Seq >
ring_mpsc/src/lib.rs:  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< S >
ring_mpsc/src/lib.rs:  pub fn iter( &self ) -> impl Iterator< Item = &S >
ring_registry/src/lib.rs:  pub fn names( &self ) -> impl Iterator< Item = &str >
ring_spsc/src/lib.rs:  pub fn iter( &self ) -> impl Iterator< Item = &S >
ring_tls/src/lib.rs:  pub fn drain( &mut self ) -> impl Iterator< Item = T > + '_
```

The body builds `Map< Range< u64 >, fn( u64 ) -> Seq >`. The signature promises
`Iterator`, and nothing else.

---

### BA48 — One Capability Is Lost, and It Is Not the Obvious One

Measured — the same fold written inline against the same fold behind the
signature:

```
--- the concrete type the method builds, written inline ---
  size_hint()  (8, Some(8))
  rev()        [Seq(7), Seq(6), Seq(5)]
--- the same fold, behind impl Iterator ---
  size_hint()  (8, Some(8))
  rev()        does not compile
```

and the error:

```
error[E0277]: the trait bound `impl Iterator<Item = Seq>: DoubleEndedIterator` is not satisfied
 --> src/bin/opacity_fail.rs:7:46
  |
7 |   let _back : Vec< Seq > = claim.sequences().rev().collect();
  |                                              ^^^ the trait `DoubleEndedIterator` is not implemented for `impl Iterator<Item = Seq>`
```

**Finding.** `DoubleEndedIterator` is the one real casualty. `Range< u64 >` is
double-ended, `Map` forwards it, and the `impl Iterator` bound erases it — so a
caller cannot walk a claim backwards, which is exactly what a consumer draining
newest-first would want.

The length is not lost, which is the part worth checking rather than assuming.
`size_hint` is a method on `Iterator` itself, so it survives the erasure and
reports `(8, Some( 8 ))` through the opaque type. `ExactSizeIterator::len` is
unavailable — but it is unavailable on the concrete type too, because `Range<
u64 >` does not implement `ExactSizeIterator` at all. Widening the return type
would not recover `len()`; only `Iterator::count` or the claim's own `len()`
gives it, and the claim's own `len()` is free.

So the cost of the opaque return is one trait, not a general loss of capability,
and the accompanying benefit is real: the concrete type is
`Map< Range< u64 >, fn( u64 ) -> Seq >`, which is not a thing any caller should
be typing, and pinning it would freeze the implementation of a `const`-arithmetic
crate to one specific fold.

---

### BA49 — The One Dependant Re-Derives the Numbering Rather Than Store the Iterator

An unnameable return type costs nothing to a caller that consumes it on the spot,
and quite a lot to one that wants to keep it. The family has exactly one of the
latter:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the only dependant, and what it holds --'
command grep -m1 -A5 -F 'pub struct Flush< '"'"'a, T >' ring_tls/src/lib.rs
echo '  -- and how it walks the range the claim already describes --'
command grep -m1 -A18 -F 'impl< T > Iterator for Flush< '"'"'_, T >' ring_tls/src/lib.rs
echo '  -- the one stronger iterator trait in all 33 crates --'
command grep -r -e 'DoubleEndedIterator' -e 'ExactSizeIterator' --include=lib.rs ring_*/src/ | sed 's|ring/||'
```

Live output:

```
  -- the only dependant, and what it holds --
pub struct Flush< 'a, T >
{
  claim : BatchClaim,
  next : u64,
  items : std::vec::Drain< 'a, T >,
}
  -- and how it walks the range the claim already describes --
impl< T > Iterator for Flush< '_, T >
{
  type Item = ( Seq, T );

  fn next( &mut self ) -> Option< Self::Item >
  {
    if self.next >= self.claim.end().0
    {
      return None;
    }
    let item = self.items.next()?;
    let seq = Seq( self.next );
    self.next += 1;
    Some( ( seq, item ) )
  }

  fn size_hint( &self ) -> ( usize, Option< usize > )
  {
    self.items.size_hint()
  -- the one stronger iterator trait in all 33 crates --
ring_tls/src/lib.rs:impl< T > ExactSizeIterator for Flush< '_, T > {}
```

**Finding.** `Flush` holds a `BatchClaim` and, beside it, a `next : u64` that
counts through the very range that claim describes. It then hand-writes
`Iterator` — incrementing `next` per item rather than zipping `claim.sequences()`
with `items` — and adds the family's only `ExactSizeIterator` impl, which appears
in one place across all thirty-three crates.

The reason is the return type. `claim.sequences()` cannot be a struct field: its
type is `Map< Range< u64 >, fn( u64 ) -> Seq >` written nowhere and nameable by
no one, so storing it means naming that type by hand, boxing it into an
allocation the crate is built to avoid
([`non_functional_requirement/002`](../non_functional_requirement/002_sixteen_bytes_and_no_allocation.md)),
or not storing it. `ring_tls` chose not to store it, and paid eight redundant
bytes plus an invariant the type does not enforce: nothing checks that `next`
stays inside `claim`, and the two would silently disagree if either were ever set
wrong.

That is where the erasure's cost actually lands — not on `rev()`, which nobody
has wanted, but on the one caller that needed the iterator to outlive the
expression that made it. Both crates are individually right: an opaque return is
correct for a `const`-arithmetic crate that should not freeze its fold, and a
hand-written `Iterator` is correct for a type that has to pair sequences with
items anyway. Neither says that the second exists because of the first.

The `+ use<>` bounds are where the four `sequences` methods visibly diverge:
`ring_batch` writes `+ use< >` on both of its functions, `ring_mpsc` writes
`+ use< S >`, `ring_claim` and `ring_consume` write neither — driven by what each
borrows, not by any disagreement about the return type itself.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](../workaround/001_the_capture_bound_edition_2024_made_necessary.md) | The `+ use< >` half of the same signature |
| [`type/001`](001_the_ring_that_can_gate_against_itself.md) | The crate's other type-level decision |
| [`non_functional_requirement/002`](../non_functional_requirement/002_sixteen_bytes_and_no_allocation.md) | The allocation that boxing the iterator would have cost |
| [`item/002`](../item/002_one_past_the_end.md) | `end()`, which both of these iterators are built from |

### Sources

| Fact | Where |
|------|-------|
| The two signatures and their bodies | `ring_batch/src/lib.rs:162-165`, `:358-362` |
| Every `impl Iterator` return in the family | Census above |
| `rev()` on the concrete type and on the opaque one | Probe and compiler output, quoted above |
| `Flush`'s fields and its hand-written `Iterator` | `ring_tls/src/lib.rs:294-299`, `:314-334` |

### Tests

| Test | Covers |
|------|--------|
| `the_sequences_returned_are_contiguous` | The forward walk, which is all the type promises |
| `a_batch_drain_reads_in_issue_order` | The same for `drain_order` |
| *(to create)* | Nothing asserts `size_hint`, which is exact and free and the only length signal the return type carries |
