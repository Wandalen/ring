# algorithm

The computations this crate performs, and what distinguishes them from each
other given that four of the five reduce to one subtraction.

### Overview Table

| ID | Name | Inputs | Result |
|----|------|--------|--------|
| 001 | [Four Readings of One Subtraction](001_four_readings_of_one_subtraction.md) | Two `Seq`, sometimes a `Capacity` | A lap count, a predicate, a free count, a pending count |
| 002 | [The `slowest` Fold](002_the_slowest_fold.md) | A `&[ Seq ]` | `Option< Seq >` |

### Why the Split Is Where It Is

The two instances divide on arity, not on subject. Instance 001 covers
everything binary — `laps_between`, `may_claim`, `free_slots`, `pending` — because
all four compute `consumer.distance_to( producer )` and then differ only in how
they present it. Instance 002 covers `slowest` alone, which takes a slice, is the
only function here that can allocate at its call site, and is the only one that
cannot be `const`.

That split produced the crate's two structural findings:

| Finding | Where |
|---------|-------|
| The four binary readings form an equivalence lattice with exactly one untested edge — `may_claim ⟺ laps_between == 0` is never asserted, while `may_claim ⟺ free_slots > 0` is swept across two laps | [001](001_four_readings_of_one_subtraction.md) |
| Two crates fold a minimum and both call it `slowest`; the decision **did** fork, when `b7e075ca` stopped `ring_cursor` delegating here — and this crate's own SQ35 is the record of it | [002](002_the_slowest_fold.md) |


### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ1 | The three boundary equivalences | n/a — coverage | Three predicates state the same boundary and only `may_claim` against `free_slots > 0` is swept; `may_claim` against `laps_between == 0` has no assertion anywhere in the crate |
| SQ2 | The untested equivalence | **latent hazard** | The single unasserted equivalence is also the only one whose statement requires reversing argument order, so the one edge no test pins is the edge a caller is most likely to write backwards |
| SQ3 | The family's three divisions | n/a — observation | All 33 crates contain exactly three `/` operators outside comments — `ring_align`'s cache-line membership test, `laps_between`'s lap count, and `ring_bench::destination_of`'s producer decode inside a torn-read assertion. None sits on a hot write or read path, and the second is still the only one in a function with no caller at all |
| SQ4 | `.min()` | n/a — observation | Two `.min()` folds exist across the 33 crates and both are named `slowest`: this crate's `iter().copied().min()` and `ring_cursor`'s `iter().map( load ).min()`. The reading that this was the one decision *not* to fork is contradicted by SQ35 in this crate's own corpus — `b7e075ca` forked it deliberately, to delete the allocation the `&[ Seq ]` parameter forced |

### Regenerate

Every division and every fold in the family. The filter drops every `//`
comment, not only `///` and `//!` — `ring_align` carries a plain `//` line that
names `a / CACHE_LINE` in prose, and counting it would put a comment in a census
of operators:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn '[^/*] / \|\.min()' --include=*.rs ring_*/src/ \
  | command grep -vE ':[[:space:]]*//' | sed -E 's/:[0-9]+:/: /' | LC_ALL=C sort
```

Live output:

```
ring_align/src/lib.rs:     a / CACHE_LINE != b / CACHE_LINE
ring_bench/src/lib.rs:     let producer = (record / workload.records_per_producer() as Record) as usize;
ring_cursor/src/lib.rs:     cursors.iter().map(|c| c.load(GATING)).min()
ring_seqno/src/lib.rs:     cursors.iter().copied().min()
ring_seqno/src/lib.rs:     earlier.distance_to(later) / capacity.get() as u64
```
