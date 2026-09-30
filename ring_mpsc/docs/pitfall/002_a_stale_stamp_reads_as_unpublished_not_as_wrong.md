# A Stale Stamp Reads as Unpublished, Not as Wrong

### Scope

- **Purpose**: Record the trap in reading the publication test — a stamp from the previous lap is a *negative* answer, and code that tests for "stamped" rather than "stamped with this sequence" reads it as a positive one.
- **Responsibility**: The trap, why it is invisible in single-lap testing, and what makes it safe here.
- **In Scope**: The stamp-equals-sequence test and its two plausible mis-readings.
- **Out of Scope**: The pattern stated positively (→ [`../pattern/002`](../pattern/002_a_sequence_stamp_as_a_lap_safe_publication_marker.md)).

### The Trap

Three tests look interchangeable and two are wrong:

| Test | Verdict on a lap-1 stamp read during lap 2 | Correct? |
|------|--------------------------------------------|----------|
| `stamp != UNSTAMPED` | published | **No** — returns the previous lap's payload |
| `stamp >= seq` | published | **No** — same, whenever the slot ran ahead |
| `stamp == seq` | not published | Yes |

**Only the third is correct, and the first is the one a reader writes by
instinct**, because `UNSTAMPED` looks like a null and "not null" looks like
"present". It is a sentinel for lap zero and nothing else.

### Why It Does Not Show Up in Testing

A ring that never wraps never produces a stale stamp. Any test that publishes
fewer than `capacity` records exercises only stamps that are either `UNSTAMPED`
or correct, so all three tests above agree — and a suite that only ever fills a
ring once would pass with the wrong one.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep -hoE '^\s*fn [a-z_0-9]+' tests/*.rs | sed 's/^ *fn //' | grep -E 'lap|stale|wrap|reuse'
```

Live output:

```
a_live_batch_still_holds_its_slots_against_reuse
a_stale_stamp_from_the_previous_lap_does_not_read_as_published
every_slot_is_reused_across_many_laps_without_loss_or_duplication
reading_past_the_end_of_a_batch_yields_none_rather_than_the_next_lap
```

`a_stale_stamp_from_the_previous_lap_does_not_read_as_published` is the test
that would catch it, and it exists because the hazard was anticipated rather
than hit. `every_slot_is_reused_across_many_laps_without_loss_or_duplication`
is the end-to-end companion.

### What Makes It Safe Here

The test is written once, in a private helper, and every reader goes through it.
The trap is therefore a hazard for anyone *modifying* this crate rather than for
anyone using it — which is why it is recorded here and not in `api/`.

**The `stamps()` accessor is where the hazard reaches outward.** It hands out
`&[ AtomicSeq ]`, and a caller reading that array directly gets no help from the
helper. It has no production caller today
(→ [`../decisions/002`](../decisions/002_the_observation_surface_kept_without_a_caller.md)),
so the exposure is latent rather than live.

### MP46 — The Wrong Test Passes Every Single-Lap Suite

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep -hoE '^\s*fn [a-z_0-9]+' tests/*.rs | sed 's/^ *fn //' | grep -E 'lap|stale|reuse'
```

Live output:

```
a_live_batch_still_holds_its_slots_against_reuse
a_stale_stamp_from_the_previous_lap_does_not_read_as_published
every_slot_is_reused_across_many_laps_without_loss_or_duplication
reading_past_the_end_of_a_batch_yields_none_rather_than_the_next_lap
```

Two tests stand between the correct test and the plausible one, and both had to
be written deliberately — neither arises from testing the happy path. A
contributor adding a fast path with the wrong comparison would be caught by
exactly these two and by nothing else.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
command grep -B2 'Do not weaken the comparison\|Equality, not' src/lib.rs
```

Live output:

```
    /// or because the stamp still holds the previous lap's sequence.
    ///
    /// **Do not weaken the comparison below.** `stamp != UNSTAMPED` and
--

        for _ in 0..max {
            // Equality, not `!= UNSTAMPED` or `>= end` — a stamp from the previous
```

**Disposition:** applied — `contiguous_end` (`src/lib.rs`) now carries a guard
doc comment naming the two weaker comparisons this finding warns against,
`stamp != UNSTAMPED` and `stamp >= end`, and stating that both read a stale
stamp from the previous lap as published, plus an inline comment repeating the
warning at the `if` itself so the two tests above are no longer the only thing
standing between the correct comparison and the plausible one. Now prints:
`**Do not weaken the comparison below.**`

### MP47 — `stamps()` Hands the Raw Array to a Caller With No Helper

Inside the crate every read goes through one private helper, so the trap is
closed. `stamps()` opens it: a caller holding the slice has the values and not
the rule, and the rule is the part that is easy to get wrong.

It has no production caller today
(→ [`../decisions/002`](../decisions/002_the_observation_surface_kept_without_a_caller.md)),
so this is latent rather than live — and it is an argument for the accessor
returning something narrower than the raw array if it survives that
decision.

**Disposition:** declined — `../decisions/002` is still **Open**: `stamps()`
and the six other observation-surface methods it covers remain kept-without-a-
caller pending that decision, and narrowing or removing `stamps()` now would
resolve, unilaterally and from inside this disposition pass, a tradeoff the
crate has explicitly deferred to a separate decision record. The finding's own
last sentence already frames the fix as conditional on that decision
surviving — it has not been decided yet, so there is nothing to change here
without preempting it.
