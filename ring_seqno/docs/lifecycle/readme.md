# lifecycle

A crate with no state has no lifecycle of its own. Both instances are about the
lifecycle of something else, seen through this crate's readings.

### Overview Table

| ID | Name | Subject |
|----|------|---------|
| 001 | [One Pair Across One Lap](001_one_pair_across_one_lap.md) | Every reading tabulated at every position of a capacity-4 ring's full cycle |
| 002 | [The Validity Window of an Answer](002_the_validity_window_of_an_answer.md) | How long a reading stays true after it is returned, and which of its two answers decays unsafely |

### The Split

001 is the **spatial** lifecycle: hold time still, walk the pair through a lap,
and read off what each function says. It shows that four of the five readings
depend only on the difference between the two positions, which is why a ring's
cycle is a cycle in the readings and a straight line in the sequences.

002 is the **temporal** one: the crate returns a value computed from positions
that were already moving. Nothing in this crate can express that, so the whole
question lives at the call sites — and the two consumers that got it right
(`ring_claim`, `ring_wait`) each solved it with a different mechanism.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ31 | Difference alone | n/a — observation | Every reading is a function of the difference alone; `slowest` alone is equivariant rather than invariant |
| SQ32 | The doctest capacities | n/a — inconsistency | Three of the four pair functions demonstrate themselves at capacity 4 and `laps_between` at 8, so the one function whose doc claims it "is the reading that decides" is also the one whose example cannot be read line-for-line against its neighbours |
| SQ33 | Answer decay | n/a — doc gap | `may_claim`'s `true` and `false` decay in opposite directions, and the crate documents neither — the warning exists only as a comment in `ring_claim` |
| SQ34 | The half-open convention | n/a — doc gap | `pending`'s doc is the only one of five that states the half-open interval convention ("up to but not including"), and the other four depend on it silently |

### Regenerate

The capacities the doctests hold each reading at:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'Capacity::new(' ring_seqno/src/lib.rs
```

Live output:

```
/// let cap = Capacity::new( 8 ).unwrap();
/// let cap = Capacity::new( 4 ).unwrap();
/// let cap = Capacity::new( 4 ).unwrap();
```
