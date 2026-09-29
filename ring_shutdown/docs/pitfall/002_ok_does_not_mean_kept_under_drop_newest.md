# Pitfall: `Ok` Does Not Mean Kept Under Drop-Newest

### Scope

- **Purpose**: Record that `Guarded::try_push` returning `Ok` does not mean the record is in the ring, and that under `RingConfig`'s *default* policy it routinely does not.
- **Responsibility**: The trap's shape, why `Refusal::Full` is unreachable under the default, what mitigates it, and how the trap was found.
- **In Scope**: `Guarded::try_push`, `Guarded::try_push_batch`, `Guarded::is_blocked`, `OverflowPolicy::DropNewest`.
- **Out of Scope**: The close half of the same predicate (→ [`pitfall/001`](001_close_is_advisory_to_an_unguarded_producer.md)); `free_capacity`'s binding/advisory split (→ [`ring_core/docs/pitfall/001`](../../../ring_core/docs/pitfall/001_free_capacity_carries_two_contracts.md)).

### Trap

The guarded surface has two refusals and they behave nothing alike:

```rust
guarded.try_push( record )   // closed  → Err( Refusal::Closed( record ) )
guarded.try_push( record )   // full    → Ok( () ),  and the record is gone
```

The second is not a bug in the guard. `RingConfig::new` defaults to
`OverflowPolicy::DropNewest`, whose entire contract is to discard the arrival
and report success. `ring_core::Producer::try_push` honours it, and a wrapper
that refused instead would be silently applying a different policy than the one
configured.

The consequence is that **`Refusal::Full` is unreachable under the default
configuration.** A caller who matches on both arms and tests only with
`RingConfig::new( n )` exercises one of them, ever.

```rust
// Reads as complete. Under the default policy the second arm is dead code and
// the `Ok` branch silently counts dropped records as published.
match guarded.try_push( record )
{
  Ok( () ) => published += 1,
  Err( Refusal::Closed( r ) ) => stash( r ),
  Err( Refusal::Full( r ) ) => retry_later( r ),
}
```

### Failure

| Carried assumption | Failure under `DropNewest` | Visibility |
|---|---|---|
| `Ok` means the record is in the ring | It means the *policy was applied*; the record may have been discarded | **Silent** — no error, no count, the loop reports success |
| `is_blocked() == false` implies room, `== true` implies refusal | Neither. A blocked-by-occupancy push still returns `Ok` | Silent, and it looks like the ring is keeping up |
| Handling both `Refusal` arms handles both failures | One arm never fires | **Invisible** — the untaken branch looks tested because it compiles |

Row 1 is the expensive one, because the number it corrupts is a *success*
count. A publisher tallying `Ok`s reports a throughput it did not achieve, and
the discrepancy only shows up against a consumer-side count — which is exactly
the comparison a single-crate test does not make.

### Mitigation

1. **Choose `OverflowPolicy::Fail` when a lost record matters.** It is one call
   — `RingConfig::new( n ).with_overflow( OverflowPolicy::Fail )` — and it is
   what makes `Refusal::Full` reachable at all. Under `Fail`, `Ok` does mean
   kept.

2. **Read the policy, not the return value, to know what `Ok` means.**
   `RingConfig::overflow()` answers it. The two configurations give the same
   signature opposite meanings, which is the same hazard shape as
   `free_capacity`'s (→ [`ring_core/docs/pitfall/001`](../../../ring_core/docs/pitfall/001_free_capacity_carries_two_contracts.md))
   arriving one layer up.

3. **Count on the consumer side.** A drain count is the only number that
   distinguishes "published" from "reported published" under a lossy policy.
   The worked example already exists one crate away: `ring_testkit` — a
   *reverse* dependency of this crate, so citing it adds no new edge —
   computes exactly this as `Outcome::vanished()`
   (`accepted.saturating_sub( received.len() + in_ring_at_end )`) and
   tabulates both policies by example (→ SD44 below). `ring_stats`
   records the same figure for a production caller willing to
   add the dependency themselves; it is deliberately not one here (→
   [`integration/001`](../integration/001_family_dependency_seam.md)).

**What does not mitigate it: documenting `try_push`'s return.** The doc comment
says it, and the doc comment is not read at the call site where `Ok` is being
counted. What would mitigate it structurally is a return type that names the
outcome rather than merely whether the call succeeded — `ring_overflow` already
has one, `Resolution`, with a `lost_an_item()` method. Threading it up through
`ring_core::Producer::try_push` would fix this at the layer that owns the
policy. That is not this crate's call to make; it is recorded here because this
is where the trap surfaces.

### How This Was Found

A test written to exercise `Refusal::Full` — `a_full_guarded_producer_refuses_with_the_transient_arm` —
failed on `unwrap_err()` against an `Ok` value. The test was written against a
default-configured ring and the arm it was testing does not exist there.

The test was fixed by giving it a `Fail`-policy ring, and a second test
(`a_full_drop_newest_ring_reports_success_and_keeps_nothing`) was added to
cover what the default actually does — rather than leaving the default's
behaviour untested, which is how it went unnoticed in the first place.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
D=docs/pitfall/002_ok_does_not_mean_kept_under_drop_newest.md
printf 'methods this doc scopes:       %s\n' "$( awk '/^### Trap/{ exit } /In Scope/{ sub( /.*In Scope\*\*: /, "" ); print }' $D )"
printf 'mentions of try_push_batch:    %s\n' "$( awk '/^### Trap/{f=1} f&&/^### Regenerate/{exit} f' $D | command grep -c 'try_push_batch' || true )"
printf 'mentions of plain try_push:    %s\n' "$( awk '/^### Trap/{f=1} f&&/^### Regenerate/{exit} f' $D | command grep -c 'try_push(' || true )"
printf 'what mitigation 1 promises:    %s\n' "$( awk '/^### Regenerate/{ exit } { print }' $D | tr '\n' ' ' | sed 's/  */ /g' | command grep -o 'Under .Fail., .Ok. does mean kept' )"
printf 'how ring_core spells the batch: %s\n' "$( cd ..; awk '/pub fn try_push_batch/{f=1} f&&/is_err\(\)/{ sub( /^ */, "" ); print; exit }' ring_core/src/lib.rs )"
printf 'and what its doctest asserts:  %s\n' "$( cd ..; command grep -o 'record 3 was consumed by the refusal' ring_core/src/lib.rs )"
printf 'what DropNewest returns:       %s\n' "$( cd ..; awk '/Resolution::DroppedIncoming/{ sub( /^ */, "" ); print; exit }' ring_core/src/lib.rs )"
printf 'guarded batch calls in suite:  %s\n' "$( command grep -c 'guarded.try_push_batch' tests/shutdown_test.rs || true )"
printf 'of those, into a full ring:    %s\n' "$( command grep -c '^fn a_batch_into_a_full' tests/shutdown_test.rs || true )"
printf 'the policies those two cover:  %s\n' "$( command grep -ohE '^fn a_batch_into_a_full_(refusing|drop_newest)' tests/shutdown_test.rs | sed 's/^fn a_batch_into_a_full_//' | tr '\n' ' ' )"
printf 'slots the ring they use has:   %s\n' "$( awk '/fn a_closed_batch_push_consumes_nothing/{f=1} f&&/= ring\(/{ sub( /.*ring\( /, "" ); sub( / \).*/, "" ); print; exit }' tests/shutdown_test.rs )"
printf 'records they offer it:         %s\n' "$( awk '/fn a_closed_batch_push_consumes_nothing/{f=1} f&&/open_records = /{ print gsub( /,/, "," ) + 1; exit }' tests/shutdown_test.rs )"
printf 'what mitigation 3 names:       %s\n' "$( awk '/^### Regenerate/{ exit } { print }' $D | tr '\n' ' ' | sed 's/  */ /g' | command grep -o 'reverse. dependency of this crate, so citing it adds no new edge' )"
printf 'the crate that ships it today: %s\n' "$( cd ..; command grep -rl 'pub fn vanished' ring_*/src --include='*.rs' | cut -d/ -f1 )"
printf 'how it computes it:            %s\n' "$( cd ..; awk '/pub fn vanished/{f=1} f&&/saturating_sub/{ sub( /^ */, "" ); print; exit }' ring_testkit/src/lib.rs )"
printf 'what it says accepted means:   %s\n' "$( cd ..; command grep -o 'Not the same as how many it stored' ring_testkit/src/lib.rs )"
printf 'its two tabulated policies:    %s\n' "$( cd ..; command grep -ohE '(Fail|DropNewest): +accepted=[0-9]+ +refused_full=[0-9]+' ring_testkit/src/lib.rs | tr '\n' ' ' )"
printf 'testkit manifest names us:     %s\n' "$( cd ..; command grep -c 'ring_shutdown' ring_testkit/Cargo.toml || true )"
printf 'our manifest names testkit:    %s\n' "$( command grep -c 'ring_testkit' Cargo.toml || true )"
printf 'how ring_stats records drops:  %s\n' "$( cd ..; command grep -ohE 'pub fn [a-z_]*drop[a-z_]*' ring_stats/src/lib.rs | sort -u | tr '\n' ' ' )"
printf 'crates calling record_drop:    %s\n' "$( cd ..; command grep -rl 'record_drop(' ring_*/src ring_*/tests --include='*.rs' | cut -d/ -f1 | sort -u | command grep -v ring_stats | tr '\n' ' ' )"
```

Live output:

```
methods this doc scopes:       `Guarded::try_push`, `Guarded::try_push_batch`, `Guarded::is_blocked`, `OverflowPolicy::DropNewest`.
mentions of try_push_batch:    0
mentions of plain try_push:    3
what mitigation 1 promises:    Under `Fail`, `Ok` does mean kept
how ring_core spells the batch: if self.try_push( record ).is_err()
and what its doctest asserts:  record 3 was consumed by the refusal
what DropNewest returns:       Resolution::DroppedIncoming => Ok( () ),
guarded batch calls in suite:  5
of those, into a full ring:    2
the policies those two cover:  refusing drop_newest 
slots the ring they use has:   8
records they offer it:         3
what mitigation 3 names:       reverse* dependency of this crate, so citing it adds no new edge
the crate that ships it today: ring_testkit
how it computes it:            self.accepted.saturating_sub( self.received.len() + self.in_ring_at_end )
what it says accepted means:   Not the same as how many it stored
its two tabulated policies:    Fail:       accepted=4  refused_full=4 DropNewest: accepted=8  refused_full=0 
testkit manifest names us:     1
our manifest names testkit:    0
how ring_stats records drops:  pub fn dropped pub fn dropped_total pub fn record_drop 
crates calling record_drop:    ring_bench ring_overflow 
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_shutdown_surface.md](../api/001_shutdown_surface.md) | Guarantee 1, whose `Full` arm this trap makes conditional on configuration |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_refusal_carries_the_record.md](../type/002_refusal_carries_the_record.md) | The two-armed refusal, one of whose arms is unreachable by default |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_core/docs/pitfall/001`](../../../ring_core/docs/pitfall/001_free_capacity_carries_two_contracts.md) | The same one-signature-two-contracts shape, one layer down |
| [`ring_overflow`](../../../ring_overflow/readme.md) | `Resolution`, the outcome type mitigation's structural fix would thread upward |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `a_full_drop_newest_ring_reports_success_and_keeps_nothing` — the trap, asserted |
| `tests/shutdown_test.rs` | `a_full_guarded_producer_refuses_with_the_transient_arm` — mitigation 1, and the test that found it |

### SD43 — The Batch Push Is In Scope, Absent From Every Section, and Loses a Record Under the Policy This Document Prescribes

*Both behaviours below are now pinned by tests in this crate; see the
disposition at the end of this section for what changed and what did not.*

`Guarded::try_push_batch` is the second method in this document's In Scope list
and appears in none of its sections — zero mentions between the Trap heading and
the recipe, against three for `try_push`. It is not an oversight of degree. The
batch path fails in both of this document's directions, and worse in each.

It has no refusal at all. The signature is
`try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize`,
so there is no `Refusal::Full( record )` to recover a payload from and no arm to
match — the trap this document opens with (*two refusals that behave nothing
alike*) has no batch analogue, because the batch form has none.

**Under `Fail`, the batch destroys a record.** Mitigation 1 prescribes that
policy with the sentence *"Under `Fail`, `Ok` does mean kept"*, and for
`try_push` that is exactly right. `ring_core::Producer::try_push_batch` is a
loop over `if self.try_push( record ).is_err() { break; }` — `try_push` takes
the record **by value** and returns `Err( record )`, and `.is_err()` discards
the `Result` and the record inside it. `ring_core` knows and says so in its own
doctest: `assert_eq!( records.next(), Some( 4 ), "record 3 was consumed by the
refusal" )`. Record 3 is not in the ring, not in the iterator, and not in the
returned count. The policy this document offers as the fix is the only one under
which the batch path loses data.

**Under `DropNewest`, the returned count is not a count of records kept.**
`try_push` answers `Resolution::DroppedIncoming => Ok( () )` for a discarded
record, so the loop's `break` never fires: it drains the iterator to the end and
returns its length. That is this document's own thesis — `Ok` means the policy
was applied — promoted from a unit to a `usize` a caller will sum into a
throughput figure. Row 1 of the Failure table says the expensive case is the one
that corrupts a success *count*; the batch path is the one that hands the caller
the count directly.

Neither was exercised. The suite called `guarded.try_push_batch` twice, both in
`a_closed_batch_push_consumes_nothing`, offering three records to an eight-slot
ring — the open assertion passes on capacity, never on policy. No test in this
crate pushed a batch into a full ring under either policy, so both behaviours
were first observed by a caller.

They are now pinned, one test per policy, each asserting the loss rather than
asserting it away. Under `Fail` the refused record is neither in the ring nor in
the iterator, and the test says so in those words; under `DropNewest` the
returned count is three against one record actually kept, and the test asserts
both numbers so the gap between them is the assertion.

**The record loss itself is not fixed here, and deliberately.** It belongs to
`ring_core::Producer::try_push_batch` — the loop that moves the record into
`try_push` and discards the `Err` — so a repair means changing that signature
for every caller in the family, not this crate's forwarding wrapper. What this
crate can do is make the behaviour a fact its own suite states rather than a
claim its docs make, which is the difference between a caller discovering it and
a caller reading it.

**Disposition:** applied —
`a_batch_into_a_full_refusing_ring_destroys_the_record_that_was_refused` and
`a_batch_into_a_full_drop_newest_ring_counts_records_it_did_not_keep` in
`tests/shutdown_test.rs`, covering both policies against a full ring; the
underlying loss stays with `ring_core::Producer::try_push_batch`, whose fix is a
family-wide signature change rather than a wrapper edit.
Now prints: `of those, into a full ring:    2`

### SD44 — The Recommended Measurement Points at a Non-Dependency While the Built One Ships in the Crate That Depends on This

Mitigation 3 is right about the shape of the fix — a consumer-side count is the
only number that separates *published* from *reported published* — and it names
`ring_stats`, which *"records drops for exactly this reason, and it is
deliberately not a dependency here"*. `ring_stats` does record them, through
`record_drop`/`dropped`/`dropped_total`. What makes the mitigation unactionable
is who would have to call it: not this crate, which has no edge and argues it
should not have one (→ [`integration/001`](../integration/001_family_dependency_seam.md)),
but the caller, at their own push site, having first wired a crate this document
does not tell them to add.

Meanwhile the measurement already exists, one crate away, in the direction
nothing was looking. `ring_testkit` — the crate that depends on **this** one
(→ [`integration/002`](../integration/002_what_the_family_says_about_this_crate.md)) —
computes `Outcome::vanished()` as
`self.accepted.saturating_sub( self.received.len() + self.in_ring_at_end )`,
documents `accepted` with the words *"Not the same as how many it stored"*, and
tabulates this crate's trap by policy in its own module header:
`Fail: accepted=4 refused_full=4 … vanished=0` against
`DropNewest: accepted=8 refused_full=0 … vanished=4`. It derives all of it from
a loop over `Guarded::try_push` matching all three arms.

So the most complete statement of this pitfall in the whole workspace is written
in a crate this document never mentions, needs no new edge to cite, and could
not have been reached by looking down the dependency list — which is where both
mitigation 3 and [`integration/001`](../integration/001_family_dependency_seam.md)
were looking. SD20 measured the blind spot; this is what it cost. A reader
following mitigation 3 has to build something; a reader pointed at
`Outcome::vanished` has a worked example and two policies' numbers.

The narrower lesson for a pitfall document: **the mitigation section is the one
place a doc reaches outside its own crate, and reverse dependencies are where
mitigations actually accumulate** — they are the crates that hit the trap.

**Disposition:** applied — Mitigation 3 no longer points only at the
non-dependency `ring_stats`; it now leads with `ring_testkit::Outcome::vanished()`
as the actionable worked example (a reverse dependency, so citing it adds no
new edge), gives its formula and both tabulated policies inline, and keeps
`ring_stats` as the production-grade option a caller must wire in themselves.
The file's own `### Regenerate` block extracted a phrase from the old wording
("records drops for exactly this reason...") — its extraction command was
updated to match the new wording and the whole block was genuinely re-run.
Now prints: `what mitigation 3 names:       reverse* dependency of this crate, so citing it adds no new edge`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
D=docs/pitfall/002_ok_does_not_mean_kept_under_drop_newest.md
awk '/^### Mitigation/{f=1} f&&/^### How This Was Found/{exit} f' $D | tr '\n' ' ' | sed 's/  */ /g' | command grep -o 'ring_testkit. .*Outcome::vanished()'
awk '/^### Regenerate/{f=1} f&&/^### APIs/{exit} f' $D | command grep 'what mitigation 3 names:' | command grep -v '^printf'
```

Live output:

```
ring_testkit` — a *reverse* dependency of this crate, so citing it adds no new edge — computes exactly this as `Outcome::vanished()
what mitigation 3 names:       reverse* dependency of this crate, so citing it adds no new edge
```
