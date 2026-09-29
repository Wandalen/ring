# Pattern: Driven, Not Self-Firing

### Scope

- **Purpose**: State the practice forced on this crate by its position — it decides whether to flush and never initiates one on its own — and account for why that is a design commitment rather than a limitation to be engineered away.
- **Responsibility**: Give the problem, the solution, its applicability, and its consequences.
- **In Scope**: The decide/act split; who supplies the facts a decision needs.
- **Out of Scope**: The driver's exact signature (→ [The Driver Surface](../api/002_the_driver_surface.md)); the specific trigger this most affects (→ [`OnBarrier` Cannot See the Barrier](../pitfall/001_on_barrier_cannot_see_the_barrier.md)).

### Problem

**A component that decides *when* something should happen has two ways to
act, and only one of them composes.**

It can watch for the condition itself — polling, subscribing, holding a timer,
running a thread — or it can be told the condition holds and answer yes or no.
The first is self-firing; the second is driven.

Self-firing is the obvious design and it fails here for a concrete reason:
**this crate cannot observe two of its three triggers.**

| Trigger | Observable from here? |
|---------|----------------------|
| `OnFull` | **Yes** — buffer occupancy is visible through `ring_tls` |
| `OnBatch( n )` | **Yes** — the count is this crate's own state |
| `OnBarrier` | **No.** `ring_barrier` is not in the transitive closure at all — neither the barrier type nor an instance of one is reachable from here (→ [the pitfall](../pitfall/001_on_barrier_cannot_see_the_barrier.md), and `FL39` below) |

A design where two triggers are self-firing and one is driven is worse than
either pure form: it has two mechanisms, two failure modes, and a variant that
behaves structurally unlike its siblings while presenting as an equal
alternative.

**There is a second, independent reason.** A self-firing flush runs at a moment
the consumer did not choose — inside an append, on a timer thread, during a
`Drop`. Every one of those makes the publication point a property of timing
rather than of design, which is exactly what
[the publication-point invariant](../invariant/002_publication_point_is_designed_not_inherited.md)
forbids. Self-firing would break the crate's central invariant in the act of
implementing its central feature.

### Solution

**The crate exposes a driver the consumer calls. Facts the decision needs but
cannot observe arrive as arguments. The crate answers and acts; it never
initiates.**

```text
consumer                          ring_flush                    ring_tls
   │                                   │                            │
   ├─ drive( barrier_reached: bool ) ─▶│                            │
   │                                   ├─ consult policy            │
   │                                   │  (occupancy, count, arg)   │
   │                                   │                            │
   │                                   ├─ if fire: seal ───────────▶│
   │                                   ├─          drain ─────────▶│
   │                                   ├─          reset ─────────▶│
   │◀─ FlushOutcome ───────────────────┤                            │
```

- **No thread.** This crate spawns nothing and owns no timer.
- **No callback registration.** Nothing here subscribes to anything.
- **No `Drop` behaviour that flushes.** Teardown flushing is an explicit call (→ [From Configuration to the Final Drain](../lifecycle/002_from_configuration_to_the_final_drain.md)).
- **Every trigger uses the same mechanism.** `OnFull` could poll occupancy itself and deliberately does not — uniformity across variants is worth more than a marginally earlier flush.

**The third bullet is the one that gets violated.** A `Drop` that flushes is
the single most natural safety addition, and it reintroduces both problems at
once: an unscheduled publication point, and a variant behaving differently from
its siblings.

### Applicability

| Situation | Applies |
|-----------|---------|
| The component cannot observe some of its own triggers | **Yes** — the forcing case here |
| The timing of the action is itself the property being controlled | **Yes** — self-firing surrenders exactly what is being designed |
| The component is a library with no runtime of its own | **Yes** |
| The caller has a natural cadence already (a tick, a frame) | **Yes** — driving costs nothing it was not already paying |
| The condition is genuinely local and cheap to poll | No — driving adds a call for nothing |
| Latency between condition and action must be minimal | No — driving bounds it to the caller's cadence |
| No caller has a suitable cadence | No — something must then own a schedule |

**The last two rows are the honest cost.** Driven means the worst-case latency
between "the buffer filled" and "the flush happened" is one driver interval,
not zero. For a tick-based consumer that is exactly right; for a latency-
sensitive consumer with no tick it is a real limitation, and the answer is that
such a consumer needs a different design, not that this crate should grow a
thread.

### Consequences

| # | Consequence | Direction |
|---|-------------|-----------|
| R1 | All three variants share one mechanism | **Positive** — one code path, one failure mode, comparable measurements |
| R2 | The crate has no runtime, no thread, no timer | **Positive** — nothing to shut down, nothing to leak, trivially testable |
| R3 | Publication happens only at points the consumer chose | **Positive** — the invariant holds by construction |
| R4 | `OnFull` may notice fullness later than it could | **Negative**, bounded by the driver interval |
| R5 | A consumer that forgets to call the driver never flushes | **Negative and silent** — indistinguishable from an idle buffer |
| R6 | The consumer must supply the barrier fact correctly | **Negative** — this crate cannot check that the announced barrier is the one the ring is gated on |
| R7 | Testing needs no threads, no timing, no sleeps | **Positive** — the scripted sequence in the acceptance criterion is possible *because* of this pattern |

**R7 deserves more weight than it usually gets.** This crate's acceptance
criterion requires proving each policy fires "at exactly its stated trigger and
at no other point." Against a self-firing design that is a statement about
concurrent timing and is close to untestable. Against a driven design it is a
scripted sequence of calls with a recorded log — deterministic, fast, no
sleeps. **The pattern is what makes the acceptance criterion checkable at all.**

**R5 and R6 are the standing costs and neither is closable from here.** Both
are the consumer holding the other end of a contract this crate states and
cannot verify — the same division
[`ring_handle`'s barrier lifecycle](../../../ring_handle/docs/lifecycle/002_the_barrier_holds_the_consumer.md)
records, where the crate makes a property expressible and the scheduler makes
it true.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_driver_surface.md](../api/002_the_driver_surface.md) | The surface this pattern produces, and where R6's unverifiable argument enters |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_evaluating_a_policy_at_an_append.md](../algorithm/001_evaluating_a_policy_at_an_append.md) | What the driver runs when called |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_publication_point_is_designed_not_inherited.md](../invariant/002_publication_point_is_designed_not_inherited.md) | R3 — self-firing would break this in the act of implementing the feature |

### Patterns

| File | Relationship |
|------|--------------|
| [001_policy_as_a_value.md](001_policy_as_a_value.md) | Its Q7 — this pattern is the other half of the property |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_on_barrier_cannot_see_the_barrier.md](../pitfall/001_on_barrier_cannot_see_the_barrier.md) | The forcing case, and R5's silent-failure shape |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's "scripted sequence with a recorded flush log" row — R7, and evidence the acceptance criterion assumes this pattern |
| [`ring_tls/docs/api/002_consolidator_read_surface.md`](../../../ring_tls/docs/api/002_consolidator_read_surface.md) | The three primitives the driver sequences, exposed separately for exactly this caller |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | R7 — `appending_never_publishes` (thirty-two appends, nothing in the ring, empty log) and `dropping_a_driver_with_records_staged_publishes_nothing` (the one remaining path by which something could have fired unbidden). Every test in the file relies on the determinism these two assert |

### FL39 — The Crate the Unobservable Trigger Is Named After Is Not in This Crate's Dependency Graph at All

The concession the problem statement makes, and the graph it makes it about:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what this instance concedes --'
awk '/^### FL/{ exit } /transitive closure/{ printf "    %d: %s\n", NR, $0 }' \
  ring_flush/docs/pattern/002_driven_not_self_firing.md | sed -E 's/^(.{0,124}).*/\1/'
echo '  -- the closure, computed from the manifests --'
python3 - <<'PY'
import os, re
M = 'ring'
def deps( c ):
  p = os.path.join( M, c, 'Cargo.toml' )
  if not os.path.exists( p ): return []
  out, sec = [], None
  for ln in open( p ):
    if ln.startswith( '[' ): sec = ln.strip()
    elif sec and 'dependencies' in sec and 'dev' not in sec and 'path' in ln:
      m = re.match( r'\s*([A-Za-z0-9_]+)\s*=', ln )
      if m: out.append( m.group( 1 ) )
  return out
seen, frontier = set(), [ 'ring_flush' ]
while frontier:
  for d in deps( frontier.pop() ):
    if d not in seen: seen.add( d ); frontier.append( d )
print( '    %d crates: %s' % ( len( seen ), ' '.join( sorted( seen ) ) ) )
print( '    ring_barrier present:', 'ring_barrier' in seen )
PY
echo '  -- and every mention of it in this crate --'
printf '    files under ring_flush naming ring_barrier: %s\n' \
  "$( command grep -rl 'ring_barrier' ring_flush/src ring_flush/tests ring_flush/Cargo.toml 2>/dev/null | wc -l )"
```

Live output:

```
  -- what this instance concedes --
    26: | `OnBarrier` | **No.** `ring_barrier` is not in the transitive closure at all — neither the barrier type nor an ins
  -- the closure, computed from the manifests --
    18 crates: ring_align ring_atomic ring_batch ring_store ring_claim ring_config ring_core ring_cursor ring_gating ring_index ring_mpsc ring_overflow ring_seqno ring_slot ring_spsc ring_stats ring_tls ring_types
    ring_barrier present: False
  -- and every mention of it in this crate --
    files under ring_flush naming ring_barrier: 0
```

The problem statement concedes a point in order to be precise: `OnBarrier` is
unobservable from here not because the barrier machinery is unreachable but
because "`ring_barrier` is in the transitive closure and no barrier *instance*
is." That is a careful distinction and it is about a graph that does not contain
the crate.

Eighteen crates are reachable from this one and `ring_barrier` is not among
them, nor is it named anywhere in this crate's source, tests or manifest. The
concession is more generous than the facts require: the crate cannot see a
barrier instance, cannot see the barrier *type*, and does not depend on the crate
that defines either.

**The pattern's conclusion is unaffected and its argument gets simpler.** Driven
rather than self-firing is forced whether the trigger's vocabulary is one
dependency away or absent entirely. What is lost is precision in the opposite of
the usual direction — a document that overstates its own access, and therefore
understates how completely the fact has to arrive from the caller.

The likely origin is worth noting because it will recur: `ring_barrier` exists,
this crate's subject is barriers, and the two facts were joined without checking
the manifest. A dependency graph is the cheapest claim in the corpus to verify
and one of the easiest to assert from familiarity.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
F=ring_flush/docs/pattern/002_driven_not_self_firing.md
command grep -m2 'not in the transitive closure at all' "$F"
```

Live output:

```
| `OnBarrier` | **No.** `ring_barrier` is not in the transitive closure at all — neither the barrier type nor an instance of one is reachable from here (→ [the pitfall](../pitfall/001_on_barrier_cannot_see_the_barrier.md), and `FL39` below) |
    26: | `OnBarrier` | **No.** `ring_barrier` is not in the transitive closure at all — neither the barrier type nor an ins
```

**Disposition:** applied — the `OnBarrier` row no longer concedes that
`ring_barrier` sits one step away in the transitive closure; it states the
measured fact that the crate is absent from the closure entirely, matching
the 18-crate census this finding ran. The pattern's own conclusion is
unaffected, exactly as the finding says, so nothing else in the document
needed to change. Now prints: `not in the transitive closure at all`

### FL40 — R5 Calls the Failure Indistinguishable From an Idle Buffer, and the Crate Exposes the Number That Distinguishes Them

The consequence as written, and the accessor that contradicts it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the consequence --'
awk '/^### FL/{ exit } /^\| R5 \|/{ printf "    %d: %s\n", NR, $0 }' \
  ring_flush/docs/pattern/002_driven_not_self_firing.md
echo '  -- the accessor --'
awk -v n1="$( command grep -n -m1 -F '  /// How many records are staged.' ring_flush/src/lib.rs | cut -d: -f1 )" -v n2="$(( $( command grep -n -m1 -F '    self.buffer.len()' ring_flush/src/lib.rs | cut -d: -f1 ) + 1 ))" 'NR >= n1 && NR <= n2 { printf "    %d: %s\n", NR, $0 }' ring_flush/src/lib.rs
echo '  -- who reads it --'
printf '    assertions inside this crate: %s\n' \
  "$( command grep -rc '\.staged()' ring_flush/tests 2>/dev/null | cut -d: -f2 | paste -sd+ | bc )"
printf '    call sites anywhere else in the family: %s\n' \
  "$( command grep -rn '\.staged()' ring_*/src ring_*/tests 2>/dev/null \
       | command grep -cv '^ring_flush/' )"
echo '  -- and whether the one outside consumer drives --'
command grep -E 'flusher\.(drive|drain_final)' ring_bench/src/lib.rs | sed 's/^/    /'
```

Live output:

```
  -- the consequence --
    97: | R5 | A consumer that forgets to call the driver never flushes | **Negative and silent** — indistinguishable from an idle buffer |
  -- the accessor --
    452:   /// How many records are staged.
    453:   ///
    454:   /// **Advisory.** Between this read and any action taken on it, an append on
    455:   /// the owning thread can change the count. It is useful for diagnostics and
    456:   /// for deciding whether a drive is worthwhile; it is not a basis for a
    457:   /// correctness decision.
    458:   #[ must_use ]
    459:   pub fn staged( &self ) -> usize
    460:   {
    461:     self.buffer.len()
    462:   }
  -- who reads it --
    assertions inside this crate: 23
    call sites anywhere else in the family: 0
  -- and whether the one outside consumer drives --
        if let FlushOutcome::Flushed { count } = flusher.drive()
      if let FlushOutcome::Flushed { count } = flusher.drain_final()
```

R5 says a consumer who forgets to drive gets a failure that is "negative and
silent — indistinguishable from an idle buffer." The two states are not
indistinguishable. An idle buffer has nothing staged; a forgotten driver has
records staged and no flush, and `staged()` is a public accessor returning
exactly that count. The crate's own suite reads it twenty-three times.

**Nothing outside this crate reads it at all.** So the accurate statement is not
that the failure is undetectable but that it is *undetected* — the signal exists,
is public, is documented, and has no consumer. That is a materially different
consequence: an undetectable failure is a design limit and closes the subject; an
unread signal is a gap somebody can close, and R5's phrasing removes the reason
to try.

**The accessor's own "advisory" caveat does not cover this use.** It warns that an append on the owning thread can change the count between the read and any action on it — a race that matters when the count drives a branch, and does not exist at teardown, where the question is whether records were left behind and nothing further will append. The comment names diagnostics as a use it is suitable for, and R5's failure is a diagnostic.

The one external consumer does drive, and drains at the end, so the failure has
never occurred. That is worth recording alongside the correction, because it
explains why nobody has needed the accessor: R5 describes a hazard for a
population of consumers that currently has one member, and that member is
correct.

**The general shape:** a consequence table written during design states what the
design *cannot* do, and stays unrevised when the implementation adds a partial
answer. `staged()` is not a full answer — it requires somebody to look — but the
difference between "no signal" and "an unwatched signal" is the whole distance
between a limitation and a task.
