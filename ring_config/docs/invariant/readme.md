# invariant

Three properties hold for every `RingConfig` that exists: `producers` is at least
one, `batch` is between one and the capacity inclusive, and the four setters may
be applied in any order without changing the result. All three are true, all three
are cheap, and none of them is written down where the person who could break it
would be looking.

The two ranges are stated on the getters, which is the reader's end rather than
the maintainer's. Commutation is stated in a test comment without its reason. And
one of the two ranges is not even enforced entirely in this crate — its lower
bound leans on a guarantee `ring_types` makes and names exactly one beneficiary
for, in a sentence that does not mention this one.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_two_ranges_that_cannot_be_violated.md) | Two Ranges That Cannot Be Violated | Both numeric invariants, where each starts, where each is preserved, and the cross-crate guarantee one borrows |
| [002](002_the_setters_commute_and_one_absence_is_why.md) | The Setters Commute, and One Absence Is Why | The single cross-field read, the write-once field it reads, and what the commutation test does not reach |

## Documented at the Reader's End

`:185` says the producer count is "always at least one". `:197` says the batch is
"always between one and the capacity inclusive". Those are the crate's only two
statements of a property rather than of a call's outcome, and both sit on a
getter — read by a caller deciding whether to trust a value, not by whoever is
about to add a fifth setter.

The declaration carries nothing. `with_batch`'s doc justifies its own clamp
without saying that the resulting bound is a contract. Nothing tells a maintainer
which properties a change would have to preserve, which is a cheap gap to leave
open and an expensive one to discover.

## One Absence Doing Structural Work

`with_batch` is the only setter in the crate that reads a second field, and the
field it reads — `capacity` — is written exactly once, inside `new`, and has no
setter at all. That is the whole mechanism behind commutation: every setter's
effect is a function of its argument and a constant, and each writes a field no
other setter touches.

`with_capacity` occurs zero times, and its absence is load-bearing. Adding it
would make `with_batch`'s read observe a mutable field and let a chain end with a
batch larger than its ring — breaking the commutation property and the documented
batch range together, with no comment at either site to say so.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two ranges, stated once each, both on a getter --'
command grep -n 'always at least one\|always between one and the capacity' ring_config/src/lib.rs
echo '  -- the four setter bodies that must preserve them --'
command grep -n 'self\.[a-z_]* = \|let capped' ring_config/src/lib.rs
echo '  -- occurrences of with_capacity, the setter whose absence makes them commute --'
command grep -c 'with_capacity' ring_config/src/lib.rs || true
echo '  -- and the cross-crate guarantee the batch range borrows --'
command grep -n 'CapacityZero' ring_types/src/capacity.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RC21 | `ring_config` | n/a — doc gap | The crate's two invariants — `producers >= 1` and `1 <= batch <= capacity` — are each stated exactly once and both statements sit on a getter (`:185`, `:197`), so the reader who needs them is the caller deciding whether to trust a value rather than the maintainer adding a fifth setter, and the declaration at `:41-49` says nothing about either while `with_batch`'s doc justifies its clamp without claiming the resulting bound holds for the type |
| RC22 | `ring_config` | n/a — doc gap | `with_batch` caps against the capacity and then raises a zero to one, which is correct only because `Capacity` can never be zero — a guarantee made in `ring_types` at `capacity.rs:44` and whose own type doc names exactly one beneficiary, `mask()` — so one of this crate's two documented ranges is enforced across a crate boundary and neither end records the connection |
| RC23 | `ring_config` | n/a — doc gap | The setters commute because `with_batch` is the only cross-field read in the crate and the field it reads is written once at `:71` and has no setter — `with_capacity` occurs zero times — so order-independence is a consequence of an absence rather than a property anything maintains, and neither the declaration, nor `with_batch`'s doc, nor the test's own comment records that adding the obvious missing setter would end it |
| RC24 | `ring_config` | n/a — coverage | `setters_commute` builds its two chains from capacity 32 with `with_producers( 2 )` and `with_batch( 8 )`, values already inside both ranges, so no clamp fires in either order and the test exercises only four assignments to disjoint fields — a probe running all twenty-four orderings with both clamps firing lands on one identical record, which is the case the test's own name promises and the one it does not reach |
