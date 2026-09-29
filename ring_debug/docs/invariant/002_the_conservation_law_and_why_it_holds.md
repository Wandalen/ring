# Invariant: The Conservation Law and Why It Holds

### Scope

- **Purpose**: State the invariant `check_ends` evaluates — that a ring's two derived readings account for the whole ring — and establish what it can and cannot detect.
- **Responsibility**: The statement, its class, the mechanism that checks it, and the reason that mechanism is nearly unfalsifiable.
- **In Scope**: D4; `Consumer::len` and `Producer::free_capacity` as the terms it is built from; the externally supplied capacity.
- **Out of Scope**: The cursor invariants (→ [`invariant/001`](001_cursor_invariants_over_a_live_ring.md)); why this door exists at all (→ [`workaround/001`](../workaround/001_the_door_ring_core_does_not_open.md)).

### Invariant Statement

**For a split ring of `capacity` slots, at a moment when nothing is writing:**

| # | Invariant | Excludes |
|---|-----------|----------|
| D4 | `pending + free == capacity` | Two ends of one ring that do not account for the same ring |

D4 is a different class of statement from D1–D3 and the difference matters:

| | D1–D3 | D4 |
|---|---|---|
| Subject | Raw cursors | Readings derived from cursors |
| Terms | All internal to the ring | `capacity` is supplied by the **caller** |
| Failure means | The ring is corrupt | The ring is corrupt, *or* the caller was wrong, *or* the reads were skewed |
| Snapshot or sequence | D1/D2 snapshot, D3 sequence | Snapshot |

**The externally supplied term is what gives D4 its only reliable diagnostic
power.** Nothing on a `ring_core` end reports the ring's capacity
([`workaround/001`](../workaround/001_the_door_ring_core_does_not_open.md)'s W3),
so the number D4 is measured against comes from outside the thing being measured —
and a caller who is wrong about it is exactly what `ReadingsDisagree` reports.

### Enforcement Mechanism

| # | Mechanism | Covers | Gap |
|---|-----------|--------|-----|
| E5 | This crate's `check_ends` | A capacity mismatch between caller and ring | Cursor corruption, of any kind — see below |

**E5 is an algebraic identity in the arithmetic it is built from, and that is the
whole finding.** `Consumer::len` resolves to `ring_spsc::available`, which is
`produced − consumed` saturating — call it *occupancy*. `Producer::free_capacity`
resolves to `capacity − occupancy`. So:

```
pending + free  ==  occupancy + ( capacity − occupancy )  ==  capacity
```

for every pair of cursor values whatsoever, corrupt or sound, as long as the
subtraction does not underflow. D4 is not evaluated against the ring; it is
evaluated against an expression that was built by subtracting from the same
constant it is compared to.

### Violation Consequences

| # | Violation | Reachable how | Consequence |
|---|-----------|---------------|-------------|
| V5 | The caller's capacity is not the ring's | A wrong argument | `ReadingsDisagree`, correctly (`a_ring_measured_against_the_wrong_capacity_disagrees`) |
| V6 | The two reads see different moments | A concurrent writer — B1 violated | A false `ReadingsDisagree` |
| V7 | `occupancy > capacity` (a D2 breach) | Real corruption | **`capacity − occupancy` underflows.** A debug build panics inside `ring_spsc`; a release build wraps, and the sum wraps back to exactly `capacity`, so the check passes |
| V8 | `consumer > producer` (a D1 breach) | Real corruption | Occupancy saturates to 0, `free` is `capacity`, the sum is `capacity`. Passes |

**V7 is the sharp one.** It is the only path by which a genuinely corrupt ring
reaches this check and produces anything other than `Ok` — and what it produces is
a panic in the crate below, not a `Violation`. In a release build, where
`overflow-checks` is unset family-wide
([`pattern/001`](../pattern/001_the_guard_that_makes_the_next_line_legal.md)'s
DB10), the two wraparounds cancel and the ring reports itself sound.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- the two readings check_ends is built from --'
command grep 'let pending\|let free\|pending + free' ring_debug/src/lib.rs | sed 's/^/  /'
echo '-- what they resolve to, one crate down --'
command grep -A3 'pub fn available( &self )' ring_spsc/src/lib.rs | sed 's/^/  /'
command grep -A2 'pub fn free_capacity( &self )' ring_spsc/src/lib.rs | sed 's/^/  /'
echo '-- the other implementation of the same idea, which saturates instead --'
command grep -A4 'pub fn free_slots' ring_seqno/src/lib.rs | sed 's/^/  /'
echo '-- the identity, over a healthy ring and two corrupt ones --'
python3 - << 'EOF'
cap = 8
for name, ( p, c ) in [ ( 'healthy', ( 5, 1 ) ), ( 'D1 corrupt', ( 3, 9 ) ), ( 'D2 lapped', ( 30, 0 ) ) ] :
  occ = max( 0, p - c )                # distance_to saturates at zero
  seq_free = max( 0, cap - occ )       # ring_seqno::free_slots saturates
  print( f"  {name:<11} producer {p:>2} consumer {c:>2} -> occupancy {occ:>2}" )
  print( f"  {'':<11}   ring_seqno  : {occ} + {seq_free} = {occ + seq_free}{' == capacity' if occ + seq_free == cap else ' != capacity -> reports'}" )
  if occ <= cap :
    print( f"  {'':<11}   ring_spsc : {occ} + {cap - occ} = {cap} == capacity  (the identity)" )
  else :
    w = ( cap - occ ) % 2**64
    print( f"  {'':<11}   ring_spsc : capacity - occupancy underflows -> debug panics, release gives {w}" )
    print( f"  {'':<11}               and {occ} + {w} wraps to {( occ + w ) % 2**64} == capacity" )
EOF
echo '-- which tests actually call check_ends --'
awk '/^fn [a-z_]+\(\)/{f=$2} /check_ends\(/{print "  " NR ": " f}' ring_debug/tests/debug_test.rs
```

Live output:

```
-- the two readings check_ends is built from --
    let pending = consumer.len();
    let free = producer.free_capacity();
    if pending + free == capacity.get()
-- what they resolve to, one crate down --
    pub fn available( &self ) -> usize
    {
      let consumed = self.ring.cursors.consumer().load( OWN );
      let produced = self.ring.cursors.producer().load( GATING );
    pub fn free_capacity( &self ) -> usize
    {
      self.ring.capacity().get().saturating_sub( self.occupancy() as usize )
-- the other implementation of the same idea, which saturates instead --
  pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
  {
    let in_flight = consumer.distance_to( producer );
    ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
  }
-- the identity, over a healthy ring and two corrupt ones --
  healthy     producer  5 consumer  1 -> occupancy  4
                ring_seqno  : 4 + 4 = 8 == capacity
                ring_spsc : 4 + 4 = 8 == capacity  (the identity)
  D1 corrupt  producer  3 consumer  9 -> occupancy  0
                ring_seqno  : 0 + 8 = 8 == capacity
                ring_spsc : 0 + 8 = 8 == capacity  (the identity)
  D2 lapped   producer 30 consumer  0 -> occupancy 30
                ring_seqno  : 30 + 0 = 30 != capacity -> reports
                ring_spsc : capacity - occupancy underflows -> debug panics, release gives 18446744073709551594
                            and 30 + 18446744073709551594 wraps to 8 == capacity
-- which tests actually call check_ends --
  504: the_two_ends_of_a_live_ring_agree()
  507: the_two_ends_of_a_live_ring_agree()
  512: the_two_ends_of_a_live_ring_agree()
  543: check_ends_on_a_genuinely_full_ring()
  566: a_ring_measured_against_the_wrong_capacity_disagrees()
```

### Invariants

| File | Relationship |
|------|--------------|
| [001_cursor_invariants_over_a_live_ring.md](001_cursor_invariants_over_a_live_ring.md) | D1–D3, and the E-table this adds E5 to |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | B1 and B2 — V6 and V5 stated as caller obligations |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_door_ring_core_does_not_open.md](../workaround/001_the_door_ring_core_does_not_open.md) | W1 — the capability gap this measures the true size of |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_saturating_arithmetic_reports_health.md](../pitfall/001_saturating_arithmetic_reports_health.md) | The saturation that makes V8 pass silently |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `check_ends` |
| [`ring_spsc/src/lib.rs`](../../../ring_spsc/src/lib.rs) | `available`, `occupancy`, `free_capacity` — the terms |
| [`ring_seqno/src/lib.rs`](../../../ring_seqno/src/lib.rs) | `free_slots` — the same idea, saturating |

### Tests

| Test | Relationship |
|------|--------------|
| `the_two_ends_of_a_live_ring_agree` | D4 holding on a healthy ring — three calls, the only positive coverage |
| `a_ring_measured_against_the_wrong_capacity_disagrees` | V5, the one violation reachable without corruption |
| `check_ends_cannot_see_the_corruption_check_can` | V8, modelled rather than executed — see DB36 |

### DB35 — the third door's check is an identity, so it cannot report a corrupt ring at all

`check_ends` asks whether `pending + free` accounts for the capacity. `pending` is
`ring_spsc::available`, which is occupancy. `free` is
`ring_spsc::free_capacity`, which is `capacity − occupancy`. The sum is the
capacity by construction, for any cursor values at all.

[`workaround/001`](../workaround/001_the_door_ring_core_does_not_open.md)'s W1
states the limitation as *"D1 is undetectable through this door"*. The measurement
above puts it further: **no cursor corruption is detectable through this door.** A
D1 breach saturates occupancy to zero and passes (V8). A D2 breach underflows the
subtraction — a panic in a debug build, and in a release build two wraparounds that
cancel exactly back to `capacity`, so it passes too (V7). D3 is a sequence property
and was never in scope.

What remains is real and worth having: a capacity the caller got wrong, and read
skew. Both are genuine faults and V5 has a test. But the crate's Contract-reachable
entry point checks a proposition that the arithmetic below it cannot falsify, and
its own documents describe the gap as one violation wide.

Recorded as a latent hazard rather than a defect because the underflow is the part
that could bite: a lapped ring passed to `check_ends` in a test binary panics
inside `ring_spsc` with no mention of `ring_debug` in the message, which is a bad
place for an investigation to start.

**The obvious repair does not survive contact with the ruling.** Bounding the two
readings — rejecting `pending > capacity` or `free > capacity` before summing them
— would catch the release-build wraparound and would cost four lines. It would
also be four lines no test in this family can reach: the corruption that produces
them cannot be constructed from a live `ring_core::Ring`, because nothing in the
family hands out a `CursorPair` and `ring_core`'s ends expose no `position()`.
`check_ends_cannot_see_the_corruption_check_can` has to assert on a `CursorPair`
for exactly that reason, and says so. Adding a guard that no test can enter is
adding the shape this corpus records as unfalsifiable in DB18, in the crate whose
own findings named it.

**Where the underflow actually lives is one crate down, and it is now counted.**
`ring_spsc`'s `self.ring.capacity().get() - self.occupancy() as usize` is one of
the eight unguarded subtractions
[`pattern/001`](../pattern/001_the_guard_that_makes_the_next_line_legal.md)'s
census lists. That is the line that wraps, that is where a `checked_sub` would
turn a panic-or-wrap into a value `check_ends` could report on, and it is not in
this crate. DB9's fix removed this crate's own, which is why none of the eight
are here; this one names the owner of another.

**Disposition:** declined — `check_ends` is not repaired here, because the readings
it receives are derived by `ring_spsc` as `capacity - occupancy` and saturated
occupancy, so no cursor corruption survives into them, and the guard that would
catch the wrapped case is unreachable from any live `ring_core::Ring` a test can
build. The gap stays recorded in
[`workaround/001`](../workaround/001_the_door_ring_core_does_not_open.md)'s W1,
and the subtraction that would have to change is `ring_spsc/src/lib.rs`'s, now
listed in `pattern/001`'s census rather than described in prose here.

**Correction (2026-09-28):** the finding above, and V7's row, describe
`ring_spsc::Producer::free_capacity` as it stood when written. It has since been
fixed (`Fix(free_capacity_underflow_on_a_precondition_violation)` in
`ring_spsc/src/lib.rs`) to read
`self.ring.capacity().get().saturating_sub( self.occupancy() as usize )`, the same
convention `ring_seqno::free_slots` already used. A D2 breach no longer underflows:
`free` saturates to `0`, so `pending + free` equals `occupancy` rather than
wrapping back to `capacity`, and `check_ends` now reports `ReadingsDisagree` on
that input instead of passing silently. V7's panic-or-wrap outcome, and the
`D2 lapped … ring_spsc` line in the Regenerate output above illustrating it,
describe the pre-fix crate — kept as the record rather than rewritten, per this
corpus's convention. V8 (a D1 breach, already saturating through `distance_to`)
is untouched by this fix and still passes silently, so the door DB35 names is
narrower than it was, not closed: this crate's own guard did not change, only the
one in `ring_spsc` it depends on.

**DB35's own "unreachable" claim is also narrower than it read.** It was scoped
to what a test can construct *from a live `ring_core::Ring`*, and stands as
written on that scope. `ring_spsc`'s own regression test for this fix
(`free_capacity_degrades_safely_even_when_a_precondition_violation_reaches_d2`,
`ring_spsc/tests/spsc_test.rs`) found D2 reachable a different way: not through
`ring_core`, but through `ring_spsc`'s own direct API under a documented
precondition violation (two racing `Producer` bit-copies), proved by `loom`
after a probabilistic stress test of millions of attempts failed to reproduce
it. That test's own Pitfall section names the lesson directly — treating an
absence of reproduction as unreachability "would have left this exact bug
undiscovered, exactly as `ring_debug`'s own DB35 reasoning left it."

### DB36 — the family has two implementations of "free slots" and they disagree exactly where it matters

`ring_seqno::free_slots` computes `capacity.saturating_sub( in_flight )`.
`ring_spsc::Producer::free_capacity` computes `capacity.get() − occupancy()`, plain.
They agree on every sound ring and diverge precisely on a lapped one: the first
returns zero, the second underflows.

The divergence is invisible from this crate, and one test walks straight into it.
`check_ends_cannot_see_the_corruption_check_can` is named for `check_ends`,
documents `check_ends`'s central limitation, and never calls it — it reimplements
the sum from `CursorPair::pending()` and `CursorPair::free_slots()`, which route
through `ring_seqno`, not through the `ring_spsc` accessors `check_ends` uses. The
suite calls `check_ends` in two tests, and this is not one of them.

For the D1 pair the test uses, the two arithmetics agree, so its conclusion is
correct. **The method is what is worth recording: a test that models the function
under test rather than calling it, in a crate whose entire subject is a derived
reading standing in for the thing it was derived from.** The model happens to be
faithful for the case chosen and is not faithful in general — which is the same
shape as the defect the crate exists to catch, one level up.
