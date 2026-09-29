# item

A per-function inventory of the crate's two types — signature, `const`ness,
atomic loads, and what covers each.

### Overview Table

| ID | Name | Type | Functions |
|----|------|------|----------:|
| 001 | [`PaddedCursor` and Its Functions](001_padded_cursor_and_its_functions.md) | `PaddedCursor` | 6 — 2 inherent, 4 by trait impl |
| 002 | [`CursorPair` and Its Readings](002_cursor_pair_and_its_readings.md) | `CursorPair` | 8, all inherent |

Plus three module-level items covered under [`api/`](../api/readme.md): the
`SeqCell` re-export, `GATING`, and `slowest`. Fourteen functions and three
module-level items is the whole public surface.

### Why an Inventory Separate From `api/`

`api/` argues about the *surface* — what it promises, why it is shaped that way,
what a caller may rely on. This definition answers narrower questions that come up
while reading code:

- Is this function `const`?
- How many atomic loads does calling it cost?
- Is it actually called anywhere, or only tested?

Those are properties of individual items rather than of the surface, and two of
them produced findings that the surface-level view did not surface:

| Finding | Where |
|---------|-------|
| `PaddedCursor::fetch_add` has **zero** callers outside this crate's own tests — it exists because `SeqCell` requires it, and every producer in the family advances with `compare_exchange` instead | [001](001_padded_cursor_and_its_functions.md) |
| `CursorPair::on_distinct_lines` is not `const` even though its callee in `ring_align` is — `addr`'s pointer-to-integer cast is what blocks it | [002](002_cursor_pair_and_its_readings.md) |

Neither is a defect. Both are the kind of thing a reader wastes twenty minutes
rediscovering.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- declared public --'
command grep -cE '^(pub |  pub )(const|fn|struct|use)' ring_cursor/src/lib.rs
echo '  -- reachable, and undeclarable: a trait impl method carries no pub --'
command grep -E '^  fn (load|store|fetch_add|compare_exchange)' ring_cursor/src/lib.rs
```

Live output:

```
  -- declared public --
17
  -- reachable, and undeclarable: a trait impl method carries no pub --
  fn load( &self, order : Ordering ) -> Seq
  fn store( &self, value : Seq, order : Ordering )
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  fn compare_exchange( &self, current : Seq, new : Seq, success : Ordering, failure : Ordering )
```

**A declaration census, and four items no declaration census can reach.** A
method in a trait impl carries no `pub` of its own — it is public because the
trait is — so any `^pub`-anchored count of this crate's surface is short by
exactly these four, and they are four of the most-called items in it.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU25 | `fetch_add` | n/a — coverage | Exactly one caller outside the crate names the receiver's type, and it is `ring_claim`'s `writing_through_the_cursor_accessor_defeats_the_gate` — a test asserting the method is reachable across a boundary where it should not be. `ring_batch`'s generic call site never receives a `PaddedCursor` at all, since its only consumer passes an `AtomicSeq`, so the method's entire external exercise is one deliberate proof of a leak |
| CU26 | `addr` | n/a — observation | `core::ptr::from_ref( self )` takes the address of the `PaddedCursor`, not of the `AtomicSeq` inside it. The two coincide only because `CacheAligned` is a single-field wrapper whose payload sits at offset zero — a property of another crate, asserted in neither |
| CU27 | `may_claim` | n/a — duplication | Documented as "exactly `free_slots` being non-zero", and implemented as its own two-load reading through `ring_seqno::may_claim` rather than as a call to `free_slots`. The two can therefore disagree — they read the cursors at different instants — and the doc comment states the equivalence as though they could not |
| CU28 | `#[ must_use ]` | n/a — observation | Thirteen attributes across a surface of seventeen items: every constructor and every reading carries one, and only the four trait-impl methods do not, because the trait declares them. It is the most consistently applied convention in the crate and no document names it |
