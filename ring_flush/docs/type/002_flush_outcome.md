# Type: Flush Outcome

### Scope

- **Purpose**: Define what a drive call reports, and fix the distinction the type exists for — "the policy did not fire" and "the policy fired and moved nothing" must not be the same value.
- **Responsibility**: Give the definition, the validation rules, and the failure modes the type is designed to make visible.
- **In Scope**: The outcome variants; what each guarantees; why the type is not `Result` and not `bool`.
- **Out of Scope**: The policy that produced the outcome (→ [Flush Policy](001_flush_policy.md)); the log that records it (→ [The Flush Log](../data_structure/002_the_flush_log.md)).

### Definition

```rust
pub enum FlushOutcome
{
  /// The policy was consulted and its trigger did not hold.
  NotTriggered,
  /// The trigger held; the buffer held no records.
  TriggeredEmpty,
  /// The trigger held; `count` records were moved into the ring.
  Flushed { count : usize },
  /// The trigger held; the ring could not accept the records.
  Rejected { staged : usize },
}
```

**The first two variants are the reason this type exists.** A `bool` return, or
a `usize` count, collapses `NotTriggered` and `TriggeredEmpty` into the same
observation — nothing moved. They mean opposite things:

| | `NotTriggered` | `TriggeredEmpty` |
|---|---|---|
| Policy fired? | No | Yes |
| Records waiting? | **Unknown** — may be many | None |
| Healthy? | Depends entirely on the policy | Always |
| If seen forever | **`OnBarrier` is never being announced** (→ [the pitfall](../pitfall/001_on_barrier_cannot_see_the_barrier.md)'s F1) | The system is idle |

**A consumer that only ever sees `NotTriggered` has the pitfall's silent
failure and can now detect it.** That is the whole design intent: the crate
cannot prevent a misconfigured `OnBarrier`, so it makes the misconfiguration
*observable* to anyone who looks.

**Why not `Result`.** Three of the four variants are ordinary, expected
outcomes; only `Rejected` is a failure. Wrapping all four in `Result` would
either misclassify `NotTriggered` as an error or bury the useful distinction
inside a success arm. `Rejected` is an outcome the caller must handle, not a
bug — the ring being full is the backpressure the design expects.

**Trait obligations:**

| Trait | Status | Why |
|-------|--------|-----|
| `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq` | **Required** | Asserted directly in the scripted test |
| `#[must_use]` | **Required** | An ignored outcome is exactly how the pitfall stays silent |
| `Default` | **Withheld** | No variant is a sensible default; `NotTriggered` would be the tempting choice and is a lie |

**`#[must_use]` is doing real work here rather than being hygiene.** The
type's entire value is that a caller can distinguish two states; a caller who
discards it gets no more than the `bool` this type exists to replace, and the
compiler is the only thing that will mention it.

### Validation

| # | Rule | Enforcement |
|---|------|-------------|
| M1 | `Flushed { count }` implies `count >= 1` | **Partially by construction, partially by convention** — the empty-buffer path returns `TriggeredEmpty` before touching the producer, but a mid-flush second producer (permitted by the API — → `FL48` below) can still produce `Flushed { count : 0 }` |
| M2 | `Flushed` implies the buffer is empty and writable afterwards | The seal/drain/reset sequence completing (→ [`algorithm/002`](../algorithm/002_sequencing_seal_drain_reset.md)) |
| M3 | `Rejected { staged }` implies the records are **still staged**, not lost | The sequence aborting before reset |
| M4 | Exactly one outcome per drive call | The driver returning once |
| M5 | The outcome matches the flush log's entry for the same call | **Nothing.** Two independent records of the same event |

**M3 is the guarantee that makes `Rejected` safe to retry** and it constrains
the sequencing rather than the type: reset must not run if drain did not
succeed, or the staged records are discarded while the caller is told they were
merely rejected. That ordering obligation belongs to
[`algorithm/002`](../algorithm/002_sequencing_seal_drain_reset.md) and is
recorded here because the type's contract is what depends on it.

**M5 is an unenforced consistency requirement between two mechanisms that
exist for the same purpose.** The outcome tells the caller what happened; the
log tells the test what happened. If they can disagree, the acceptance
criterion is checking a record that need not match reality. The cheap
resolution is that the log entry is written from the outcome rather than
alongside it — one source, two readers — and that is worth doing precisely
because nothing would otherwise catch a divergence.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_sequencing_seal_drain_reset.md](../algorithm/002_sequencing_seal_drain_reset.md) | M2 and M3 — the ordering the outcome's guarantees rest on |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_driver_surface.md](../api/002_the_driver_surface.md) | The surface returning this type |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_flush_log.md](../data_structure/002_the_flush_log.md) | M5 — the second record, and the argument for deriving it from this one |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_a_policy_fires_only_at_its_trigger.md](../invariant/001_a_policy_fires_only_at_its_trigger.md) | Its E4 — this type is what makes the invariant observable at runtime rather than only in tests |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_on_barrier_cannot_see_the_barrier.md](../pitfall/001_on_barrier_cannot_see_the_barrier.md) | F1 — the failure this type's first two variants exist to separate |

### Types

| File | Relationship |
|------|--------------|
| [001_flush_policy.md](001_flush_policy.md) | The input; this type is the output of consulting it |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | Row 176 — the recorded flush log this type must stay consistent with |
| [`ring_tls/docs/api/002_consolidator_read_surface.md`](../../../ring_tls/docs/api/002_consolidator_read_surface.md) | The primitives whose partial failure produces `Rejected` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | `an_empty_trigger_is_recorded_and_is_not_a_non_trigger` is that assertion: a trigger that fired against an empty buffer is `TriggeredEmpty` **and logged**, while a call that did not fire is `NotTriggered` **and not logged** — the two states a bare boolean would have merged. `a_call_that_did_not_fire_is_not_recorded` holds the other half |

### FL47 — M5 Recommends, as Future Work, the Thing the Source Already Does in the Same Words

The recommendation and the implementation, side by side:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the row and the recommendation --'
awk '/^### FL/{ exit } /^\| M5 \||cheap resolution|one source, two readers/{ printf "    %d: %s\n", NR, $0 }' \
  ring_flush/docs/type/002_flush_outcome.md | sed -E 's/^(.{0,124}).*/\1/'
echo '  -- what the crate does --'
awk -v n1="$( command grep -n -m1 -F '  /// Derive the log entry from the outcome, so the two cannot disagree.' ring_flush/src/lib.rs | cut -d: -f1 )" -v n2="$(( $( command grep -n -m1 -F '      log.entries.push( FlushEntry { policy : self.policy, cause, outcome } );' ring_flush/src/lib.rs | cut -d: -f1 ) + 1 ))" 'NR >= n1 && NR <= n2 { printf "    %d: %s\n", NR, $0 }' ring_flush/src/lib.rs
echo '  -- and how the third document describes the same mechanism --'
awk '/^### FL/{ exit } /^\| M4 \|/{ printf "    nfr/001: %s\n", $0 }' \
  ring_flush/docs/non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md \
  | sed -E 's/^(.{0,124}).*/\1/'
```

Live output:

```
  -- the row and the recommendation --
    69: | M5 | The outcome matches the flush log's entry for the same call | **Nothing.** Two independent records of the sam
    83: alongside it — one source, two readers — and that is worth doing precisely
  -- what the crate does --
    650:   /// Derive the log entry from the outcome, so the two cannot disagree.
    651:   fn record( &mut self, cause : FlushCause, outcome : FlushOutcome ) -> FlushOutcome
    652:   {
    653:     if let Some( log ) = self.log.as_mut()
    654:     {
    655:       log.entries.push( FlushEntry { policy : self.policy, cause, outcome } );
    656:     }
  -- and how the third document describes the same mechanism --
    nfr/001: | M4 | Log completeness | Assert `outcome` and log entry agree for every drive call in the scenario |
    nfr/001: | M4 | Agreement on every drive call — no tolerance |
```

M5's Enforcement cell says "**Nothing.** Two independent records of the same
event," and the paragraph beneath proposes the remedy: "the log entry is written
from the outcome rather than alongside it — one source, two readers — and that
is worth doing precisely because nothing would otherwise catch a divergence."

That is what `record` does. It takes the outcome as a parameter and constructs
the entry from it, under a comment reading "Derive the log entry from the outcome,
so the two cannot disagree." The recommendation was adopted, in the shape it was
made, and the row recommending it was never revisited.

**Three documents now describe this one mechanism in three incompatible ways.**
This instance says nothing enforces the agreement and proposes building it;
[`nfr/001`](../non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md)
lists it as a measurement, M4, with a threshold of "no tolerance" (→ its FL34);
the source makes disagreement unconstructible. A reader can pick up any one of
the three and form a coherent and wrong picture of what is guaranteed.

The failure mode is specific and worth naming: **a document that proposes a fix
carries no marker distinguishing "not done" from "done and not updated."** M5's
row and paragraph read identically in both worlds. What would have caught it is
the Enforcement column being the sort of claim a recipe can check — the same
discipline the findings below apply — rather than a sentence.

### FL48 — M1 Says `count == 0` Is `TriggeredEmpty` "by Definition", and the Source Comment Names the Path That Produces `Flushed { count : 0 }`

The rule, the code that would have to hold it, and the code's own note:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the rule and its stated enforcement --'
awk '/^### FL/{ exit } /^\| M1 \|/{ printf "    %s\n", $0 }' \
  ring_flush/docs/type/002_flush_outcome.md
echo '  -- where count comes from --'
awk -v n1="$(( $( command grep -n -m1 -F '    let staged = self.buffer.len();' ring_flush/src/lib.rs | cut -d: -f1 ) + 1 ))" -v n2="$(( $( command grep -n -m1 -F '    self.record( cause, FlushOutcome::Flushed { count } )' ring_flush/src/lib.rs | cut -d: -f1 ) + 1 ))" 'NR >= n1 && NR <= n2 && ( /staged == 0|free_capacity|let count|Flushed|Rejected|TriggeredEmpty|second producer|already unrecoverable/ ) \
     { printf "    %d: %s\n", NR, $0 }' ring_flush/src/lib.rs
echo '  -- and what try_push_batch returns when the first push fails --'
awk -v n1="$( command grep -n -m1 -F '  pub fn try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize' ring_core/src/lib.rs | cut -d: -f1 )" -v n2="$(( $( command grep -n -m1 -F '      accepted += 1;' ring_core/src/lib.rs | cut -d: -f1 ) + 4 ))" 'NR >= n1 && NR <= n2 { printf "    %d: %s\n", NR, $0 }' ring_core/src/lib.rs
```

Live output:

```
  -- the rule and its stated enforcement --
    | M1 | `Flushed { count }` implies `count >= 1` | **Partially by construction, partially by convention** — the empty-buffer path returns `TriggeredEmpty` before touching the producer, but a mid-flush second producer (permitted by the API — → `FL48` below) can still produce `Flushed { count : 0 }` |
  -- where count comes from --
    620:     if staged == 0
    622:       return self.record( cause, FlushOutcome::TriggeredEmpty );
    626:     // buffer is read. `free_capacity` is a snapshot, but on a ring this
    629:     if self.producer.free_capacity() < staged
    631:       return self.record( cause, FlushOutcome::Rejected { staged } );
    637:     let count = self.producer.try_push_batch( &mut self.buffer.drain() );
    642:     // this push — and at this point the shortfall is already unrecoverable:
    647:     self.record( cause, FlushOutcome::Flushed { count } )
  -- and what try_push_batch returns when the first push fails --
    441:   pub fn try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize
    442:   {
    443:     let mut accepted = 0;
    444: 
    445:     for record in records.by_ref()
    446:     {
    447:       if self.try_push( record ).is_err()
    448:       {
    449:         break;
    450:       }
    451:       accepted += 1;
    452:     }
    453: 
    454:     accepted
    455:   }
```

M1's Enforcement column claims construction: `count == 0` cannot reach `Flushed`
because it is `TriggeredEmpty` by definition. That holds for the *empty-buffer*
route — `run` returns `TriggeredEmpty` before touching the producer when nothing
is staged. It does not hold for the route `count` actually comes from.
`try_push_batch` returns however many records it placed, breaking on the first
refusal, so a refusal on the first record returns zero, and `run` reports
`Flushed { count : 0 }`.

**The source knows this and says so.** The comment above the report explains that
`count < staged` is deliberately not asserted, that it is reachable when a second
producer takes the space between the capacity check and the push, and that by
then the shortfall is unrecoverable. `count == 0` is the limiting case of exactly
that, and it produces a variant whose documented meaning — the trigger fired and
moved records — is false.

**The precondition is reachable through the public API.** The comment calls a
second producer on the same ring a contract violation, and
[`pattern/001`](../pattern/001_policy_as_a_value.md)'s FL37 measures how one is
obtained: `ring_core::Producer::try_clone` hands out another on both
multi-producer backends. So M1 rests on a convention, like the once-only property
of `drain_final` (→ [`decisions/002`](../decisions/002_the_final_drains_signature.md)),
and the table presents it as a structural guarantee.

The right correction is to the table rather than to the code. The source's
reasoning for not asserting is sound — a `debug_assert!` promises a guarantee
that evaporates in release, and no runtime check restores records already
consumed. What is wrong is an Enforcement column reading "Construction" for a
rule construction does not enforce.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
F=ring_flush/docs/type/002_flush_outcome.md
command grep -m2 'Partially by construction, partially by convention' "$F"
```

Live output:

```
| M1 | `Flushed { count }` implies `count >= 1` | **Partially by construction, partially by convention** — the empty-buffer path returns `TriggeredEmpty` before touching the producer, but a mid-flush second producer (permitted by the API — → `FL48` below) can still produce `Flushed { count : 0 }` |
    | M1 | `Flushed { count }` implies `count >= 1` | **Partially by construction, partially by convention** — the empty-buffer path returns `TriggeredEmpty` before touching the producer, but a mid-flush second producer (permitted by the API — → `FL48` below) can still produce `Flushed { count : 0 }` |
```

**Disposition:** applied — M1's Enforcement cell no longer claims plain
"Construction"; it names both paths, the genuinely-structural empty-buffer
check and the convention-only second-producer gap the source code's own
comment already concedes, and cross-references the `FL37` sibling finding
(`pattern/001_policy_as_a_value.md`) that establishes the second producer is
reachable through the public API rather than merely hypothetical. Declined
the code-level fix (asserting `count < staged` or otherwise closing the race)
— the finding's own reasoning says that's wrong: a `debug_assert!` evaporates
in release and no runtime check can un-consume records already taken by a
racing producer, so the correction belongs in the table, not the code, per
the finding's own conclusion. Now prints: `Partially by construction`
