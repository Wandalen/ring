# Decision: A Watch Does Not Latch

**Status:** pending. Current behaviour is "does not latch", by default rather
than by decision — no code was written either way, and the transition is real.

### Scope

- **Purpose**: Record why a `Watch` that has reported a violation goes back to reporting `Ok` when the ring returns to a valid state, and what would settle whether it should.
- **Responsibility**: The context, the two positions, the deferral, and the cost of deferring.
- **In Scope**: The recovery transition; what "recovered" can and cannot mean to this crate.
- **Out of Scope**: The write-deferral that makes a *repeated* fault report repeatedly (→ [`pattern/002`](../pattern/002_commit_nothing_until_every_check_has_passed.md)); the state machine itself (→ [`lifecycle/001`](../lifecycle/001_from_one_observation_to_a_sequence.md)).

### Context

`Watch::observe` compares the current reading against the baseline and returns.
It holds no flag, so nothing distinguishes a watch that has never seen a violation
from one that saw ten and is now looking at a valid pair.

A cursor written wrongly and then written correctly therefore leaves a watch that
passes. That is `lifecycle/001`'s T6, and it is reachable in practice: a
corrupting write followed by a legitimate producer advance restores the ordering
the checks test for.

### Options

| Option | What the instrument reports | What it costs |
|---|---|---|
| Do not latch (current) | Only what is currently observable | An investigator polling at intervals can miss a fault entirely |
| Latch on first violation | Every observation after a fault, forever | A fourth field, and a `Watch` that cannot be reused across phases without a reset verb |
| Latch with an explicit `reset` | The same, until the caller says otherwise | The above, plus a decision about what `reset` means when the ring is still broken |

### Decision

**Deferred, and the reasoning is worth keeping because both positions are
coherent.**

A latch means the instrument *remembers*, which is what an investigator usually
wants — a fault that appeared once and cleared is exactly the kind that is hardest
to catch and most worth knowing about.

Not latching means the instrument reports what it can currently observe, and
nothing else. This is the stricter reading of what a *checker* is, and it avoids
a question this crate cannot answer: what "recovered" means. A ring whose consumer
cursor was corrupted and then overwritten with a plausible value has not
recovered — it has lost records silently — and a `Watch` has no way to tell that
from a genuine transient. **Latching would make the instrument claim knowledge
about the ring's history that it does not have.**

### Consequences

The deferral is cheap to reverse — a `bool` field and one branch — and the
information needed to decide does not exist yet. What would settle it is an actual
investigation where the un-latched behaviour lost something that mattered.
Adding a latch now would be a guess about how the crate gets used, made before it
has been used, which is the shape of decision this family declines to make.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
S=ring_debug/src/lib.rs
echo '-- Watch fields: a latch would be a fourth --'
command grep -m1 -A6 -F 'pub struct Watch' $S
echo '-- and nothing in observe records that a violation ever happened --'
printf 'assignments to self in observe: %s\n' "$( command grep -m1 -A35 -F '  pub fn observe( &mut self, pair : &CursorPair ) -> Result< (), Violation >' $S | command grep -c 'self\.[a-z_]* =' )"
echo '-- is the recovery transition exercised? --'
python3 - << 'EOF'
import re
t = open( 'ring_debug/tests/debug_test.rs' ).read()
parts = re.split( r'^fn ([a-z_]+)\(\)', t, flags = re.M )
faulted, walked = [], []
for name, body in zip( parts[ 1 : : 2 ], parts[ 2 : : 2 ] ) :
  # each observe is classified by whichever of Err/Ok appears first before the
  # next observe in the same test — the crate writes `Err` on its own line, so a
  # bare `Err\(` match misses six of the seven and reads as evidence of absence
  occ = [ m.start() for m in re.finditer( r'\.observe\(', body ) ]
  seen_err = False
  for i, s in enumerate( occ ) :
    window = body[ s : occ[ i + 1 ] if i + 1 < len( occ ) else len( body ) ]
    e = re.search( r'is_err|\bErr\s*\(', window )
    o = re.search( r'is_ok|\bOk\s*\(', window )
    if e and ( not o or e.start() < o.start() ) :
      if not seen_err : faulted.append( name )
      seen_err = True
    elif o and seen_err :
      walked.append( name ); break
print( f"tests observing a violation at all:   {len( faulted )}" )
print( f"of those, tests that then observe Ok: {len( walked )}" )
EOF
```

Live output:

```
-- Watch fields: a latch would be a fourth --
pub struct Watch
{
  producer : Seq,
  consumer : Seq,
  capacity : Capacity,
}

-- and nothing in observe records that a violation ever happened --
assignments to self in observe: 2
-- is the recovery transition exercised? --
tests observing a violation at all:   8
of those, tests that then observe Ok: 1
```

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_one_observation_to_a_sequence.md](../lifecycle/001_from_one_observation_to_a_sequence.md) | T6, the transition this decision is about |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_commit_nothing_until_every_check_has_passed.md](../pattern/002_commit_nothing_until_every_check_has_passed.md) | Why a *persisting* fault reports every time, which is the property latching would extend |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `Watch`'s three fields, and the absence of a fourth |

### Tests

| Test | Relationship |
|------|--------------|
| `a_failed_observation_leaves_the_baseline_alone` | The property latching would extend — a persisting fault already reports every time |
| `a_watch_follows_an_ordinary_run_quietly` | The multi-observation path along which a recovery would occur |

### DB15 — the transition this decision is about is not exercised

T6 — violation, then a valid reading, then `Ok` — is named as a real transition in
`lifecycle/001` and is the entire subject of this decision. No test walks it.

The suite has twenty-six tests, seven of which observe a violation, and not one of
those seven carries on to observe again. So the behaviour the decision defers on
is **documented, reasoned about at length, and unverified**: if `observe` did
latch today, by accident or by a half-applied edit, every test in this crate would
still pass and this document would still describe the crate correctly by
coincidence.

**The recipe above measures that directly now, and used to measure a proxy for
it.** It counted tests calling `observe` more than twice, which was zero when this
was written and became one the moment a *clone* test was added — a test with
nothing to do with T6. A proxy that a fourth unrelated test can move is a proxy
that will eventually report the transition as covered while it is not, which is
the failure this whole document is about, one level up.

**A pending decision about behaviour that no test pins is a decision about
something nobody has confirmed is true.** The fix is one test, not a design
change, and it is worth having *before* the decision is settled either way —
because the test is what makes "current behaviour" a fact rather than a reading of
the source.

### DB16 — deferring is stated as cheap, and its cost is the one thing not measured

The Consequences section above says the deferral is cheap to reverse, and by the
usual measure it is: one field, one branch.

That measure is the wrong one. The cost of *not* latching is borne by an
investigator who polls a live ring and sees nothing — and it is invisible by
construction, because a missed fault leaves no trace to count. Every other cost in
this crate's documents is measured with a command; this one cannot be, and the
document does not say so.

Recorded because the asymmetry is systematic rather than local: **a deferred
decision's cost accrues to whoever hits the case, and the deferring document is
written by someone who has not.** The family's other Pending entries share the
shape — `ring_factory`'s Pending 8 sat as a suspicion until a crate that reached
both doors measured it — and the pattern there was that the evidence arrived from
a *consumer*, not from the deferring crate. This crate has no consumer.

