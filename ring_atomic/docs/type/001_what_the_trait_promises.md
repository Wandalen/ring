# Type: What the Trait Promises a Caller

### Scope

**Purpose:** Record what `SeqCell` obliges an implementor to provide and what it
therefore permits a caller to assume — and the two things it declines to require
that every one of its users depends on.

**Responsibility:** The trait's declaration in full: its absent supertrait, its four
method signatures, and the one contract clause in the file.

**In Scope:** `ring_atomic/src/lib.rs:108-151`.

**Out of Scope:** What `AtomicSeq` and `CountingSeq` add beyond the trait is
[`item/001`](../item/001_six_constructors_for_two_types.md); the counting methods
that live off the trait are [`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md).
That monotonicity is relied on and unenforced is
[`invariant/001`](../invariant/001_monotonicity_is_relied_on_and_not_required.md).

---

## The Whole Declaration, and What Bounds It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the trait declares, in full --'
command grep -m1 -A20 -F 'pub trait SeqCell' ring_atomic/src/lib.rs
echo '  -- must_use here, and what it is spent on --'
printf '    #[ must_use ] in ring_atomic/src : %s\n' \
  "$( command grep -c 'must_use' ring_atomic/src/lib.rs || true )"
command grep -A 1 'must_use' ring_atomic/src/lib.rs \
  | command grep 'pub .*fn ' | sed 's/^/      /'
printf '    #[ must_use ] in ring_types/src  : %s\n' \
  "$( command grep -rc 'must_use' ring_types/src/ | cut -d: -f2 | paste -sd+ | bc )"
echo '  -- and every generic bound on SeqCell in the family --'
command grep -r ': SeqCell' --include=*.rs . | command grep -v 'ring_atomic/' | sed 's|ring/||'
```

Live output:

```
  -- what the trait declares, in full --
pub trait SeqCell : Sync
{
  /// Read the current sequence.
  fn load( &self, order : Ordering ) -> Seq;

  /// Overwrite the sequence.
  fn store( &self, value : Seq, order : Ordering );

  /// Advance by `n` and return the sequence as it was *before* the advance —
  /// which is the first sequence the caller now owns.
  ///
  /// # Monotonicity
  ///
  /// **Not guaranteed by this method.** `n` is a `u64` because the cell is a
  /// `u64`, and the addition wraps: `fetch_add( u64::MAX )` moves the cursor
  /// *back* by one and returns a value indistinguishable from a legitimate
  /// claim of a huge range. `AtomicSeq::new` likewise accepts any `Seq`, so a
  /// cursor seeded from persistence or a fixture can start arbitrarily close to
  /// the top. Callers own the monotonicity that `ring_types` and `ring_consume`
  /// each describe as something "every gate in the family relies on".
  ///
  -- must_use here, and what it is spent on --
    #[ must_use ] in ring_atomic/src : 6
        pub const fn new( value : Seq ) -> Self
        pub fn new( value : Seq ) -> Self
        pub const fn new( value : Seq ) -> Self
        pub fn new( value : Seq ) -> Self
        pub fn counts( &self ) -> OpCounts
    #[ must_use ] in ring_types/src  : 11
  -- and every generic bound on SeqCell in the family --
ring_claim/tests/claim_test.rs:/// `cursor()` returns a `&PaddedCursor`, that `PaddedCursor : SeqCell` is a
ring_batch/src/lib.rs:pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim
ring_batch/src/lib.rs:pub fn claim_gated< P : SeqCell, C : SeqCell >
ring_tls/src/lib.rs:    C : SeqCell,
```

---

### AT45 — The Family's Concurrency Primitive Does Not Require Concurrency

`SeqCell` had no supertrait. Every method takes `&self` and mutates, so the trait is
built entirely on interior mutability — which is the shape of a type meant to be
shared across threads, and is also the shape of `Cell< u64 >`, which must never be.
Nothing in the declaration distinguishes them.

A `SeqCell` backed by `Cell< u64 >` implements every method correctly, satisfies the
trait, and is accepted by the family's multi-producer claim function:

```
  ring_batch::claim< C : SeqCell > accepted it : BatchClaim { start: Seq(0), count: 8 }
  needs_sync::< AtomicSeq >()                  : compiles
```

The error does eventually arrive, but only when something else independently demands
`Sync`, and it describes the implementor's private field rather than the trait's
expectation:

```
error[E0277]: `Cell<u64>` cannot be shared between threads safely
  --> src/bin/notsync.rs:36:17
   |
36 |   needs_sync::< PlainCell >();
   |                 ^^^^^^^^^ `Cell<u64>` cannot be shared between threads safely
```

**Finding.** All three generic bounds on `SeqCell` in the family — two in
`ring_batch`, one in `ring_tls` — are bare `C : SeqCell`, and none adds `Sync`. The
gap was never reached because both implementations are `Sync` by auto-derivation
from the atomics inside them, so the requirement was satisfied by accident of
composition rather than by declaration.

`pub trait SeqCell : Sync` states the actual requirement, costs nothing for either
existing implementation, and moves the error from the eventual use site to the `impl`
that was wrong — where the message names the trait instead of a `Cell`.

**Disposition:** applied — the declaration now reads `pub trait SeqCell : Sync`, so
the requirement is stated where the trait is defined rather than inferred
per-instantiation at each of the three bound sites. A `Cell< u64 >`-backed
implementor no longer compiles at all: the error arrives at its own `impl`, naming
this trait, instead of arriving later at whatever independently demanded `Sync` and
naming a private field. `a_third_type_implements_the_trait_and_drives_through_it`
writes such an implementor from outside the crate and closes with a `requires_sync`
bound on it, so the supertrait is exercised as a requirement on a stranger rather
than satisfied by the two impls a reader can already see. What this does not buy:
the three bound sites in `ring_batch` and `ring_tls` are unchanged and still read
as bare `C : SeqCell` — the requirement reaches them, but a reader standing at one
of those signatures still has to open this crate to learn that it does. Now prints:
`pub trait SeqCell : Sync`

---

### AT46 — The One Return Value That Cannot Be Recovered Is the One You May Discard

`fetch_add`'s doc comment is the most careful sentence in the trait: it returns "the
sequence as it was *before* the advance — which is the first sequence the caller now
owns." Ownership of a slot range is transferred by that return value and recorded
nowhere else. Discard it and the advance has still happened: those sequences are
claimed, permanently, by nobody.

Discarding it compiled silently. So did discarding `load`. Only `compare_exchange`
objected — and not because the trait asked:

```
warning: unused `Result` that must be used
  --> src/bin/discard.rs:13:3
```

That warning comes from `Result` being `#[ must_use ]` in the standard library. Remove
the `Result` and the trait would have said nothing about any of its three returns.

**Finding.** The crate used `#[ must_use ]` five times and spent every one of them on
a value the caller could trivially obtain again: four constructors, and `counts()`. Not
one was on a trait method. One crate down, `ring_types` applies the attribute eleven
times — including to `next`, `advanced_by` and `distance_to`, which are pure functions
over a `Copy` type where discarding the result costs nothing at all.

So the family's convention was present, understood, and applied precisely inversely to
risk: eleven marks on returns that are free to recompute, zero on the one return whose
loss silently strands a slot range.

**Disposition:** applied — in part, and the part declined is the interesting one.
`SeqCell::fetch_add` now carries `#[ must_use = "the returned sequence is the claim —
dropping it claims a range nobody will use" ]`, which is the one return whose loss is
irreversible; the count in this crate goes from five to six and the family census one
document over goes from zero trait-method marks to one. `load` and `compare_exchange`
were deliberately left unmarked: `load`'s result is free to re-obtain, which is the
same test this finding used to condemn the other five, and `compare_exchange` already
inherits the standard library's `Result` guard, so marking either would repeat the
inverse-to-risk mistake rather than correct it. The attribute cost more than the
"three-line change with no behavioural effect" this entry predicted — because it is
written on the trait declaration it reaches every implementor and every caller, and it
broke six existing call sites across `ring_atomic` and `ring_cursor` that deliberately
discard a claim while counting advances. Each took a `let _ =` and a `Fix(AT5)` comment
saying why the discard is intended. What this does not buy: a caller who writes
`let _ =` to silence the warning gets exactly the stranded range the attribute exists
to prevent, and nothing distinguishes that from the six legitimate discards. Now
prints: `    #[ must_use ] in ring_atomic/src : 6`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/002`](002_the_report_that_is_all_public.md) | The other public surface, and what its openness permits |
| [`item/001`](../item/001_six_constructors_for_two_types.md) | The concrete types behind the trait, and their constructors |
| [`invariant/001`](../invariant/001_monotonicity_is_relied_on_and_not_required.md) | The property the trait relies on and does not require |
| [`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md) | The methods deliberately kept off the trait, and what that costs |
| [`decisions/002`](../decisions/002_a_trait_because_the_criteria_needed_two.md) | Why the trait exists at all |

### Sources

| Fact | Where |
|------|-------|
| The declaration in full, no supertrait | `ring_atomic/src/lib.rs:108-151` |
| The one contract clause in the file | `ring_atomic/src/lib.rs:145-148` |
| Five `must_use`, all on recoverable returns | Census above |
| Eleven `must_use` one crate down | `ring_types/src/`; census above |
| Three bare `C : SeqCell` bounds, no `Sync` | Census above |
| A `!Sync` implementor accepted by `claim` | Release probe, quoted above |
| The eventual error naming `Cell<u64>` | `RUSTFLAGS='--cfg demand_sync' cargo build`, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_cell_drives_through_the_trait_alone` | That a caller can work entirely through `SeqCell`, using the concrete `AtomicSeq` |
| `compare_exchange_succeeds_on_the_expected_value_and_reports_the_actual_otherwise` | The one method carrying a contract clause, at both outcomes |
| *(to create)* | No test implements `SeqCell` for a type other than the crate's own two, so nothing exercises what the trait actually requires of an implementor |
| *(to create)* | Nothing asserts `AtomicSeq : Sync`, which is the property three generic bounds silently depend on |
