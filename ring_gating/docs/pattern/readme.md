# pattern

Two shapes this crate participates in rather than invents. One is structural —
where the cursors live and who may touch them. One is an API convention the
family applies four times under three different sets of names.

### Overview Table

| ID | Name | Shape |
|----|------|-------|
| 001 | [The Owned Set with Shared Readers](001_the_owned_set_with_shared_readers.md) | One owner, many `&self` readers, mutation through the leaf's interior mutability |
| 002 | [The Predicate, the Quantity and the Reason](002_the_predicate_the_quantity_and_the_reason.md) | Ask *how many*, ask *may I*, ask *why not* — three rungs, of which this crate is the only complete instance |

### Where Each Appears

| Pattern | Instances in the family |
|---------|-------------------------|
| Owned set, shared readers | `ring_gating::GatingSet` (owner: `ring_mpsc`); the same shape via `Arc` in `ring_publish`'s tests |
| Quantity / predicate / reason | `ring_seqno` free functions (2 rungs), `ring_cursor::CursorPair` (2 rungs, over those), `ring_barrier::Barrier` (2 rungs + a wait), `ring_gating::GatingSet` (3 rungs) |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT42 | Ownership | n/a — observation | This half owns a `Vec< PaddedCursor >` and lends `&PaddedCursor`; `ring_barrier` borrows a slice it does not own. The two halves of one feature took opposite ownership decisions, each right for its side — a producer's gate outlives every consumer, a consumer's barrier does not |
| GT43 | The role the pattern does not model | n/a — observation | Every method takes the producer as a bare `Seq` argument. There is no producer type, no handle, and no way for the set to tell one caller from another — so "shared readers" is enforced by which reference a caller happens to hold, and the pattern names a role the code does not represent |
| GT44 | The quantity/predicate pair | n/a — duplication | It appears four times across four crates under three different name pairs — `free_slots`/`may_claim` twice, once as `ring_seqno`'s free functions and once as `ring_cursor`'s methods over them, then `available`/`admits` and `headroom`/`admits` — so the shape is a convention the family follows without naming, and the one pair that repeats does so across a layer boundary rather than between peers |
| GT45 | The pattern's rationale | n/a — doc gap | It is written down once, in `ring_cursor::may_claim`'s doc — "how much room" is a batch-sizing input, "may I" is a branch — and nowhere else. Three further sites spell the pair without arguing it, so the reason the family keeps two methods over one survives in a single doc comment on the crate that happens to have written it |
| GT46 | How the predicate is derived | n/a — inconsistency | `admits` is literally `count <= self.headroom( producer )` — the predicate defined in terms of the quantity, one line. `ring_cursor::may_claim` is not defined in terms of `free_slots`; it takes its own loads. Same pattern, two crates, opposite implementations, and only one is consistent with its own quantity by construction |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the quantity/predicate pair: four crates, three distinct name pairs --'
command grep -rnE 'pub (const )?fn (free_slots|may_claim|available|admits|headroom)\b' \
  --include=*.rs ring_seqno/src/ ring_cursor/src/ ring_barrier/src/ ring_gating/src/ \
  | sed 's|ring/||' | LC_ALL=C sort | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- the third member only this crate has --'
command grep -n '^  pub fn check' ring_gating/src/lib.rs | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and the one place the shape is argued rather than just spelled --'
awk '/Kept as its own method because/,/zero is the boundary/' ring_cursor/src/lib.rs
```

Live output:

```
  -- the quantity/predicate pair: four crates, three distinct name pairs --
ring_barrier/src/lib.rs:  pub fn available( &self, from : Seq ) -> u64
ring_barrier/src/lib.rs:  pub fn admits( &self, from : Seq, count : u64 ) -> bool
ring_cursor/src/lib.rs:  pub fn free_slots( &self ) -> usize
ring_cursor/src/lib.rs:  pub fn may_claim( &self ) -> bool
ring_gating/src/lib.rs:  pub fn headroom( &self, producer : Seq ) -> usize
ring_gating/src/lib.rs:  pub fn admits( &self, producer : Seq, count : usize ) -> bool
ring_seqno/src/lib.rs:pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
ring_seqno/src/lib.rs:pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
  -- the third member only this crate has --
  pub fn check( &self, producer : Seq, count : usize ) -> Result< (), RingError >
  -- and the one place the shape is argued rather than just spelled --
  /// actually asks. Kept as its own method because the two readings answer
  /// different questions — "how much room" is a batch-sizing input, "may I"
  /// is a branch — and a caller that only needs the branch should not have to
  /// know that zero is the boundary.
```

**Four crates, three name pairs, one shape, one written rationale.** The pair
that repeats — `free_slots`/`may_claim` — does so across a layer boundary, as
`ring_seqno`'s free functions and again as `ring_cursor`'s methods over them, so
three of the four occurrences are genuinely independent choices. Only this
crate adds the third member — the reading that says *why* — which is
[`002`](002_the_predicate_the_quantity_and_the_reason.md)'s subject and what
makes the convention visible as a convention rather than a coincidence of
naming.
