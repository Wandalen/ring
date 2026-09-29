# Pitfall Doc Definition

### Scope

- **Purpose**: Traps a caller of this surface can fall into, each with the failure it produces and what actually mitigates it.
- **Responsibility**: Two traps: a close that a raw producer ignores, and an `Ok` that kept nothing.
- **In Scope**: `Guarded` vs raw `Producer`; `OverflowPolicy::DropNewest` under a guarded push.
- **Out of Scope**: `free_capacity`'s split contract (→ [`ring_core/docs/pitfall/001`](../../../ring_core/docs/pitfall/001_free_capacity_carries_two_contracts.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Close Is Advisory to an Unguarded Producer](001_close_is_advisory_to_an_unguarded_producer.md) | The crate's central limitation, stated as a limitation rather than papered over | 🔄 |
| 002 | [`Ok` Does Not Mean Kept Under Drop-Newest](002_ok_does_not_mean_kept_under_drop_newest.md) | The default overflow policy makes `Refusal::Full` unreachable and a lost record silent | 🔄 |

**The split is which promise the caller is holding.** `001` is about a promise
this crate makes and cannot enforce: `close` stops publication, unless the caller
kept a raw producer, in which case it stops nothing. `002` is about a promise the
caller thinks they are holding and never had: `Ok` means the record is in the
ring, which is true under one overflow policy and false under the default.

They are separate documents rather than one "things that are not what they look
like" because their mitigations do not overlap and their failures do not look
alike. `001`'s mitigation is a type — hold a `Guarded` — and its worst outcome is
a hang at teardown with no error. `002`'s is a configuration line, and its worst
outcome is a number that is silently too high. A caller who applies one is
completely unprotected from the other.

That non-overlap is also this definition's recurring defect, and both `001`'s
findings are about a seam. **`001`'s** are about what the partition hides: its
own demonstration snippet annotates a return value `002` exists to refute, so the
two failures compose in the five lines meant to isolate one of them (SD41), and
the failure it itself calls the expensive one is the only one with no mitigation
listed and the only loop in the crate with no spin budget, in a crate that
imports `ring_wait` and budgets its two harmless waits (SD42). **`002`'s** are
about what it left outside its own frame: the batch push it declares In Scope
appears in no section, loses a record outright under the policy its mitigation
prescribes, and returns a count of records *pulled* under the default (SD43),
and its consumer-side-counting mitigation points down the dependency list at a
crate it must not depend on, while the crate that depends on **this** one already
ships the measurement and tabulates both policies (SD44).

Read together: a pitfall is written from the author's side of the call, and three
of these four findings are things visible only from the caller's side or from one
crate further out.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/pitfall
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each states a trap:           %s\n' "$( command grep -lc '^### Trap' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each states a mitigation:     %s\n' "$( command grep -lc '^### Mitigation' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'mitigations listed in all:    %s\n' "$( awk '/^### Mitigation/{f=1} /^### Regenerate/{f=0} f&&/^[0-9]\. \*\*/{ n++ } END{ print n+0 }' [0-9][0-9][0-9]_*.md )"
printf 'of those, a type not a rule:  %s\n' "$( awk '/^### Mitigation/{f=1} /^### Regenerate/{f=0} f&&/^[0-9]\. \*\*Hold a /{ n++ } END{ print n+0 }' [0-9][0-9][0-9]_*.md )"
printf 'failure rows across both:     %s\n' "$( awk '/^### Failure/{ f=1; h=0 } /^### Mitigation/{f=0} f&&/^\|---/{ h=1; next } f&&h&&/^\| /{ n++ } END{ print n+0 }' [0-9][0-9][0-9]_*.md )"
printf 'of those, silent or invisible: %s\n' "$( awk '/^### Failure/{ f=1; h=0 } /^### Mitigation/{f=0} f&&/^\|---/{ h=1; next } f&&h&&/^\| / && /[Ss]ilent|[Ii]nvisible/{ n++ } END{ print n+0 }' [0-9][0-9][0-9]_*.md )"
printf 'test names cited in tables:   %s\n' "$( awk '/^### Tests/{f=1} f&&/^\| /{ print }' [0-9][0-9][0-9]_*.md | command grep -ohE '\ba_[a-z_]+|\ban_[a-z_]+' | sort -u | wc -l )"
printf 'of those, in the suite:       %s\n' "$( cd ../..; for n in $( awk '/^### Tests/{f=1} f&&/^\| /{ print }' docs/pitfall/[0-9][0-9][0-9]_*.md | command grep -ohE '\ba_[a-z_]+|\ban_[a-z_]+' | sort -u ); do command grep -q "^fn $n(" tests/shutdown_test.rs && echo x; done | wc -l )"
printf 'the guarantee both rest on:   %s\n' "$( cd ../..; command grep -o 'pub fn is_closed( &self ) -> bool' src/lib.rs | head -1 )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
each states a trap:           2
each states a mitigation:     2
mitigations listed in all:    6
of those, a type not a rule:  1
failure rows across both:     6
of those, silent or invisible: 4
test names cited in tables:   4
of those, in the suite:       4
the guarantee both rest on:   pub fn is_closed( &self ) -> bool
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD41 | the trap's own snippet annotates a return value the next document exists to refute | **misleading doc** | [`001`](001_close_is_advisory_to_an_unguarded_producer.md)'s five-line demonstration ends `producer.try_push( record ).unwrap();   // a raw ring_core::Producer. Accepted.`, and *Accepted* is exactly the word [`002`](002_ok_does_not_mean_kept_under_drop_newest.md) was written to remove — under `RingConfig`'s default `DropNewest` that call returns `Ok( () )` for a discarded record, so on a full ring the `unwrap()` succeeds and nothing was accepted; the trap itself is unharmed (a raw producer does publish into a closed ring with room), but the snippet demonstrates two failures while annotating one, and this document's Out of Scope bullet files the other as *"a different failure with a different mitigation"* — true of the mitigations, false of the failures, which compose precisely here: an unguarded producer under the default policy has two independent ways to report a publication that did not happen, and a caller who applies one mitigation still has the other; the general shape is that a pitfall directory partitions by cause while a caller meets failures by symptom, so both documents are correct in isolation and neither owns the state a default-configured crate is actually in. |
| SD42 | the failure the document calls expensive was the only one with no mitigation and the only loop with no budget; it has a budget now | **latent hazard** | [`001`](001_close_is_advisory_to_an_unguarded_producer.md)'s Failure table has three rows and its own prose names row 2 — `drain_all` failing to terminate — as *"the one that costs a debugging session"*, then lists three mitigations of which zero bound the loop: all three prevent a raw producer existing or publishing, so a caller already holding one, which is the situation every mitigation assumes was avoided, gets nothing; the crate is not short of machinery — `wait_for_close` and `for_space_or_close` each take `spins : usize`, pass it to `ring_wait`, and return `Err( Empty )`/`Err( Full )` on exhaustion, while `drain_all` and `discard_all` take no budget and return `usize`, leaving no value in which exhaustion could be reported — and the asymmetry inverts the risk, since the two budgeted functions wait on a flag another thread sets while the two unbudgeted ones loop on a ring another thread fills, which is the case this document says hangs; the cost is not the hang, which is the honest price of [`invariant/001`](../invariant/001_exactly_one_liveness_flag.md), but that it is indistinguishable from a slow drain forever, and a `drain_all_bounded( …, spins )` returning `Err( Empty )` was recorded as a decision nowhere — the two on file are about `into_inner` and token uniqueness; `Stopped::drain_all_bounded( consumer, out, budget )` now exists, returning `Ok( n )` once the ring is observed empty and `Err( RingError::Empty )` when the budget runs out first, with `a_bounded_drain_separates_finishing_from_running_out` pinning both arms and the zero-budget reading against one ring. |
| SD43 | the batch push is in scope, absent from every section, and loses a record under the policy this document prescribes — now exercised in both directions, with the loss left where it lives | **latent hazard** | `Guarded::try_push_batch` is second in [`002`](002_ok_does_not_mean_kept_under_drop_newest.md)'s In Scope list and is mentioned zero times between the Trap heading and the recipe, against three for `try_push`, and it fails in both of the document's directions and worse in each: it returns `usize`, so there is no `Refusal::Full( record )` and no arm to match, and the two-refusals framing the document opens with has no batch analogue at all; under `Fail` — the policy mitigation 1 prescribes with *"Under `Fail`, `Ok` does mean kept"* — `ring_core`'s loop is `if self.try_push( record ).is_err() { break; }` where `try_push` takes the record by value and returns `Err( record )`, so `.is_err()` discards it, a loss `ring_core`'s own doctest asserts (`records.next() == Some( 4 ), "record 3 was consumed by the refusal"`); under `DropNewest` `try_push` answers `Resolution::DroppedIncoming => Ok( () )` so the `break` never fires and the returned count is records *pulled*, the document's own thesis promoted from a unit to a number a caller sums; and neither was exercised, since the suite's two guarded batch calls both sat in `a_closed_batch_push_consumes_nothing` offering three records to an eight-slot ring, passing on capacity rather than policy; `a_batch_into_a_full_refusing_ring_destroys_the_record_that_was_refused` and `a_batch_into_a_full_drop_newest_ring_counts_records_it_did_not_keep` now drive both policies against a full ring, while the underlying loss stays with `ring_core::Producer::try_push_batch`, whose fix is a family-wide signature change rather than a wrapper edit. |
| SD44 | the recommended measurement points at a non-dependency while the built one ships in the crate that depends on this | **misleading doc** | [`002`](002_ok_does_not_mean_kept_under_drop_newest.md)'s mitigation 3 has the right shape — a consumer-side count is the only number separating *published* from *reported published* — and names `ring_stats`, which does record drops via `record_drop`/`dropped`/`dropped_total` but is called today only by `ring_bench` and `ring_overflow`, so acting on the advice means the caller wiring a crate at their own push site that this crate has no edge to and argues against acquiring (→ [`integration/001`](../integration/001_family_dependency_seam.md)); the measurement meanwhile already exists in the direction nothing was looking, in `ring_testkit`, the one crate that depends on **this** one, as `Outcome::vanished()` = `accepted.saturating_sub( received.len() + in_ring_at_end )`, with `accepted` documented *"Not the same as how many it stored"* and both policies tabulated in its module header (`Fail: accepted=4 refused_full=4 … vanished=0` against `DropNewest: accepted=8 refused_full=0 … vanished=4`), all derived from a loop over `Guarded::try_push` matching three arms — so the workspace's most complete statement of this pitfall sits in a crate this document never mentions and needs no new edge to cite, unreachable by reading down a dependency list, which is where both mitigation 3 and [`integration/001`](../integration/001_family_dependency_seam.md) were looking; SD20 measured the blind spot and this is what it cost, the narrower lesson being that the mitigation section is the one place a doc reaches outside its crate and reverse dependencies are where mitigations accumulate, because they are the crates that hit the trap. |
