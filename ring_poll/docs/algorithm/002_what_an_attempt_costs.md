# Algorithm: What An Attempt Costs

### Scope

- **Purpose**: Account for what one attempt of `push_batch_within` spends, beyond the ring operation itself — a record pulled from the caller's iterator and destroyed — and how that cost scales with the budget.
- **Responsibility**: The mechanism inside `ring_core::try_push_batch` that causes it, where it is documented, where it is not, and what the return value does and does not show.
- **In Scope**: Iterator consumption across retries; the accounting a caller can and cannot perform afterwards.
- **Out of Scope**: The retry loop's shape and termination (→ [`001`](001_bounded_retry.md)); the misreading of budget-as-batch-size (→ [`../pitfall/002`](../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md)).

### Abstract

`push_batch_within` retries by calling `Producer::try_push_batch` again. That call
pulls from the iterator before it can know whether the ring has room, so every
refused attempt consumes one record and drops it. A budget of N against a full
ring therefore destroys up to N records, and the function's return value — the
count published — is the same whether one attempt ran or four.

### The mechanism, one crate down

`ring_core::try_push_batch` is a `for` loop over the iterator:

```
for record in records.by_ref()
  if try_push( record ) is Err, break
  accepted ← accepted + 1
return accepted
```

`try_push` takes the record **by value** and hands it back inside `Err`. The
`is_err()` test discards that `Err`, and with it the record. The iterator has
already advanced past it.

`ring_core` documents this and pins it with a doctest — *"record 3 was consumed by
the refusal"* — so at that layer the behaviour is intended and visible.

### What the retry multiplies

Every row below uses `refusing_ring`, whose overflow policy is `Fail`, and an
iterator of `0..12`:

| Budget | Slots free | Published | Destroyed | Iterator left at | Asserted by |
|---|---|---|---|---|---|
| `once()` | 4 | 4 | 1 — record `4` | `5` | `..._eats_one_record_per_attempt` |
| `new( 3 )` | 4 | 4 | 2 — records `4`, `5` | `6` | `..._spends_a_second_attempt_...` |
| `new( 9 )` | 0 | 0 | 1 — record `0` | *unasserted* | `..._stops_at_the_first_attempt_...` |

The third row is the early exit at work: an attempt that moves nothing ends the
loop, so a hopeless budget costs one record rather than nine. It is also the one
row no test pins — that test asserts the return value is `0` and never looks at
the iterator, so the cheapest case is the unmeasured one.

The cost accumulates only while attempts keep succeeding partially, which is
exactly the case a larger budget is chosen for → PL3.

### Where the caller can see it

Not in the return value, which counts successes only. Not on the producer, which
has no record of what it refused. At the free-function layer the only place the
loss is observable is the caller's own iterator position, and only if the caller
kept a handle to it. The wrapper layer added a second place — see PL4:

| Observation site | Shows the loss? |
|---|---|
| `push_batch_within`'s return | no — published count only |
| `consumer.len()` | no — identical for one attempt or two |
| `Tick::progress()` | no — sums what moved, never what vanished → PL4 |
| `Tick::lost()` | **yes**, for the wrapper layer — the count `progress()` leaves out |
| `records.next()` after the call | **yes** — the gap in the sequence is the loss |

This is what `push_batch_within_eats_one_record_per_attempt` and
`push_batch_within_spends_a_second_attempt_after_a_productive_first` assert
together, against the same ring and the same iterator: one attempt leaves
`records.next()` at `Some( 5 )`, two attempts leave it at `Some( 6 )`, and both
return `4`. The difference between the two runs is invisible everywhere except
that one call.

### Evidence

| # | Claim | Test |
|---|---|---|
| A1 | One attempt destroys one record — `next()` is `Some( 5 )` | `push_batch_within_eats_one_record_per_attempt` |
| A2 | A second productive attempt destroys a second — `Some( 6 )` | `push_batch_within_spends_a_second_attempt_after_a_productive_first` |
| A3 | An attempt that moves nothing ends the loop | `push_batch_within_stops_at_the_first_attempt_that_moves_nothing` |
| A4 | The published count is unchanged by the loss | A1 and A2 — both assert `4` |
| A5 | A batch that fits costs nothing | `push_batch_within_publishes_the_whole_iterator_when_it_fits` |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'the discarding line, in ring_core: %s\n' "$( awk '/pub fn try_push_batch/{f=1} f && /^  }$/{exit} f' ring_core/src/lib.rs | command grep -oE 'if self.try_push\( record \).is_err\(\)' )"
printf 'ring_core doc names the loss:      %s\n' "$( command grep -c 'consumed by the refusal' ring_core/src/lib.rs || true )"
printf 'ring_poll doc prose lines:         %s\n' "$( awk '/^\/\/\/ Publish from .records/{f=1} f && /^\/\/\/ ```/{exit} f' ring_poll/src/lib.rs | wc -l )"
printf 'of those naming the loss:          %s\n' "$( awk '/^\/\/\/ Publish from .records/{f=1} f && /^\/\/\/ ```/{exit} f' ring_poll/src/lib.rs | command grep -ciE 'consume|discard|drop|destroy|eaten|lost' || true )"
printf 'what ring_poll doc does promise:   %s\n' "$( awk '/^\/\/\/ Publish from .records/{f=1} f && /^pub fn push_batch_within/{exit} f' ring_poll/src/lib.rs | command grep -oE 'Returns how many records were published' )"
printf 'slots in its doctest ring:         %s\n' "$( awk '/^\/\/\/ Publish from .records/{f=1} f && /^pub fn push_batch_within/{exit} f' ring_poll/src/lib.rs | command grep -oE 'RingConfig::new\( [0-9]+ \)' )"
printf 'records that doctest pushes:       %s\n' "$( awk '/^\/\/\/ Publish from .records/{f=1} f && /^pub fn push_batch_within/{exit} f' ring_poll/src/lib.rs | command grep -oE '0\.\.[0-9]+' )"
printf 'so the doctest can lose:           %s records\n' "$( slots=$( awk '/^\/\/\/ Publish from .records/{f=1} f && /^pub fn push_batch_within/{exit} f' ring_poll/src/lib.rs | command grep -oE 'RingConfig::new\( [0-9]+ \)' | command grep -oE '[0-9]+' ); recs=$( awk '/^\/\/\/ Publish from .records/{f=1} f && /^pub fn push_batch_within/{exit} f' ring_poll/src/lib.rs | command grep -oE '0\.\.[0-9]+' | command grep -oE '[0-9]+$' ); [ "$recs" -le "$slots" ] && echo 0 || expr "$recs" - "$slots" )"
printf 'tests pinning the loss:            %s\n' "$( command grep -c 'records.next()' ring_poll/tests/poll_test.rs || true )"
printf 'Tick fields in total:              %s\n' "$( awk '/^pub struct Tick/{f=1} f && /^}$/{exit} f' ring_poll/src/lib.rs | command grep -cE '^  [a-z_]+ :' || true )"
printf 'and their names:                   %s\n' "$( awk '/^pub struct Tick/{f=1} f && /^}$/{exit} f' ring_poll/src/lib.rs | command grep -oE '^  [a-z_]+ :' | tr -d ' :' | tr '\n' ' ' )"
printf 'of those naming a loss:            %s\n' "$( awk '/^pub struct Tick/{f=1} f && /^}$/{exit} f' ring_poll/src/lib.rs | command grep -cE '^  (lost|dropped|destroyed|eaten) :' || true )"
printf 'sites adding to moved:             %s\n' "$( command grep -c 'self.moved +=' ring_poll/src/lib.rs || true )"
printf 'iterator positions asserted:       %s\n' "$( command grep -A2 'records.next()' ring_poll/tests/poll_test.rs | command grep -oE 'Some\( [0-9] \)' | sort | tr '\n' ' ' )"
printf 'longest doc comment in the tests:  %s\n' "$( awk '/^\/\/\//{ n++; next } /^fn /{ if ( n > m ) { m = n; f = $2 } n = 0; next } !/^#\[/{ n = 0 } END{ sub( /\(\)$/, "", f ); print m " lines, on " f }' ring_poll/tests/poll_test.rs )"
printf 'push_batch tests in the suite:     %s\n' "$( command grep -c '^fn push_batch_within_' ring_poll/tests/poll_test.rs || true )"
```

Live output:

```
the discarding line, in ring_core: if self.try_push( record ).is_err()
ring_core doc names the loss:      1
ring_poll doc prose lines:         28
of those naming the loss:          4
what ring_poll doc does promise:   Returns how many records were published
slots in its doctest ring:         RingConfig::new( 8 )
records that doctest pushes:       0..5
so the doctest can lose:           0 records
tests pinning the loss:            5
Tick fields in total:              3
and their names:                   budget moved lost 
of those naming a loss:            1
sites adding to moved:             4
iterator positions asserted:       Some( 2 ) Some( 4 ) Some( 5 ) Some( 5 ) Some( 6 ) 
longest doc comment in the tests:  31 lines, on then_saturates_instead_of_reporting_false_no_progress
push_batch tests in the suite:     4
```

### Algorithms

| File | Relationship |
|------|--------------|
| [001_bounded_retry.md](001_bounded_retry.md) | The loop this cost is paid inside, and its early exit |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md`](../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md) | The caller-facing misreading this cost sits underneath |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_tick_path_surface.md`](../api/001_tick_path_surface.md) | `push_batch_within`'s signature, and what it reports |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `push_batch_within` and its doc comment |
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | `try_push_batch` — the `for` loop that discards the refused record |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | A1–A4, all four in the `push_batch_within` block |

### PL3 — the wrapper inherits a documented data loss and does not redocument it

`ring_core::try_push_batch` destroys the record it could not place, says so, and
pins it with a doctest that asserts the gap in the iterator.
`ring_poll::push_batch_within` calls it in a loop and its own doc comment says
none of that. What it says is *"Returns how many records were published"*, which
is true and is the whole accounting offered.

The doctest under that sentence pushes five records into an eight-slot ring, so
it never refuses and never loses anything — a reader who checks the example
learns the good case only.

The loss also gets worse in exactly the situation the wrapper adds value for. A
caller reaches for a budget above one when the ring is under pressure and a
consumer is active; that is precisely the pattern of partial acceptance where
each attempt places some records and destroys one. `ring_core`'s single refusal
becomes N refusals, and the crate that multiplied it is the one that does not
mention it.

The knowledge does exist in this crate. `push_batch_within_eats_one_record_per_attempt`
carries the longest doc comment in the test file — it names the mechanism, cites
`ring_core`'s doctest as pinning it, and states outright that *"What nothing
asserted is the consequence for a `budget`: retrying is not free, and a budget of
N against a full ring destroys N records rather than one."*

That paragraph is the missing doc comment, and it is in `tests/`. A consumer
reading `docs.rs` for `push_batch_within` sees the promise and not the price;
the price is written down where only a maintainer of this crate will find it.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -F "A refused attempt consumes and drops that record" ring_poll/src/lib.rs
```

Live output:

```
/// the ring has room. A refused attempt consumes and drops that record, so a
```

**Disposition:** applied — `push_batch_within`'s rustdoc in `src/lib.rs` now
states the retry-driven data loss directly, in the same paragraph as the
early-stop behaviour, rather than leaving it only in the test file's doc
comment. Now prints: `A refused attempt consumes and drops that record`

### PL4 — a tick reports progress it made and never the records it consumed

`Tick` had two fields, `budget` and `moved`. `push_batch` added the published
count to `moved` and returned it; nothing anywhere added to a second counter, and
there was no second counter to add to.

So a tick that spent three attempts against a saturated ring, published four
records and destroyed three, reported `Progress::Made( 4 )`. The two numbers a
scheduler would want — what got through, and what the attempt cost — were
collapsed into the first one.

This matters more for `Tick` than for the free function because `Tick` is
explicitly the accounting layer: its own doc says every method *"delegates to the
free function of the same shape and adds only the accounting"*. The accounting it
added was one-sided. A free-function caller at least still holds the iterator and
can measure the gap; a `Tick` caller has handed the iterator in and read back a
`Progress` from which the loss was unrecoverable.

The obstacle looked structural. `push_batch_within` returns one number, so a
second counter seemed to require either changing that signature — which the free
layer's other three callers do not want — or having the tick inspect the
iterator, which it cannot do generically because `T` is opaque to it.

Neither is actually necessary. The tick does not need to see the records; it
needs to know how many left the iterator. Wrapping the caller's iterator in
`.inspect( | _ | offered += 1 )` for the duration of the call counts them without
touching `T` and without changing what `push_batch_within` returns. The
difference between what left and what arrived is the loss, and it is exact.

**Disposition:** applied — `Tick` gained a third field, `lost`, and a
`pub const fn lost( &self ) -> usize` accessor. `Tick::push_batch` in
`src/lib.rs` now counts the records leaving the caller's iterator by wrapping it
in `by_ref().inspect(…)` before handing it to `push_batch_within`, then records
`offered - moved` as destroyed, guarded by a `debug_assert!` that the batch
helper never publishes more than it was handed. `push` and `recv` deliberately do
not touch `lost` — a refused single push hands the record back to the caller, so
nothing was destroyed — and `only_a_batch_can_lose_a_record` in
`tests/poll_test.rs` pins that asymmetry, alongside
`a_tick_records_what_a_refused_batch_destroyed` for the counting itself. The
three fields now read: Now prints: `budget moved lost`
