# Lifecycle: From One Observation to a Sequence

### Scope

- **Purpose**: Specify `Watch`'s states and transitions — the stateful half of the crate, which exists because D3 is not a property of any single reading.
- **Responsibility**: The states a watch occupies, what moves it between them, and the two transitions that are easy to get wrong.
- **In Scope**: `Watch::new`, `Watch::observe`, `Watch::last`; the baseline's update rule.
- **Out of Scope**: The comparison performed at each observation (→ [`algorithm/001`](../algorithm/001_checking_a_pair_without_touching_it.md)); D3 itself (→ [`invariant/001`](../invariant/001_cursor_invariants_over_a_live_ring.md)).

### States

| # | State | Meaning | Holds |
|---|-------|---------|-------|
| W0 | **Unstarted** | No `Watch` exists | Nothing |
| W1 | **Watching** | A baseline is held and every observation so far has passed | `(p, c)` from the most recent passing observation, and the capacity |
| W2 | **Reporting** | The most recent observation found a violation | The **same** baseline as before that observation — deliberately unchanged |

**There is no terminal state.** A `Watch` in W2 is still usable and still
reporting; `observe` may be called again and will compare against the same
baseline. That is the point — see T4.

### Transitions

| # | From | To | Trigger | Effect |
|---|------|----|---------|--------|
| T1 | W0 | W1 | `Watch::new` on a pair satisfying D1 and D2 | Baseline set to the observed `(p, c)` |
| T2 | W0 | W0 | `Watch::new` on a pair violating D1 or D2 | **No watch is created.** The violation is returned instead |
| T3 | W1 | W1 | `observe` finds no violation | Baseline advances to the new `(p, c)` |
| T4 | W1 | W2 | `observe` finds any violation | Baseline **unchanged**; the violation is returned |
| T5 | W2 | W2 | `observe` finds a violation again | Baseline unchanged; reported again |
| T6 | W2 | W1 | `observe` finds no violation | Baseline advances |

### Behavioral Invariants

Four properties hold over every path through this machine, and each one is a
thing the obvious implementation gets wrong:

1. **The baseline only ever advances on success.** T4 and T5 leave it alone. A
   `observe` that stored unconditionally would adopt a corrupt reading as normal,
   and the fault would be reported once and then never again.
2. **The baseline is monotonic.** Since it advances only through T3 and T6, and
   both require D3 to hold, the stored `(p, c)` never moves backwards over the
   watch's whole life — which is what makes D3 checkable at all.
3. **W2 is not absorbing.** T6 exists: a ring that recovers is reported as
   recovered. The machine describes a *condition*, not a latched alarm.
4. **No transition writes to the ring.** Every edge is driven by loads, so the
   machine can be run against a live pair without changing what it observes.

#### T2 and T4 are the two that matter

**T2 — a watch refuses to baseline a broken pair.** The alternative is a `Watch`
constructed against an already-corrupt ring that adopts the corruption as its
reference point and reports nothing thereafter. The instrument would be silent
precisely where it was installed to speak, and the silence would read as health.
`Watch::new` therefore returns `Result< Self, Violation >` rather than `Self` —
the failure is not constructible-and-broken, it is not constructed at all
(`a_watch_refuses_to_baseline_a_broken_pair`).

**T4 — a failed observation does not become the new baseline.** The natural
implementation reads, compares, stores, returns; storing on the way out means a
permanent fault is reported once and then treated as normal. T5 exists only
because T4 holds: a corruption that persists is reported on every observation,
not on the first (`a_failed_observation_leaves_the_baseline_alone` asserts both —
that `last()` is unmoved, and that the second look still fails).

**T6 is real and is not a bug.** A ring can recover: a cursor written wrongly and
then written correctly leaves a watch comparing against a stale-but-valid
baseline, and forward movement from it passes. The crate reports what it can
observe; it does not claim to remember that something was once wrong. Anything
stronger would need a fault latch, which is a decision for a caller that knows
what a fault should mean to it, not for the instrument.

#### What this machine cannot see

| # | Case | Why |
|---|------|-----|
| S1 | Corruption before `Watch::new` | The baseline is the first observation; there is nothing before it to compare against |
| S2 | Corruption fully repaired between two observations | Both readings are legal and forward; the machine has no third sample |
| S3 | Both cursors advanced by the same wrong amount | D1, D2 and D3 all hold — this is [`invariant/001`](../invariant/001_cursor_invariants_over_a_live_ring.md)'s V4 |

**S1 is structural and cannot be removed** — no instrument sees what happened
before it was switched on. **S2 is the cost of sampling** rather than
instrumenting: closing it would mean hooking every `store`, which means changing
`ring_cursor`, which means putting this crate's cost on the family's hot path
(→ [the pitfall](../pitfall/001_saturating_arithmetic_reports_health.md)'s P4).

The three are listed because a reader who knows only that the crate "checks
cursor invariants" will assume more coverage than exists.

### Failure

| # | Failure | Consequence |
|---|---------|-------------|
| M1 | Baseline updated on a failed observation | A permanent fault is reported once, then silently accepted (T4 lost) |
| M2 | `Watch::new` returning `Self` rather than `Result` | A watch installed on a broken ring is silent forever (T2 lost) |
| M3 | D3 checked after D1/D2 | A backwards cursor is reported as whichever ordering violation it happened to cause |
| M4 | The baseline read afresh inside `observe` rather than reused | Two different observations compared against each other; violations reported that never existed |

**M4 is subtle and is why `check_seqs` is split out of `check`.** `observe` has
already loaded `p` and `c` for the D3 comparison; calling `check( pair )` at that
point would load them a *second* time. The D3 comparison and the D1/D2 comparison
would then be about different moments, and a legal advance landing between the
two reads could be reported as a violation.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug
echo '-- the order observe checks in --'
awk '/pub fn observe\( &mut self/{ f = 1 } f && ( /CursorWentBackwards/ || /check_seqs/ || /\.load\( OBSERVE \)/ ) { sub( /^ */, "" ); print "  " $0 } f && /^  }$/{ exit }' src/lib.rs
echo '-- the two tests the failure table cites for D3 and for M3 --'
for t in a_backwards_consumer_is_named_as_the_consumer a_watch_still_catches_the_stateless_violations ; do
  echo "  $t"
  awk -v t="$t" '$0 ~ "fn " t { f = 1 } f && ( /pair_at\(/ || /\.store\(/ || /Violation::/ ) { sub( /^ */, "" ); print "    " $0 } f && /^}$/{ exit }' tests/debug_test.rs
done
echo '-- every test that drives the machine, as a sequence of observations --'
awk '
  /^fn [a-z_]+\(\)/ { name = $2; order[ ++n ] = name }
  /Watch::new\(/ { newcall[ name ] = ( /expect|unwrap/ ? "T1" : "T2" ) }
  /observe\(/ { seq[ name ] = seq[ name ] ( /is_ok/ ? "o" : "e" ) }
  END {
    for ( i = 1; i <= n; i++ )
    {
      m = order[ i ]
      if ( seq[ m ] != "" || newcall[ m ] != "" )
        printf "  %-50s new:%-3s observes:%s\n", m, newcall[ m ], seq[ m ]
    }
    t1 = t2 = t3 = t4 = t5 = t6 = "NO"
    for ( m in newcall ) { if ( newcall[ m ] == "T1" ) t1 = "yes"; if ( newcall[ m ] == "T2" ) t2 = "yes" }
    for ( m in seq )
    {
      s = seq[ m ]
      for ( j = 1; j <= length( s ); j++ )
      {
        cur = substr( s, j, 1 ); prev = ( j > 1 ? substr( s, j - 1, 1 ) : "o" )
        if ( prev == "o" && cur == "o" ) t3 = "yes"
        if ( prev == "o" && cur == "e" ) t4 = "yes"
        if ( prev == "e" && cur == "e" ) t5 = "yes"
        if ( prev == "e" && cur == "o" ) t6 = "yes"
      }
    }
    printf "  covered:  T1 %s  T2 %s  T3 %s  T4 %s  T5 %s  T6 %s\n", t1, t2, t3, t4, t5, t6
  }
' tests/debug_test.rs
echo '-- control for behavioural invariant 4: stores in executable crate code --'
printf '  %s\n' "$( command grep -n 'store(' src/lib.rs | command grep -vcE ':(//!|///)' || true )"
```

Live output:

```
-- the order observe checks in --
  Violation::CursorWentBackwards
  Violation::CursorWentBackwards
  check_seqs( producer, consumer, self.capacity )?;
-- the two tests the failure table cites for D3 and for M3 --
  a_backwards_consumer_is_named_as_the_consumer
    let pair = pair_at( 8, 20, 15 );
    pair.consumer().store( Seq( 4 ), Ordering::Release );
    Violation::CursorWentBackwards { cursor : Cursor::Consumer, was : Seq( 15 ), now : Seq( 4 ) }
  a_watch_still_catches_the_stateless_violations
    let pair = pair_at( 8, 5, 1 );
    pair.consumer().store( Seq( 9 ), Ordering::Release );
    Err( Violation::ConsumerAheadOfProducer { producer : Seq( 5 ), consumer : Seq( 9 ) } )
-- every test that drives the machine, as a sequence of observations --
  a_cursor_that_goes_backwards_is_caught()           new:T1  observes:oe
  a_backwards_consumer_is_named_as_the_consumer()    new:T1  observes:e
  a_stateless_check_cannot_see_a_reset_and_a_watch_can() new:T1  observes:e
  a_watch_refuses_to_baseline_a_broken_pair()        new:T2  observes:
  a_failed_observation_leaves_the_baseline_alone()   new:T1  observes:ee
  a_watch_that_faulted_reports_ok_once_the_ring_recovers() new:T1  observes:eoo
  a_watch_follows_an_ordinary_run_quietly()          new:T1  observes:o
  a_watch_still_catches_the_stateless_violations()   new:T1  observes:e
  a_cloned_watch_forks_the_baseline()                new:T1  observes:ooe
  observing_a_foreign_pair_answers_about_neither_ring() new:T1  observes:e
  covered:  T1 yes  T2 yes  T3 yes  T4 yes  T5 yes  T6 yes
-- control for behavioural invariant 4: stores in executable crate code --
  0
```

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_cursor_invariants_over_a_live_ring.md](../invariant/001_cursor_invariants_over_a_live_ring.md) | D3 and V3 — what this machine exists to detect; V4 as S3 |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_checking_a_pair_without_touching_it.md](../algorithm/001_checking_a_pair_without_touching_it.md) | The D1/D2 comparison each transition runs, and M4's `check_seqs` split |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | A3 and B3 — T4 and S1 stated as a guarantee and a precondition |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_saturating_arithmetic_reports_health.md](../pitfall/001_saturating_arithmetic_reports_health.md) | P3 — the reset this machine exists for, and why no stateless check reaches it |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_violation.md](../type/001_violation.md) | `CursorWentBackwards` — the variant only this machine produces |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `Watch`, its three methods, and the store-only-on-success rule |

### Tests

| File | Relationship |
|------|--------------|
| `tests/debug_test.rs` | T1/T3 — `a_watch_follows_an_ordinary_run_quietly`; T2 — `a_watch_refuses_to_baseline_a_broken_pair`; T4/T5 — `a_failed_observation_leaves_the_baseline_alone`; D3 — `a_cursor_that_goes_backwards_is_caught`, `a_backwards_consumer_is_named_as_the_consumer`; S1's counterpart — `a_stateless_check_cannot_see_a_reset_and_a_watch_can`; M3's absence — `a_backwards_consumer_is_named_as_the_consumer` |

### DB37 — the failure table credits M3 to a test running on a pair where M3 cannot fail

M3 is *"D3 checked after D1/D2 — a backwards cursor is reported as whichever
ordering violation it happened to cause."* Detecting it needs a pair that breaks
D3 **and** an ordering invariant at the same time, so the two candidate answers
differ.

The document credits its absence to `a_watch_still_catches_the_stateless_violations`.
That test starts at `pair_at( 8, 5, 1 )` and stores consumer `9`: both cursors moved
forward, so D3 holds. There is no D3 answer for the implementation to prefer, and
the test would pass identically under M3.

The evidence is present and is cited two clauses earlier for something else.
`a_backwards_consumer_is_named_as_the_consumer` starts at `pair_at( 8, 20, 15 )` and
stores consumer `4`, which leaves producer `20` against consumer `4` at capacity
`8` — a distance of `16`, so D2 is broken as well as D3 — and asserts
`CursorWentBackwards`. **That is exactly the discrimination M3 describes**, and the
table lists it under D3 while giving M3's row to a test that cannot make it fail.

The implementation is correct — `observe` returns on both D3 branches before
reaching `check_seqs`, which the recipe shows. The finding is that a reader
checking the guard would check the wrong one.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F '| `tests/debug_test.rs` |' ring_debug/docs/lifecycle/001_from_one_observation_to_a_sequence.md
```

Live output:

```
| `tests/debug_test.rs` | T1/T3 — `a_watch_follows_an_ordinary_run_quietly`; T2 — `a_watch_refuses_to_baseline_a_broken_pair`; T4/T5 — `a_failed_observation_leaves_the_baseline_alone`; D3 — `a_cursor_that_goes_backwards_is_caught`, `a_backwards_consumer_is_named_as_the_consumer`; S1's counterpart — `a_stateless_check_cannot_see_a_reset_and_a_watch_can`; M3's absence — `a_backwards_consumer_is_named_as_the_consumer` |
```

**Disposition:** applied — the Tests table's M3's-absence clause no longer
credits `a_watch_still_catches_the_stateless_violations`, a test whose pair
never breaks D3 and so cannot discriminate M3; it now credits
`a_backwards_consumer_is_named_as_the_consumer`, the test that actually breaks
D3 alongside an ordering invariant and asserts the D3 answer is chosen. Now
prints: `a_backwards_consumer_is_named_as_the_consumer`

### DB38 — the transition defended at greatest length is the only one with no test

The state machine has six transitions. Reconstructed from the suite's observation
sequences, five of them are exercised: T1 and T2 by the two forms of
`Watch::new`, T3 by a passing observation, T4 by a failing one, T5 by a second
failing one. **T6 — a watch that reported a violation and then observes cleanly —
appears in no test.** No test in the suite performs a failing observation followed
by a passing one.

T6 is not an incidental edge. It is the transition this document argues for hardest
— *"T6 is real and is not a bug… The machine describes a condition, not a latched
alarm"* — it is the reason W2 is described as non-absorbing, and it is the entire
content of [`decisions/002`](../decisions/002_a_watch_does_not_latch.md), which
defers latching to a caller on the grounds that the instrument reports a condition.

The behaviour almost certainly works: T6 is the same code path as T3, reached from
a different state, and the state is only two `Seq` fields. The gap is that the one
claim distinguishing this design from the latching alternative is the one claim the
suite does not check, so a change that made W2 absorbing would pass everything
here — including `a_failed_observation_leaves_the_baseline_alone`, which asserts
that the second observation *also* fails and is satisfied by a latch.
