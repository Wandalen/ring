# API: A Shared Cell That Is Not `Sync`

### Scope

**Purpose:** Record what `SeqCell` requires of an implementor, what it therefore
guarantees to a caller, and the gap between that and the thing every caller needs
from it.

**Responsibility:** The trait's (absent) supertrait list, the three generic bounds
the family writes against it, and the one test that exercises it as a trait
object.

**In Scope:** `ring_atomic/src/lib.rs:108`;
`ring_atomic/tests/atomic_test.rs:289-296`;
`ring_batch/src/lib.rs:221`, `:306`; `ring_tls/src/lib.rs:280`.

**Out of Scope:** The return values the trait hands back, and the missing
`must_use`, are [`api/001`](001_the_return_value_that_is_a_claim.md).

---

## What the Trait Asks For, and Where It Is Asked For

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the declaration, with nothing to its right --'
command grep -m1 -F 'pub trait SeqCell' ring_atomic/src/lib.rs
echo '  -- every generic bound the family writes on it --'
command grep -rE '< *[A-Z] *: *SeqCell|^ *[A-Z] *: *SeqCell,' --include=lib.rs */src/ | sed 's|ring/||'
echo '  -- and the one test that certifies the trait object --'
command grep -m1 -A11 -F '  assert_eq!( cell.counts().fetch_adds, THREADS * EACH );' ring_atomic/tests/atomic_test.rs | tail -n 8
```

Live output:

```
  -- the declaration, with nothing to its right --
pub trait SeqCell : Sync
  -- every generic bound the family writes on it --
ring_batch/src/lib.rs:pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim
ring_batch/src/lib.rs:pub fn claim_gated< P : SeqCell, C : SeqCell >
ring_tls/src/lib.rs:    C : SeqCell,
  -- and the one test that certifies the trait object --
#[ test ]
fn a_cell_drives_through_the_trait_alone()
{
  // Both implementations must be usable behind `dyn SeqCell`, which is what
  // lets one piece of production code be run against either.
  fn advance( cell : &dyn SeqCell ) -> Seq
  {
    cell.fetch_add( 2, Ordering::AcqRel )
```

Three bare bounds, one single-threaded object-safety test — and, since AT7 was
written, a supertrait where the declaration used to end.

---

### AT7 — `&dyn SeqCell` Cannot Cross a Thread Boundary

The trait exists so that a cursor shared between a producer and a consumer can be
read and advanced through one interface. It declared no supertrait — not `Send`,
not `Sync`, not `Debug` — so a trait object of it was `!Sync`, and by extension
`&dyn SeqCell` was `!Send`:

```
error[E0277]: `dyn SeqCell` cannot be shared between threads safely
  --> src/bin/dynshare.rs:10:14
   |
10 |     s.spawn( || { cell.fetch_add( 1, Ordering::AcqRel ) } );
   |       -----  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ `dyn SeqCell` cannot be shared between threads safely
   |       |
   |       required by a bound introduced by this call
   |
   = help: the trait `Sync` is not implemented for `dyn SeqCell`
   = note: required for `&dyn SeqCell` to implement `Send`
```

The cell inside is an `AtomicU64` and is perfectly `Sync`; the abstraction was what
lost it. Any caller wanting the dynamic form across threads had to write
`&( dyn SeqCell + Sync )` at every site, and nothing in the crate said so.

**Finding.** The trait was object-safe and its object unusable for the crate's
only stated purpose. The suite certified exactly the half that worked:
`a_cell_drives_through_the_trait_alone` builds a `&dyn SeqCell`, calls it on the
test thread, and stops — so the property the comment claims ("one piece of
production code run against either") was demonstrated only in the configuration the
family never uses.

`Sync` as a supertrait cost nothing. Both implementors already satisfied it, no
sensible implementor could fail to, and it makes the dynamic form work without a
bound at every call site.

**Disposition:** applied — `SeqCell` now declares `Sync` as a supertrait, and the
declaration is the first thing the recipe above prints. The dynamic form works
across threads without a bound at any call site, and
`both_cells_and_the_object_form_are_sync` asserts it three ways: `AtomicSeq`,
`CountingSeq` and `dyn SeqCell` each pass a `requires_sync` bound, and two scoped
threads then share one cell through `&dyn SeqCell` and advance it 2,000 times.
The test was proven able to fail by deleting the supertrait, which turns the whole
test target red with the same `E0277` quoted above rather than a runtime failure —
so the guard fires at compile time, where AT7 argued it belonged. What this does
not buy: nothing forces a future method onto the trait to be thread-safe in any
sense beyond `Sync` on the implementor, and the three generic `C : SeqCell` bounds
in `ring_batch` and `ring_tls` still read as bare bounds at their own sites — they
inherit the requirement now, but a reader there still cannot see it. Now prints:
`pub trait SeqCell : Sync`

---

### AT8 — The Generic Bound Accepts a Cell That Cannot Be Shared at All

Three sites in the family are generic over `SeqCell`, and all three write the bare
bound. That form compiles for a concrete `AtomicSeq` because `Sync` is inferred
per-instantiation, never because the bound asked for it — so an implementor backed
by `Cell< u64 >` satisfies every signature in the family:

```
  ring_batch::claim< C : SeqCell > accepted it : BatchClaim { start: Seq(0), count: 8 }
  needs_sync::< AtomicSeq >()                  : compiles
```

and the same type fails the moment anything actually asks:

```
error[E0277]: `Cell<u64>` cannot be shared between threads safely
  --> src/bin/notsync.rs:36:17
   |
36 |   needs_sync::< PlainCell >();
   |                 ^^^^^^^^^ `Cell<u64>` cannot be shared between threads safely
```

**Finding.** `ring_batch::claim` will hand a `BatchClaim` for eight sequences to a
caller holding a cell with no atomicity whatsoever, and the type system raises
nothing — the claim is only meaningful because the caller happened to pass a real
atomic. The bound names the operations and says nothing about the one property
that makes the operations mean anything.

This is not reachable today: `AtomicSeq`, `CountingSeq`, and `PaddedCursor` are the
only implementors and all three are `Sync`. It becomes reachable the moment anyone
writes a fourth — a single-threaded test double, most obviously, which is exactly
the kind of implementor a trait like this invites — and the failure would be a
silent loss of atomicity, not a compile error.

The fix is one word on the declaration, and it propagates to all three bound sites
and both existing implementors without touching either.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/001`](001_the_return_value_that_is_a_claim.md) | The other declaration-level attribute the trait does not carry |
| [`type/001`](../type/001_what_the_trait_promises.md) | The same gap read as a question about what the type commits to |
| [`invariant/002`](../invariant/002_every_increment_survives.md) | The atomicity the bound assumes and does not require |
| [`pattern/001`](../pattern/001_one_place_where_an_atomic_is_created.md) | Why one trait, and what having one buys |

### Sources

| Fact | Where |
|------|-------|
| The declaration and its empty supertrait list | `ring_atomic/src/lib.rs:108` |
| The three generic bound sites | `ring_batch/src/lib.rs:221`, `:306`; `ring_tls/src/lib.rs:280` |
| The single-threaded object-safety test | `ring_atomic/tests/atomic_test.rs:289-296` |
| `&dyn SeqCell` is `!Send` | Probe, quoted above |
| A `Cell`-backed implementor satisfies `claim` | Probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_cell_drives_through_the_trait_alone` | That the trait is object-safe — on one thread, which is the case that works |
| *(to create)* | Nothing puts a `&dyn SeqCell` on two threads, which is the case the family needs and the trait does not support |
