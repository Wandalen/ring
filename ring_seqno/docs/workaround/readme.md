# workaround

Compensations the family applies around this crate's shape — each with what it
costs, and the condition under which it can be deleted.

### Overview Table

| ID | Name | Works around | Deletable when |
|----|------|--------------|----------------|
| 001 | [The Diagnostic That Reimplements the Readings](001_the_diagnostic_that_reimplements_the_readings.md) | Saturation erases the state a diagnostic exists to find | Never — the workaround is correct, but its ordering coupling should be pinned |
| 002 | [`laps_between` Has No Caller](002_laps_between_has_no_caller.md) | An export whose consumers each built something else instead | Either a caller appears or the function goes |

### The Two Are the Same Story from Opposite Ends

001 is a consumer that needed a reading this crate could not give and wrote its
own. 002 is a reading this crate gave that no consumer wanted.

Both are the family declining to route through `ring_seqno`, and in both cases the
crate is unchanged and unaware. That is what makes them workarounds rather than
defects: nothing here is broken, and something here is unused.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ51 | `check_seqs`'s guards | **latent hazard** | `check_seqs`'s two guards must stay in their current order or the second one panics — an undocumented coupling created by bypassing `distance_to` |
| SQ52 | Complementary comparisons | n/a — duplication | `may_claim`'s `<` and `check_seqs`'s `>` are exact complements around one boundary, in different crates, with nothing linking them |
| SQ53 | `laps_between`'s doc | n/a — doc gap | `laps_between` has zero callers outside this crate while its own doc claims it "is the reading that decides" publication safety — the role `may_claim` actually fills |

### Regenerate

The two complementary comparisons, in their two crates:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'distance_to( producer ) <' ring_seqno/src/lib.rs
command grep 'fn check_seqs' ring_debug/src/lib.rs
```

Live output:

```
  consumer.distance_to( producer ) < capacity.get() as u64
fn check_seqs( producer : Seq, consumer : Seq, capacity : Capacity ) -> Result< (), Violation >
```
