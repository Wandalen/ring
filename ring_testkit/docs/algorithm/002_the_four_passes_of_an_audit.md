# Algorithm: The Four Passes Of An Audit

### Scope

- **Purpose**: Describe how a run is checked — two arithmetic tests over the counters, then two linear scans over the delivered records — and what each pass can and cannot see.
- **Responsibility**: The order the checks run in, why that order is visible to a caller, and the model each scan silently assumes.
- **In Scope**: `Outcome::audit`, `audit_delivery_order`, `audit_received`.
- **Out of Scope**: What the checked properties *mean* (→ [`invariant/001`](../invariant/001_every_minted_record_is_somewhere.md)); the values reported (→ [`type/001`](../type/001_outcome_and_anomaly.md)).

### Abstract

Checking a run is four passes in a fixed order, not one predicate. The first two
are arithmetic on the counters and answer *is every record somewhere* and *did
more leave the ring than ever entered it*. The third and fourth are linear scans
of the delivered list and answer *could these records have been produced* and
*did they arrive in order*. Only the first two have access to the counters; only
the last two are reachable from a concurrent test, which is why that pair also
exists as a separate public function.

The order matters to a caller because the result is one `Anomaly`, not a set. A
run that violates two properties reports whichever check runs first, and the
checks are not ordered by where the violation sits in the data.

### Algorithm

**Pass 1 — placement (`Outcome::audit`).** Sum the five terminal counts and
compare against the mint:

```text
placed = accepted + refused_full + refused_closed + refused_staging + staged_at_end
if placed != minted  ->  Anomaly::Unaccounted { minted, placed }
```

Five addends and one comparison, constant time. It reads no records at all. A
record that was accepted and then destroyed inside the ring is counted in
`accepted` and passes this pass, which is
[`decisions/readme.md`](../decisions/readme.md) Closed 4 and is deliberate.

**Pass 2 — delivery ceiling (`Outcome::audit`).** Add what left the ring to what
is still sitting in it, and compare against what the ring ever took:

```text
out = received.len() + in_ring_at_end
if out > accepted  ->  Anomaly::Overdelivered { accepted, out }
```

Also constant time, also counters only. Pass 1 cannot see this failure: a run
can account for every minted record and still report more records out of the
ring than ever entered it, and that is the one state where `vanished()` answers
`0` for a reason that is not health
(→ [`../data_structure/002`](../data_structure/002_nine_counters_and_a_number_that_is_two_things.md)
TK11, which is why this variant exists at all).

**Pass 3 — provenance (`audit_delivery_order`, first loop).** Walk the whole
delivered list; any `value >= minted` is a record no producer in this run could
have created:

```text
for value in received      ->  Unminted { value, minted } on the first that is >= minted
```

**Pass 4 — ordering (`audit_delivery_order`, second loop).** Walk adjacent
pairs; any pair whose two records do not advance through `published` is a
violation:

```text
for ( previous, then ) in received.windows( 2 )
  ->  OutOfOrder { previous, then } unless both appear in published
      and then's position there is the later one
```

Judging by position in `published` rather than by raw mint value is what lets a
script interleave a `Push` with a pending `Stage`/`Flush` and still have a
correct delivery pass. The bare-list forms have no `published` to consult:
`audit_received` compares the raw values instead (`then <= previous`), which is
why its own contract is a single-producer one, and `audit_received_unordered`
sorts first and then looks only for equality. Equality is a violation on purpose
in every form — a duplicate is the `then == previous` case, so one arm reports
both duplicates and reversals rather than two.

**Each pass runs to completion before the next begins.** Pass 3 does not stop at
the first record; it stops at the first *offending* record, having already passed
over every earlier one without checking its order. So the position of a violation
in the list does not determine which one is reported — the pass number does.

### Complexity

| Pass | Reads | Cost |
|---|---|---|
| 1 | six counters | constant |
| 2 | two counters and a length | constant |
| 3 | every delivered record | linear |
| 4 | every adjacent pair, and `published` for each | quadratic |

Four passes rather than one fused loop, at four times the traversals and no
extra allocation — and pass 4's lookup into `published` makes it quadratic in
the delivered count rather than linear. For a fixture checking a few thousand
records once at the end of a run that is not a cost worth fusing away, and the
separation is what lets passes 3 and 4 be called without an `Outcome` in hand,
against raw mint values, as `audit_received`.

### Termination

Every pass is a bounded `for` over a finite slice with no continuation state,
including pass 4's inner `position` scan of `published`; the early returns only
shorten them. `audit` cannot loop.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'counters summed in pass 1:      %s\n' "$( awk '/let placed =/' src/lib.rs | command grep -o 'self\.[a-z_]*' | wc -l )"
printf 'loops inside audit_received:    %s\n' "$( awk '/pub fn audit_received/{f=1} f && /^  for /{n++} f && /^}$/{exit} END{print n}' src/lib.rs )"
printf 'anomaly arms reachable from it: %s\n' "$( awk '/pub fn audit_received/{f=1} f && /Err\( Anomaly::/{ print }' src/lib.rs | command grep -o 'Anomaly::[A-Za-z]*' | sort -u | tr '\n' ' ' )"
printf 'ordering comparison used:       %s\n' "$( awk '/pub fn audit_received/{f=1} f && /if then/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'declared bound of the model:    %s\n' "$( command grep -m1 -o 'One producer, one consumer' tests/exhaustive_test.rs )"
printf 'where that bound is now stated: %s\n' "$( command grep -m1 -o '# Single-producer' src/lib.rs )"
printf 'audit forms taking a bare list: %s\n' "$( command grep -c '^pub fn audit_received' src/lib.rs )"
```

Live output:

```
counters summed in pass 1:      5
loops inside audit_received:    2
anomaly arms reachable from it: Anomaly::OutOfOrder Anomaly::Unminted 
ordering comparison used:       if then <= previous
declared bound of the model:    One producer, one consumer
where that bound is now stated: # Single-producer
audit forms taking a bare list: 2
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_script_surface.md](../api/001_the_script_surface.md) | The signatures of both checked entry points |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_minted_record_is_somewhere.md](../invariant/001_every_minted_record_is_somewhere.md) | The law pass 1 enforces |
| [../invariant/002_the_shape_a_delivered_list_must_have.md](../invariant/002_the_shape_a_delivered_list_must_have.md) | The two properties passes 3 and 4 enforce |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_outcome_and_anomaly.md](../type/001_outcome_and_anomaly.md) | The four variants these passes return |

### Algorithms

| File | Relationship |
|------|--------------|
| [001_from_a_step_to_an_outcome.md](001_from_a_step_to_an_outcome.md) | The run that produces the counters pass 1 reads |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `Outcome::audit`, `audit_delivery_order`, `audit_received` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | `a_duplicate_and_a_reversal_are_one_anomaly`, `an_outcome_that_adds_up_is_still_checked_for_its_records` |

### TK3 — `audit_received`'s ascent check assumes one producer

The ordering pass rejects any adjacent pair that does not **strictly ascend**,
and that is only a property of a delivered list when exactly one producer minted
the records and delivered them in mint order. Two producers on one ring — which
`ring_mpsc` is built for, and which `ring_core::Producer::try_clone` hands out —
legitimately interleave, so a correct run delivers `[ 0, 3, 1, 4 ]` and this
function calls it `OutOfOrder { previous : 3, then : 1 }`.

The assumption was invisible in the signature, which takes a `&[ u32 ]` and a
count, and it was contradicted by the function's own doc comment, which says it
is separate from `Outcome::audit` precisely because *"under `loom::model` there
is no single script and no accounting, only the records a consumer thread saw"* —
the concurrent case is the one it is documented as existing for, and the
single-producer case is the only one it is correct for.

Nothing tripped it: `tests/exhaustive_test.rs` declares its bound as
*"One producer, one consumer"*, so every list this function had ever seen was
minted by one thread. The hazard was a consumer widening that model and reading a
false anomaly as a real one.

**Disposition:** applied — as a second form and a stated bound, not as a change
to the existing one. `audit_received` keeps its ascent check unchanged, because a
single-producer model does guarantee ascent and a test that stops checking it
stops catching reordering; what it gained is a `# Single-producer` doc section
saying so, and a pointer to `audit_received_unordered`, which runs the same
`Unminted` pass and replaces the ascent pass with a sort-then-adjacent-equality
pass — keeping both failures a ring can actually produce (a record no producer
minted, a record delivered twice) and dropping the one only a single producer
guarantees. `interleaved_producers_pass_only_the_unordered_audit` pins the
divergence: `[ 0, 100, 1, 101, 2 ]` passes the unordered form and the ordered form
reports `OutOfOrder { previous : 100, then : 1 }`. What this does not buy: the
signature is still `&[ u32 ]` and a count in both forms, so choosing the wrong one
is still a silent choice — the model's producer count is not a value either
function can see, and nothing checks that a caller picked the form matching its
own model. Now prints: `audit forms taking a bare list: 2`

### TK4 — "the first that does not" is first by check, not first by position

`Outcome::audit`'s doc summary reads *"Check every property a run should hold,
and name the first that does not."* A reader takes "first" to mean the earliest
violation in the run. It means the earliest violation in the **pass order**, and
the two disagree whenever a list holds more than one kind of problem.

`audit_received( &[ 0, 2, 1, 9 ], 3 )` returns
`Unminted { value : 9, minted : 3 }`. The list holds two violations: a descent
from `2` to `1` at index 1, and a value no producer minted at index 3. The
provenance scan traverses the whole slice before the ordering scan runs at all,
so the anomaly reported is the one that is **last** in the list, and the one at
index 1 is never reached.

`a_duplicate_and_a_reversal_are_one_anomaly` fixes the "one, not a set" half of
this in a test. The half that is not tested, and not stated anywhere, is which
one that single anomaly will be.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
command grep -F 'first in the fixed pass order the checks run in, not first by position' src/lib.rs
```

Live output:

```
    /// first in the fixed pass order the checks run in, not first by position in
```

**Disposition:** applied — `Outcome::audit`'s doc summary now states that
"first" means first in the fixed pass order, not first by position in the
record list, resolving the ambiguity `audit_received( &[ 0, 2, 1, 9 ], 3 )`
demonstrates.
Now prints: `first in the fixed pass order the checks run in, not first by position`
