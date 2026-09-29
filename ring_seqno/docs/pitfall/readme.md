# pitfall

Changes and readings that pass every automatic check and are still wrong.

### Overview Table

| ID | Name | Passes | Wrong because |
|----|------|--------|---------------|
| 001 | [Implementing `may_claim` With `laps_between`](001_implementing_may_claim_with_laps_between.md) | every test, every lint, every doctest | trades a comparison for a division on the hottest path in the family |
| 002 | [Reading `free_slots` on a Narrow Target](002_reading_free_slots_on_a_narrow_target.md) | every test, on every machine anyone runs them on | the disagreement it caused was invisible where the suite executes — fixed, but the cast that would have hidden the fix is still there and no lint names it |

### What Makes These Two a Pair

Both are about the same three functions and the same fact — that `laps_between`,
`may_claim` and `free_slots` are provably equivalent at the claim boundary.

001 is what a reviewer does *because* they are equivalent: collapse three
implementations into one. The collapse is correct and costs a division.

002 is what happened *despite* their being equivalent: on a target where `usize`
narrowed, they stopped agreeing, and the test that would have noticed could not
run where the disagreement existed. The arithmetic is fixed — `free_slots` now
narrows only its already-`capacity`-bounded result — so what 002 keeps is the
walk that missed it, and the two findings that outlived the repair.

The first is a change nobody has made. The second was a property the code already
had, and the reason it kept it for as long as it did is the half worth reading.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ43 | `ring_batch::claim_gated` | n/a — observation | `ring_batch::claim_gated` takes an `order : Ordering` and hardcodes `Acquire` for two of its three atomic operations — the shape this instance warns about, already present one crate over |
| SQ44 | `Seq::next` | n/a — doc gap | `Seq::next`'s doc claims release-mode saturation; the body is `Self( self.0 + 1 )` and no `overflow-checks` override exists, so release wraps |
| SQ45 | `ring_batch`'s cast | n/a — observation | `ring_batch:323` casts `free_slots( … ) as usize` when it already returns `usize` — a no-op that would have silently absorbed a *widening* repair. The repair taken narrowed the result instead, so the cast absorbed nothing and is still there, still loaded for the next person; **SQ54** is why nothing reports it |
| SQ46 | The sweep's width | n/a — coverage | The sweep that would have caught the narrowing ran entirely inside values where `usize` and `u64` coincide, so it passed identically on a 32-bit target and the divergence at 2^32 had no test that could reach it. The suite is unchanged after the fix, because the repair was structural |
| SQ54 | `clippy::unnecessary_cast` | n/a — observation | The lint is warn-by-default in `complexity`, needs no `pedantic` opt-in, and is active in `ring_batch` — it flags four other redundant-cast shapes in that very file — yet says nothing about `ring_batch:323`: it skips casts on values returned by free functions from *other* crates. SQ45 is therefore reachable by no automated check, only by reading |

### Regenerate

The redundant cast and the addition that does not saturate:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'free_slots' ring_batch/src/lib.rs
command grep -A2 'pub const fn next' ring_types/src/id.rs
```

Live output:

```
use ring_seqno::free_slots;
/// makes `free_slots` see zero in-flight sequences on every call, so the gate
  if ( free_slots( at, behind, capacity ) as usize ) < count
  pub const fn next( self ) -> Self
  {
    Self( self.0 + 1 )
```
