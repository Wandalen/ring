# Pitfall: A Bigger Budget Is Not a Bigger Batch

### Scope

- **Purpose**: Record that `push_batch_within( .., Budget::new( 9 ) )` against a full ring costs one ring operation, not nine.
- **Responsibility**: The extra exit the batch helper has, why it is right, and the two shapes of caller code that assume otherwise.
- **In Scope**: `push_batch_within`'s early exit and its interaction with the budget; the absence of any budget-to-count relationship.
- **Out of Scope**: Budget magnitude as a latency decision (→ [`001_non_parking_is_not_bounded_latency.md`](001_non_parking_is_not_bounded_latency.md)).

### Trap

Reading the budget as "how hard it tries" and the batch helper as "the budgeted
version of `try_push_batch`" gives the wrong model of both. The batch helper has
an extra exit the single-record one does not:

```
if this attempt moved zero records, stop — whatever the budget says
```

So the observed behaviour splits by what the *first* attempt does:

| First attempt | Budget 9 actually spends | Records published | Records gone from the iterator |
|---|---|---|---|
| Moved 4 of 12 | 2 attempts — the second moves nothing and ends it | 4 | **6** — the four published, plus one eaten by each refusal |
| Moved 0 of 12 | **1 attempt** | 0 | **1** — the record the single refusal pulled and dropped |

Both are correct. Only the first matches the naive reading.

The fourth column is the one that is easy to miss and expensive to be wrong
about. `ring_core::try_push_batch` pulls from the iterator *before* it can know
whether there is room, so every refusal consumes and drops the record it could
not place. The published count never shows it: two tests read the iterator back
afterwards instead, asserting `Some( 6 )` after a two-attempt run and `Some( 5 )`
after a one-attempt run.

The **6** is asserted. The **1** is not — the zero-moved test hands
`push_batch_within` an inline `&mut ( 0..4 )` and keeps no binding, so there is
no iterator left to read. This table carried three columns until those
assertions were read into it, and one of the two was sitting inside a test this
document already cited → PL43.

### Failure

**A retry loop written around the return value.** This looks reasonable and is
wrong:

```rust
// Wrong: assumes a budget of 9 means nine chances at the ring.
let published = push_batch_within( &mut producer, &mut records, Budget::new( 9 ) );
assert!( published > 0, "surely one of nine attempts got through" );
```

Nine attempts were authorised. One was made. The assert fires on a full ring the
first time saturation happens in the field, which is the same "works under light
load" shape the crate's non-parking rule is about — arrived at from the opposite direction.

**Sizing a batch by the budget.** The budget bounds attempts, and each attempt
publishes as much as `try_push_batch` will take. There is no relationship
between the budget and how many records move; `drain_up_to`'s `max` is the only
number on this surface that bounds a *count*.

### Mitigation

**The early exit is right anyway, so the fix is to read it rather than remove
it.** The reasoning is in
[`../algorithm/001`](../algorithm/001_bounded_retry.md): a retry does not make
this thread better at pushing, it buys elapsed time for another thread to drain.
An attempt that moved nothing is the case with no evidence a consumer is running
at all, so the budget would be spent on nothing.

It is a heuristic. A consumer *could* be about to drain, and in that case the
early exit gives up slightly too soon. The alternative — spending the full budget
on a ring nobody is reading — is worse in the case that actually happens on a
tick path, which is a producer outrunning a stalled consumer.

For a caller: treat the return value as "how many moved", never as "how many
attempts were spent".

```text
cargo nextest run -p ring_poll push_batch_within_stops_at_the_first_attempt_that_moves_nothing
cargo nextest run -p ring_poll push_batch_within_spends_a_second_attempt_after_a_productive_first
```

The two tests are deliberately a pair — one asserts each row of the first three
columns, because a single test of either row would leave the other reading as an
accident. The second of them also asserts the fourth column, three lines further
down, in a line this document did not read for as long as it described the two
of them only as a pair.

**Bounding a count, on this side, is the caller's own job.** This section used to
point at `drain_up_to`'s `max`, which is a real count bound and is on the
consuming side; there is no parameter anywhere on `push_batch_within` that bounds
how many records it may take. The publishing answer is to bound the iterator
before handing it over — `records.by_ref().take( n )` — which appears nowhere in
this crate's source or tests → PL44.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'batch tests in the suite:     %s\n' "$( command grep -oE 'fn push_batch_within_[a-z_]+' tests/poll_test.rs | sed 's/fn //' | tr '\n' ' ' )"
printf 'of those, reading it back:    %s\n' "$( awk '/^fn /{n=$0;sub(/^fn /,"",n);sub(/\(\)$/,"",n)} /records\.next\(\)/{print n}' tests/poll_test.rs | tr '\n' ' ' )"
printf 'the values they assert:       %s\n' "$( awk '/records\.next\(\),/{getline;sub(/^ +/,"");sub(/,$/,"");print}' tests/poll_test.rs | tr '\n' ' ' )"
printf 'the message beside the six:   %s\n' "$( awk '/fn push_batch_within_spends_a_second_attempt/{f=1} f&&/^\}$/{exit} f' tests/poll_test.rs | command grep -oE 'two attempts[^"]*' )"
printf 'the zero-moved test iterator: %s\n' "$( awk '/fn push_batch_within_stops_at_the_first_attempt/{f=1} f&&/^\}$/{exit} f' tests/poll_test.rs | command grep -oE '&mut \( [0-9]+\.\.[0-9]+ \)' )"
printf 'params of push_batch_within:  %s\n' "$( awk '/^pub fn push_batch_within/{f=1} f&&/^\)/{exit} f' src/lib.rs | command grep -oE '^  [a-z_]+ :' | tr -d ' :' | tr '\n' ' ' )"
printf 'a count bound among them:     %s\n' "$( awk '/^pub fn push_batch_within/{f=1} f&&/^\)/{exit} f' src/lib.rs | command grep -c 'max' || true )"
printf 'the count bound drain has:    %s\n' "$( awk '/^pub fn drain_up_to/{f=1} f&&/^\)/{exit} f' src/lib.rs | command grep -oE 'max : usize' )"
printf 'take( in src or tests:        %s\n' "$( command grep -rc 'take( ' src tests 2>/dev/null | command grep -v ':0$' | wc -l )"
printf 'what eats the record:         %s\n' "$( command grep -oE 'for record in records.by_ref\(\)' ../ring_core/src/lib.rs )"
printf 'and drops it here:            %s\n' "$( awk '/pub fn try_push_batch/{f=1} f&&/accepted$/{exit} f' ../ring_core/src/lib.rs | command grep -oE 'if self.try_push\( record \).is_err\(\)' )"
```

Live output:

```
batch tests in the suite:     push_batch_within_publishes_the_whole_iterator_when_it_fits push_batch_within_stops_at_the_first_attempt_that_moves_nothing push_batch_within_spends_a_second_attempt_after_a_productive_first push_batch_within_eats_one_record_per_attempt 
of those, reading it back:    push_batch_within_spends_a_second_attempt_after_a_productive_first push_batch_within_eats_one_record_per_attempt a_tick_records_what_a_refused_batch_destroyed a_tick_records_a_fully_refused_batch_as_lost_not_moved a_reset_tick_reports_no_progress_and_keeps_its_budget 
the values they assert:       Some( 6 ) Some( 5 ) } }  
the message beside the six:   two attempts, two records eaten — 4 by the first refusal, 5 by the second
the zero-moved test iterator: &mut ( 0..4 )
params of push_batch_within:  producer records budget 
a count bound among them:     0
the count bound drain has:    max : usize
take( in src or tests:        1
what eats the record:         for record in records.by_ref()
and drops it here:            if self.try_push( record ).is_err()
```

### Algorithms

| File | Relationship |
|------|--------------|
| [`../algorithm/001_bounded_retry.md`](../algorithm/001_bounded_retry.md) | The shape, and the one place the batch version departs from it |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_tick_path_surface.md`](../api/001_tick_path_surface.md) | Lists `push_batch_within` as bounded by `budget` "and stops early" — the qualifier this pitfall expands |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_a_budget_bounds_attempts_not_time.md`](../invariant/002_a_budget_bounds_attempts_not_time.md) | The budget bounds attempts; this pitfall is what happens when attempts are read as records |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`001_non_parking_is_not_bounded_latency.md`](001_non_parking_is_not_bounded_latency.md) | The other way a budget's meaning gets over-read |

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_budget_clamps_to_one.md`](../type/001_budget_clamps_to_one.md) | The parameter being misread |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `push_batch_within` — the zero-moved early exit, which the single-record `push_within` does not have |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `push_batch_within_stops_at_the_first_attempt_that_moves_nothing` and `push_batch_within_spends_a_second_attempt_after_a_productive_first` — one per row of the Trap table's first three columns |
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `push_batch_within_eats_one_record_per_attempt` and the second test above — the only two that read the iterator back, and so the only two that can see the fourth column at all |

### PL43 — the assertion proving the missing column was inside a test this document already cited

This file exists because a caller's naive model of `push_batch_within` is wrong,
and its Trap table is the correction: a budget of nine buys attempts, not
records, and the early exit means the count spent depends on what the first
attempt did.

That correction was accurate and incomplete. The table accounted for attempts
spent and records published, and said nothing about records *destroyed* — so a
caller who read it, saw *4 published*, and concluded eight of twelve remained in
their iterator was still wrong after reading the document written to stop them
being wrong. Six are gone, not four: the four published, plus one eaten by the
refusal that ended the first attempt and one by the refusal that ended the
second.

The part worth recording is where that number already was. Not in some uncited
test — in `push_batch_within_spends_a_second_attempt_after_a_productive_first`,
one of the two tests this document cites by name, three lines below the
assertion it did read, with a message that spells the loss out: *"two attempts,
two records eaten — 4 by the first refusal, 5 by the second"*. The document
described the two tests as *"a pair — one asserts each row of the table above"*,
and that framing is what stopped the reading. A test assigned a row is a test
whose job is known, and the rest of its body stops being looked at.

The second figure is worse off than the first. The zero-moved row's **1** is
asserted nowhere: `push_batch_within_stops_at_the_first_attempt_that_moves_nothing`
passes an inline `&mut ( 0..4 )` and keeps no binding, so no iterator survives
the call to be read. That figure follows from
`push_batch_within_eats_one_record_per_attempt`'s general rule rather than from
a test of that case, and the column is half-pinned — two of four batch tests
read the iterator back at all.

The failure mode is specific to documents of this kind. A pitfall document is
trusted *more* than the API doc it supplements, because a reader arrives at it
already knowing their model is suspect. An omission there does not read as an
omission — it reads as the complete list of what to watch for. Cross-referenced
with [`../lifecycle/002`](../lifecycle/002_where_a_record_can_end_up.md) PL31,
which records the same loss from the publishing side.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F "Records gone from the iterator" ring_poll/docs/pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md
```

Live output:

```
| First attempt | Budget 9 actually spends | Records published | Records gone from the iterator |
```

**Disposition:** applied — the Trap table above now carries a fourth column
naming what the iterator lost, populated with the two asserted figures (6 and
1) rather than stopping at attempts-spent and records-published. Now prints: `Records gone from the iterator`

### PL44 — the mitigation named a count bound that only exists on the other side

The Mitigation section's closing advice used to end *"and use `drain_up_to`'s
`max` when a count is what needs bounding."* Both halves are true and they are
about different operations.

`drain_up_to( consumer, out, max )` does take a count bound, and it is the only
one on this surface. `push_batch_within( producer, records, budget )` takes a
producer, an iterator and a budget — no count, no limit, nothing that bounds how
many records it may pull. The two are not alternatives: one consumes and one
publishes, and a caller who needs *publish at most n* cannot reach for the drain
limit at all.

The answer is `records.by_ref().take( n )` — bound the iterator before handing it
over, which costs nothing, composes with the early exit, and incidentally caps
the destruction PL43 describes, since a refusal can only eat a record the
iterator was willing to yield. That expression appears zero times in this crate's
source and tests.

So the pitfall pointed a caller with a publishing problem at a consuming tool,
and the one-line fix that does solve it is unwritten anywhere. The advice has
been corrected in place. The finding is that the mis-aim was easy to make and
hard to notice: `max` is genuinely the crate's count bound, it is genuinely on
this surface, and nothing about the sentence signals that it is unreachable from
where the reader is standing.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F "bound the iterator before handing it" ring_poll/docs/pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md
```

Live output:

```
The answer is `records.by_ref().take( n )` — bound the iterator before handing it
```

**Disposition:** applied — the Mitigation section's closing advice no longer
points a publishing-side caller at `drain_up_to`'s consuming-side `max`; it
now names the actual answer, bounding the iterator with
`records.by_ref().take( n )` before the call. Now prints: `bound the iterator before handing it`
