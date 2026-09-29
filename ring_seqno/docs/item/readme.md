# item

A per-function inventory — signature, casts, constness, and what covers each.

### Overview Table

| ID | Name | Functions | Split on |
|----|------|----------:|----------|
| 001 | [The Three Capacity Readings](001_the_three_capacity_readings.md) | 3 | takes a `Capacity` |
| 002 | [The Two Readings Without a Capacity](002_the_two_readings_without_a_capacity.md) | 2 | does not |

Five functions is the whole crate — there are no types, constants or re-exports
to inventory alongside them. See [`api/001`](../api/001_five_functions_and_no_types.md).

### Why an Inventory Separate From `api/`

`api/` argues about the surface: what it promises, what shape it has, what a
caller may rely on. This definition answers per-item questions that come up while
reading a call site:

- Which direction does this function's cast go?
- Is it `const`, and could it be?
- Is it actually called anywhere, or only tested?

The split on `Capacity` is not cosmetic. It is exactly the split on **which
functions perform a cast**, and the cast is where the crate's one portability
hazard lives:

| Group | Casts | Consequence |
|-------|-------|-------------|
| Takes a `Capacity` (001) | all three widen `usize → u64` to compare; `free_slots` then **narrows** its already-`capacity`-bounded result back to `usize` | The three agree on every target — they once did not ([`non_functional_requirement/002`](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md) § SQ37) |
| No `Capacity` (002) | none | `pending` and `slowest` are width-independent |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ26 | `free_slots` | n/a — observation | `free_slots` is the only reading that narrows at all, and it is the one two other crates decide writes with — which is why its narrowing was moved onto the already-bounded result rather than left on the raw distance |
| SQ27 | `laps_between`'s callers | n/a — observation | `laps_between` has zero callers outside this crate — none in any of the other 32 crates, in production or in test code |
| SQ28 | The three casts | **latent hazard** | The crate crossed a width boundary three times — `capacity.get() as u64` twice, which cannot lose, and `distance_to( … ) as usize` once, which could — and named the risk at exactly one of the three. The losing cast is gone; line 98 now widens, saturates, then narrows a result bounded by `capacity` ([`non_functional_requirement/002`](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md) § SQ37) |
| SQ29 | `pending` | n/a — duplication | `pending` is `Seq::distance_to` with the arguments swapped and nothing added |
| SQ30 | The two capacity-free readings | n/a — observation | `pending` and `slowest` are the two functions that take no `Capacity`, and they are also the two whose answers are not bounded by one — the parameter list states the mathematical fact |

### Regenerate

Five signatures and every width crossing between them:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -E '^\s*pub fn |as u64|as usize' ring_seqno/src/lib.rs
```

Live output:

```
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
  earlier.distance_to( later ) / capacity.get() as u64
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
  consumer.distance_to( producer ) < capacity.get() as u64
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
pub fn pending( producer : Seq, consumer : Seq ) -> u64
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
```
