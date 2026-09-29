# API: The Return Value That Is a Claim

### Scope

**Purpose:** Record what each `SeqCell` method hands back, which of those return
values is a caller's new property rather than information, and what the crate does
to stop that value being dropped.

**Responsibility:** The four trait method signatures, the five `#[ must_use ]`
attributes the crate spends, and the family-wide distribution of that attribute.

**In Scope:** `ring_atomic/src/lib.rs:108-151`, `:196`, `:204`, `:350`,
`:365`, `:409`.

**Out of Scope:** What the methods issue to the hardware is
[`algorithm/001`](../algorithm/001_one_intrinsic_or_two.md). The `counts` return
value, which is `must_use` and is only information, is
[`item/002`](../item/002_counts_the_method_that_is_not_a_snapshot.md).

---

## Four Methods, Three Returns, Five Attributes — None on the Same Item

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the trait: four methods, three of them handing something back --'
command grep -m1 -A20 -F 'pub trait SeqCell' ring_atomic/src/lib.rs | command grep -E '^  fn [a-z_]+\('
echo '  -- the one whose return value is the sequence range the caller now owns --'
command grep -m1 -A2 -F '  /// Advance by `n` and return the sequence as it was *before* the advance —' ring_atomic/src/lib.rs
echo '  -- where the crate does spend the attribute --'
command grep -A1 'must_use' ring_atomic/src/lib.rs | command grep -E 'pub (const )?fn' | sed 's/^/    /'
echo '  -- and family-wide --'
tot=0
for c in $( ls ring | command grep '^ring_' )
do
  f="ring/$c/src/lib.rs"; [ -f "$f" ] || continue
  tot=$(( tot + $( command grep -c 'must_use' "$f" || true ) ))
done
echo "    on inherent items, all 33 crates : $tot"
tr=0
for c in $( ls ring | command grep '^ring_' )
do
  f="ring/$c/src/lib.rs"; [ -f "$f" ] || continue
  tr=$(( tr + $( awk '/^pub trait/{i=1} i&&/^}/{i=0} i&&/must_use/{c++} END{print c+0}' "$f" ) ))
done
echo "    on any trait method              : $tr"
```

Live output:

```
  -- the trait: four methods, three of them handing something back --
  fn load( &self, order : Ordering ) -> Seq;
  fn store( &self, value : Seq, order : Ordering );
  -- the one whose return value is the sequence range the caller now owns --
  /// Advance by `n` and return the sequence as it was *before* the advance —
  /// which is the first sequence the caller now owns.
  ///
  -- where the crate does spend the attribute --
      pub const fn new( value : Seq ) -> Self
      pub fn new( value : Seq ) -> Self
      pub const fn new( value : Seq ) -> Self
      pub fn new( value : Seq ) -> Self
      pub fn counts( &self ) -> OpCounts
  -- and family-wide --
    on inherent items, all 33 crates : 279
    on any trait method              : 1
```

Five attributes, all on inherent items. Four are constructors and the fifth
returns a report. None is on a method whose return value is a claim.

---

### AT5 — `fetch_add`'s Own Doc Says the Return Value Is Property, and Nothing Stops It Being Dropped

The trait states the contract plainly: `fetch_add` returns "the sequence as it was
*before* the advance — which is the first sequence the caller now owns." The
advance is unconditional and there is no way to undo it, so a caller who ignores
the returned `Seq` has moved the cell forward by `n` and lost the only reference to
the `n` sequences it just took.

That is the single most consequential value in the crate, and it was the one value
the compiler would let you drop in silence. `cell.fetch_add( 8, order );` as a bare
statement compiled clean under `-D warnings`.

Meanwhile all five `#[ must_use ]` the crate carried sat where the consequence
of dropping is nothing at all. `AtomicSeq::new` and `CountingSeq::new` return a
fresh cell that owns no state anyone else can observe; dropping one wastes a
`AtomicU64` and changes nothing. `counts` returns a report; dropping it wastes four
`Relaxed` loads.

**Finding.** The attribute was spent on four constructors and a getter — the items
where forgetting the result is free — and withheld from the three methods where
forgetting it is irreversible. The crate's own doc comment named the property that
would be lost, one line above the signature it failed to guard.

The cases where discarding really is intended were already in the family, and every
one of them is a counting test:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the only deliberate discard of a claimed sequence in 33 crates --'
command grep -m1 -A6 -F '      scope.spawn( ||' ring_cursor/tests/cursor_test.rs
echo '  -- through the wrapper that forwards to this trait --'
command grep -m1 -A3 -F '  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq' ring_cursor/src/lib.rs
```

Live output:

```
  -- the only deliberate discard of a claimed sequence in 33 crates --
      scope.spawn( ||
      {
        for _ in 0..PER_THREAD
        {
          // Fix(AT5): the claim is deliberately discarded — this loop counts advances,
          // it does not consume the ranges they hand out.
          let _ = cursor.fetch_add( 1, Ordering::AcqRel );
  -- through the wrapper that forwards to this trait --
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  {
    self.0.get().fetch_add( n, order )
  }
```

This entry counted one such site. That was wrong, and the error was in the
direction that flatters the fix: the census above finds the first occurrence in one
file, and adding the attribute turned up **six** — three in `ring_cursor`'s test
file and three in `ring_atomic`'s own, each a loop that counts advances without
consuming the ranges they hand out. The cost of closing the gap was six `let _ =`
bindings, not one, and the shape of the mistake is worth keeping: a census written
with `-m1` answers "does this exist" and reads like an answer to "how many".

**Disposition:** applied — to `fetch_add` only, and the method now carries
`#[ must_use = "the returned sequence is the claim — dropping it claims a range
nobody will use" ]` on the **trait declaration**, so it reaches every present and
future implementor including `ring_cursor`'s forwarding impl — which is the
propagation AT6 measured and found unused. `load` and `compare_exchange` were left
alone on purpose: `load`'s result is free to re-obtain, so marking it would repeat
the inverse-to-risk error this entry is about, and `compare_exchange` already
inherits the standard library's `Result` guard. Each of the six discard sites the
change surfaced took a `let _ =` and a comment naming why the claim is dropped there.
What this does not buy: `let _ =` silences the attribute completely, so a caller who
strands a range and a caller who deliberately counts advances write the identical
line — the guard catches the accident of *forgetting*, never the decision to
discard. Now prints: `    on any trait method              : 1`

---

### AT6 — The Attribute Propagates to Both Impls, Proved, and the Family Never Uses It This Way

`#[ must_use ]` written on the *trait declaration* applies to every implementation,
including ones outside this crate that the author never sees. Patching the
declaration and compiling a file that drops two of the three returns:

```
error: unused return value of `fetch_add` that must be used
 --> tests/dropped.rs:9:3
  = note: `-D unused-must-use` implied by `-D warnings`
error: unused return value of `load` that must be used
  --> tests/dropped.rs:10:3
```

That file compiled clean under `-D warnings` against the crate as it then stood.
Two errors, at the exact sites, from two attributes. Half of that probe is now the
crate: AT5 put the attribute on `fetch_add`'s declaration, so the first of those two
errors is real and the second is still hypothetical.

**Finding.** The guard is one attribute per method and it reaches every present and
future implementor, including `ring_cursor`'s forwarding impl, which is how the
value actually reaches the ring assemblies. Nothing in the crate, the family, or
the design corpus argued against it; the mechanism was simply unused.

Not once, and not just here. When this was written the attribute appeared 255 times
across all 33 crates and never on a trait method — not on `SeqCell`, not on
`ring_event`'s `Fill` or `Peek`, not on `ring_slot`'s `Slot`. Every one of those 255
was inherent, so every one applied to exactly the item it was written on. The team
had a strong habit of reaching for `must_use`, and a blind spot for the one form of
it that crosses a crate boundary.

The census at the top of this document reads 263 and 1 now. One method of one trait
in one crate moved, which narrows the blind spot by exactly that much and closes
nothing: `ring_event`'s `Fill` and `Peek` and `ring_slot`'s `Slot` are still bare,
and the three-crates-one-hazard split below is unchanged.

The same gap is open one tier up: `ring_batch::claim` returns a `BatchClaim`
covering `count` sequences and carries no `must_use` either
([`ring_batch` § BA6](../../../ring_batch/docs/api/001_twelve_items_seven_must_use.md)),
while `ring_claim` three tiers above puts the attribute on the *type* with a
message naming the irreversibility rule. Three crates, one hazard, three different
amounts of protection, and the least protected is the primitive all of them sit on.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/001`](../type/001_what_the_trait_promises.md) | The same method read as a contract rather than a signature |
| [`invariant/001`](../invariant/001_monotonicity_is_relied_on_and_not_required.md) | The monotone advance that makes a dropped return unrecoverable and not merely wasteful |
| [`pattern/001`](../pattern/001_one_place_where_an_atomic_is_created.md) | The trait as the family's single atomic construction point |
| [`integration/002`](../integration/002_five_crates_downstream.md) | The five crates the missing guard would have covered |

### Sources

| Fact | Where |
|------|-------|
| The four method signatures | `ring_atomic/src/lib.rs:108-151` |
| `fetch_add`'s ownership sentence | `ring_atomic/src/lib.rs:116-117` |
| The `must_use` attributes, five inherent and one on the trait | `ring_atomic/src/lib.rs`; census above |
| The deliberate discards, three per test file in two crates | `ring_cursor/tests/cursor_test.rs`; `ring_atomic/tests/atomic_test.rs` |
| 255 inherent and 0 on traits when written; 263 and 1 now | Censuses above |
| The two errors from the patched declaration | Patched-copy probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_cell_drives_through_the_trait_alone` | That the trait is usable as `&dyn SeqCell`, which is what makes the declaration-level attribute reach every caller |
| `both_cells_and_the_object_form_are_sync` | The family's other declaration-level guard, asserted three ways rather than left to auto-derivation |
| *(to create)* | Nothing asserts that a dropped `fetch_add` is a compile error — the attribute makes it one under `-D warnings`, but proving that needs a compile-fail harness the family does not have |
