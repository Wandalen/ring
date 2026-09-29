# algorithm

Step sequences and their termination arguments. `ring_index` has two: a fold
that is one bitwise `and`, and a range expansion that is that fold applied
`count` times. Neither loops unboundedly, and only one of them can fail — which
is the whole content of this definition.

The termination argument is trivial in both cases and therefore not the
interesting part. What is interesting is where each algorithm's correctness
actually lives: `of` is correct because a constructor in `ring_types` refuses
non-powers-of-two, and `run` is correct for exactly the inputs a caller is
trusted to supply. Both crates state those preconditions; neither enforces them
at the point of use.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_one_and_of_a_mask.md) | One `and` of a Mask | `of`'s body, the borrowed mask, and why there is no error path |
| [002](002_a_run_is_the_fold_applied_count_times.md) | A Run Is the Fold Applied `count` Times | `run`'s loop, its allocation, and the lazy version built elsewhere |

## Where Each Algorithm's Correctness Lives

Neither function validates anything. `of` is total because `Capacity::new`
rejected every input that would break the mask identity, one crate up. `run` is
total for the same reason plus one more: it assumes `start + ( count - 1 )` does
not overflow `u64`, which nothing checks and which the type system does not
express.

The asymmetry matters because the two preconditions have different enforcement.
The first is structural — a `Capacity` that violates it cannot be constructed.
The second is a convention: `run`'s doc explains that a batch claim is
contiguous and therefore wraps at most once, which is true of batch claims and
not of the `count` parameter it actually accepts.

## The Fold Is Not Where the Family Spends Its Time

The module comment argues the power-of-two constraint pays for itself because
the fold "sits on the claim path and the read path of every single operation the
family performs." The claim path does not fold —
[`integration/001`](../integration/001_two_dependents_and_a_third_that_did_it_again.md)
IX12 records that `ring_claim`, `ring_publish` and `ring_consume` do not depend
on this crate at all. Folding happens when a slot is touched, not when a
sequence is claimed.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^\/\/\/ assert_eq!\( of\( Seq\( 13 \), cap \), SlotIndex\( 5 \) \);$/{ n1 = NR } n1 && NR >= n1 + 2 && NR <= n1 + 6 { print } /^\/\/\/ assert_eq!\( run\( Seq\( 0 \), 0, cap \), vec!\[\] \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 6 { print }' ring_index/src/lib.rs
command grep -m1 -A3 -F '  pub const fn mask( self ) -> usize' ring_types/src/capacity.rs
command grep -m1 -A3 -F '  pub const fn advanced_by( self, n : u64 ) -> Self' ring_types/src/id.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX1 | `ring_index` | n/a — observation | Totality is bought by a constructor one crate away; neither `mask` nor `of` checks anything |
| IX2 | `ring_index` | n/a — coverage | The mask identity holds at `u64::MAX`; no test goes above `u32::MAX` |
| IX3 | `ring_index` | **misleading doc** | `run`'s doc states "wraps at most once" as a property of `run`; it is a property of batch claims, and `run` accepts any `count` |
| IX4 | `ring_batch` | n/a — duplication | `ring_batch::drain_order` is a lazy `run` built from `of`; the one crate with the use case did not call `run` |
