# Pattern: Commit Nothing Until Every Check Has Passed

### Scope

- **Purpose**: Record the write-deferral pattern shared by `Watch::new` and `Watch::observe`, and what it buys.
- **Responsibility**: Where each mutation sits relative to each fallible step, and the failure mode the ordering prevents.
- **In Scope**: `new`'s check-before-construct; `observe`'s three deferred writes; the guarantee they jointly produce.
- **Out of Scope**: `Watch`'s layout (→ [`data_structure/001`](../data_structure/001_a_watch_is_three_scalars_and_no_identity.md)); its states (→ [`lifecycle/001`](../lifecycle/001_from_one_observation_to_a_sequence.md)).

### Abstract

Both of `Watch`'s fallible verbs place every write after every check. `new`
validates the pair and *then* constructs; `observe` runs three comparisons and
only then assigns its three fields.

Stated that way it sounds like ordinary hygiene. It is not — it is the difference
between an instrument that reports a permanent fault on every observation and one
that reports it once and then treats it as the new normal. **The natural
implementation gets this wrong**, and the reason it is easy to get wrong is that
the wrong version passes any test that checks a fault is detected.

### Pattern

**Name:** commit-nothing-until-checked.

**Shape:** a fallible `&mut self` method whose field assignments are placed after
its last `return Err` / `?`, so a failed call leaves the receiver exactly as it
was.

| Site | Fallible steps | Writes | Order |
|---|---|---|---|
| `Watch::new` | `check_seqs( ... )?` | Constructs `Self` | Check, then construct — a `Watch` is never born holding a state its own checker rejects |
| `Watch::observe` | D3 producer, D3 consumer, `check_seqs( ... )?` | `self.producer`, `self.consumer` | All three checks, then both writes |
| `check`, `check_ends` | — | none | Not applicable; both are `&`-only and write nothing at all |

**The receiver-state guarantee this produces** is `api/001`'s A3: a `Watch` whose
observation failed will fail the same way on the next observation of the same
state, because its baseline is unchanged. A caller polling a broken ring gets a
violation every time, not once.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
S=ring_debug/src/lib.rs
T=ring_debug/tests/debug_test.rs
echo '-- observe: every fallible step, then every write --'
command grep -m1 -A35 -F '  pub fn observe( &mut self, pair : &CursorPair ) -> Result< (), Violation >' $S \
  | command grep -E 'return Err|check_seqs|self\.(producer|consumer) =|Ok\( \(\) \)' | sed 's/^ */   /'
echo '-- new: the check precedes the construction --'
command grep -m1 -A7 -F '  pub fn new( pair : &CursorPair ) -> Result< Self, Violation >' $S \
  | command grep -E 'check_seqs|Ok\( Self' | sed 's/^ */   /'
echo '-- and what the crate writes down about the reach of that check --'
printf '   rustdoc sections recording what new does not guarantee: %s\n' \
  "$( command grep -c '# What this does not guarantee' $S )"
echo '-- the one test that separates the two implementations --'
printf '   a_failed_observation_leaves_the_baseline_alone:      %s\n' \
  "$( command grep -c 'fn a_failed_observation_leaves_the_baseline_alone' $T )"
printf '   tests in the suite:                                 %s\n' \
  "$( command grep -c '#\[ test \]' $T )"
echo '-- every last() assertion, and the test it sits in --'
awk '/^fn [a-z_]+\(\)/{f=$2} /\.last\(\)/{print "   "f}' $T
echo '   (the ordinary-run one asserts after a SUCCEEDING observe, so it passes either way;'
echo '    only the failed-observation test distinguishes the two implementations)'
```

Live output:

```
-- observe: every fallible step, then every write --
   return Err
   return Err
   check_seqs( producer, consumer, self.capacity )?;
   self.producer = producer;
   self.consumer = consumer;
   Ok( () )
-- new: the check precedes the construction --
   check_seqs( producer, consumer, capacity )?;
   Ok( Self { producer, consumer, capacity } )
-- and what the crate writes down about the reach of that check --
   rustdoc sections recording what new does not guarantee: 1
-- the one test that separates the two implementations --
   a_failed_observation_leaves_the_baseline_alone:      1
   tests in the suite:                                 28
-- every last() assertion, and the test it sits in --
   a_failed_observation_leaves_the_baseline_alone()
   a_failed_observation_leaves_the_baseline_alone()
   a_watch_that_faulted_reports_ok_once_the_ring_recovers()
   a_watch_that_faulted_reports_ok_once_the_ring_recovers()
   a_watch_follows_an_ordinary_run_quietly()
   a_cloned_watch_forks_the_baseline()
   a_cloned_watch_forks_the_baseline()
   (the ordinary-run one asserts after a SUCCEEDING observe, so it passes either way;
    only the failed-observation test distinguishes the two implementations)
```

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_one_observation_to_a_sequence.md](../lifecycle/001_from_one_observation_to_a_sequence.md) | The failed-observation self-loop this pattern creates |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_a_watch_is_three_scalars_and_no_identity.md](../data_structure/001_a_watch_is_three_scalars_and_no_identity.md) | The three fields being deferred, and DB5 which DB12 composes with |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | Guarantee A3 and precondition B3, which this pattern implements and bounds |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `Watch::new` and `Watch::observe` |

### Tests

| Test | Relationship |
|------|--------------|
| `a_failed_observation_leaves_the_baseline_alone` | DB11 — the only test in the suite that would fail against the inverted implementation |
| `a_watch_refuses_to_baseline_a_broken_pair` | DB12 — the construction-time guarantee, asserted on the `Result` |
| `a_stateless_check_cannot_see_a_reset_and_a_watch_can` | DB12 — the D3 case, and why `new` cannot check it |

### DB11 — the natural implementation inverts a permanent fault into a single lost message


Write the obvious `observe`: read the two cursors, compare against the baseline,
store the new reading, return the comparison's result. Every check still runs.
Every violation is still constructed and returned. The function is one line
different and that line is at the end.

What changes is what the *second* call reports. Having stored the corrupt reading
as its baseline, the watch now compares the next corrupt reading against a corrupt
baseline — finds them consistent — and returns `Ok`. A cursor that went backwards
and stayed there is reported exactly once, and a caller sampling every hundred
milliseconds sees one violation in a stream of successes and has no way to tell it
from a transient.

**The version that adopts the corruption passes every "is a violation detected"
test in this suite.** `a_cursor_that_goes_backwards_is_caught` calls `observe`
once. So does `a_backwards_consumer_is_named_as_the_consumer`. The distinguishing
test is `a_failed_observation_leaves_the_baseline_alone`, which exists precisely
because the property is invisible to single-call assertions — and it is the only
one of the suite's twenty-six tests that would fail against the inverted
implementation.

That ratio is the finding: **one test out of twenty-six separates a correct
instrument from one that launders a permanent fault into a transient**, and it
does so by asserting about the receiver rather than about the return value. The
denominator has grown twice since this was written and the numerator has not; the
recipe above prints both, so the ratio is checkable rather than remembered.

### DB12 — `new` checks first, which makes one state unrepresentable and one still reachable


`Watch::new` runs `check_seqs` before constructing, so no `Watch` value exists
whose *baseline* violates D1 or D2. That is a genuine construction-time
guarantee and it is the reason `a_watch_refuses_to_baseline_a_broken_pair` can
assert about a `Result` rather than about a subsequent call.

It does not extend as far as it appears to, in two specific ways worth pinning:

**D3 is unchecked at construction, necessarily.** A backwards move is a property
of two readings, and `new` has one. So a `Watch` started *after* a cursor reset
holds a baseline that is entirely consistent with itself and entirely wrong about
the ring's history — `api/001`'s precondition B3, and a limit no implementation
removes.

**And the guarantee is about the baseline, not about the pair.** Combined with
[DB5](../data_structure/001_a_watch_is_three_scalars_and_no_identity.md), a
`Watch` validated against one pair and then observed against another carries a
construction-time guarantee that says nothing about the ring it is now reporting
on. The two findings compose into something neither states alone: **the strongest
guarantee this crate offers at construction is silently voided by a call that
type-checks.**

Neither limit was wrong, and neither was written anywhere a caller would meet it.
Both were reachable only from this document and from `api/001`'s precondition
list, which is to say from two places nobody consults while typing
`Watch::new( pair )?`.

**Disposition:** applied — `Watch::new`'s rustdoc carries a *What this does not
guarantee* section stating both edges where the constructor is read: that D3 is
unchecked here of necessity, because a backwards move is a property of two
readings and `new` holds one; and that what was validated is the baseline rather
than the pair, so observing against a different `CursorPair` voids the guarantee
without failing to compile. The limits are now told by the thing that has them.
Now prints: `rustdoc sections recording what new does not guarantee: 1`
