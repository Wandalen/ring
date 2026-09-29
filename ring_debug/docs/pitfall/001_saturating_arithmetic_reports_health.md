# Pitfall: Saturating Arithmetic Reports Health

### Scope

- **Purpose**: Record the measurement that gives this crate its reason to exist — that the family's cursor arithmetic does not merely fail to detect a corrupted cursor, but reports the healthiest state it can express.
- **Responsibility**: State what was measured, what the readings were, and why the two corruption modes degrade so differently.
- **In Scope**: `ring_seqno`'s three readings under both cursor invariant violations; the asymmetry between them.
- **Out of Scope**: The checks that catch these (→ [Cursor Invariants Over a Live Ring](../invariant/001_cursor_invariants_over_a_live_ring.md)); whether `ring_seqno` should change (it should not — see below).

### Abstract

**A ring whose consumer cursor has run ahead of its producer reports `free_slots
= capacity`, `pending = 0`, and `may_claim = true`.** Those are the exact
readings of a freshly-constructed, empty, healthy ring. Nothing in the family's
arithmetic distinguishes the two states, and the reading a producer acts on —
`may_claim` — says go ahead.

This is not a bug in `ring_seqno`. It is the documented, deliberate consequence of
a stated assumption, and this crate exists because assumptions of that shape
need something that checks them.

### The measurement

Three readings taken against a `CursorPair` of capacity 8, in each of three
states. Reproducible — the probe is six lines against `ring_cursor`'s public
surface:

```rust
let pair = CursorPair::new( Capacity::new( 8 ).unwrap() );
pair.producer().store( Seq( 3 ), Ordering::Release );
pair.consumer().store( Seq( 9 ), Ordering::Release );   // consumer ahead
println!( "{} {} {}", pair.free_slots(), pair.pending(), pair.may_claim() );
```

| State | `free_slots` | `pending` | `may_claim` | Distinguishable from healthy? |
|---|---:|---:|:---:|---|
| Healthy: producer 3, consumer 0 | 5 | 3 | `true` | — |
| **D1 violation**: producer 3, consumer 9 | **8** | **0** | **`true`** | **No.** Identical to an empty ring |
| **D2 violation**: producer 30, consumer 0 | 0 | **30** | `false` | **Yes.** `pending` exceeds capacity |

### The two corruptions degrade in opposite directions

**D1 — the consumer ahead of the producer — is invisible and unsafe.** All three
readings improve. `free_slots` rises to its maximum, `pending` falls to zero,
and `may_claim` stays `true`. A producer consulting the ring is told there is
room for a full ring's worth of records, so it publishes into slots the consumer
has already claimed to have read — and the reading that would have stopped it is
the one that saturated.

**D2 — the producer more than a lap ahead — is visible and safe.** `may_claim`
returns `false`, which is the conservative answer, and `pending` returns 30
against a capacity of 8 — a reading that cannot occur in any valid state and is
therefore self-evidently corrupt to anyone who checks it. Nothing checks it, but
the information survives.

**The asymmetry comes from two lines, not one — `distance_to`'s own clamp, and a
second inside `ring_seqno::free_slots`.** `Seq::distance_to` is a `saturating_sub`:

```rust
pub const fn distance_to( self, later : Self ) -> u64
{
  later.0.saturating_sub( self.0 )
}
```

D1 is exactly the case where this subtraction would go negative, so D1 is
exactly the case where `distance_to` itself saturates — and saturating to zero
means "no distance," which every caller reads as "nothing in flight." D2 does
not saturate here: `distance_to` returns its true, large distance, and that raw
value is what survives unclamped into `pending`'s reading. But `free_slots` does
not stop at `distance_to` — `ring_seqno::free_slots` wraps the result in a second
`saturating_sub`, and that second clamp is the one D2 hits: it is what turns
D2's large distance into `free_slots = 0`, the reading of a legitimately full
ring, rather than a reading no clamp touched at all.

### Why `ring_seqno` is right and should not change

`free_slots`'s own documentation states the assumption plainly:

> "never negative, because a producer that appears to be behind its consumer is
> a state the family's monotonic sequences exclude."

That is correct. Sequences are monotonic; the state is excluded; and inside that
assumption `saturating_sub` is the right operation, because it is branch-free on
the hot path and the branch it removes can never be taken.

**Making the arithmetic defensive would be the wrong fix.** A `checked_sub`
returning `Option` puts a branch and an error path on the gating read that
`ring_gating` and `ring_consume` perform on every claim that reaches them —
neither of the family's two in-house backends' own claim path does — to handle
a state that a correct program never reaches, paying a cost on the crates that
are hot for the benefit of a diagnostic. The right shape is what exists: fast
arithmetic that assumes the invariant, and a separate, opt-in checker that
verifies the assumption when someone wants it verified.

**This crate is that checker, and this pitfall is the argument for it.** Without
the measurement above, "runtime invariant checks over a live ring" reads as
belt-and-braces — checking something the type system already ensures. What the
measurement shows is that the failure is silent, safe-looking, and lands on the
one reading a producer acts on.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| P1 | A cursor is corrupted into D1 by any means | **Nothing.** The ring reads as empty and healthy; the producer overwrites unread slots |
| P2 | A cursor is corrupted into D2 | `pending` exceeds capacity. Detectable by anyone who compares them, and nobody does |
| P3 | A cursor is reset to zero mid-run | Both cursors monotonic-looking at any single observation; only a comparison against a *previous* observation shows it (→ [`lifecycle/001`](../lifecycle/001_from_one_observation_to_a_sequence.md)) |
| P4 | Someone "fixes" this by making `distance_to` checked | `ring_gating` and `ring_consume`'s gating reads acquire a branch and an error path, for a state a correct program never reaches — neither in-house backend's own claim path reaches this line at all |

**P3 is the one no stateless check can reach**, and it is the reason this crate
has a stateful half at all. A cursor pair at (0, 0) is perfectly valid. A cursor
pair that was at (500, 480) a moment ago and is now at (0, 0) is corrupt, and
the two are the same reading.

**P4 is the pitfall aimed at a reader of this instance rather than at the
family.** Having seen that the arithmetic hides a corruption, the obvious
response is to stop it hiding. That trade is bad in the direction it is usually
made — a cost on every claim, for a benefit on a path that should never execute.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- crates whose manifest names ring_seqno --'
# Cargo.toml (the workspace root) is not a member crate's manifest — excluded
# so this stays a census of crates, not of the workspace itself.
printf '  %s\n' "$( command grep -rl 'ring_seqno' --include=Cargo.toml | command grep -v '^Cargo\.toml$' | sed 's|ring/||;s|/Cargo.toml||' | sort | tr '\n' ' ' )"
printf '  ring_spsc among them: %s\n' "$( command grep -c 'ring_seqno' ring_spsc/Cargo.toml || true )"
printf '  ring_mpsc among them: %s\n' "$( command grep -c 'ring_seqno' ring_mpsc/Cargo.toml || true )"
echo '-- non-test call sites of the readings, in src/ only --'
command grep -r 'free_slots(\|ring_seqno::pending(\|may_claim(' --include=*.rs */src | command grep -v '^ring_seqno/' | command grep -v '///' | command grep -v '^ring_cursor/' | sed 's|^ring/||' | cut -c1-100 | sed 's/^/  /'
echo '-- what the two in-house backends compute instead --'
for f in spsc mpsc ; do
  printf '  %s free_capacity: %s\n' "$f" "$( awk '/pub fn free_capacity/{ f = 1 ; next } f && NF && !/^ *\{$/ { l = $0 ; sub( /^ */, "", l ) ; print l ; exit }' ring_$f/src/lib.rs )"
done
echo '-- clamps on the free_slots path --'
command grep -r 'saturating_sub' ring_seqno/src/lib.rs ring_types/src/id.rs | sed 's|^ring/||' | sed 's/^/  /'
echo '-- and the three readings each clamp produces --'
printf '  D1 producer 3 consumer 9 cap 8: distance_to saturates 3-9 -> 0, then 8-0 -> free 8\n'
printf '  D2 producer 30 consumer 0 cap 8: distance_to gives 30,     then 8-30 -> free 0\n'
```

Live output:

```
-- crates whose manifest names ring_seqno --
  ring_batch ring_consume ring_cursor ring_gating ring_seqno 
  ring_spsc among them: 0
  ring_mpsc among them: 0
-- non-test call sites of the readings, in src/ only --
  ring_batch/src/lib.rs:  if ( free_slots( at, behind, capacity ) as usize ) < count
  ring_consume/src/lib.rs:      .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );
  ring_debug/src/lib.rs://! assert_eq!( pair.free_slots(), 8 );
  ring_debug/src/lib.rs://! assert!( pair.may_claim() );
  ring_gating/src/lib.rs:      ring_seqno::free_slots( producer, slowest, self.capacity )
  ring_shutdown/src/lib.rs:    pair.may_claim()
  ring_wait/src/lib.rs:  wait_until( kind, spins, || pair.may_claim() ).map_err( | _ | RingError::Full
-- what the two in-house backends compute instead --
  spsc free_capacity: self.ring.capacity().get().saturating_sub( self.occupancy() as usize )
  mpsc free_capacity: self.claimer.headroom()
-- clamps on the free_slots path --
  ring_seqno/src/lib.rs:  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
  ring_types/src/id.rs:    later.0.saturating_sub( self.0 )
-- and the three readings each clamp produces --
  D1 producer 3 consumer 9 cap 8: distance_to saturates 3-9 -> 0, then 8-0 -> free 8
  D2 producer 30 consumer 0 cap 8: distance_to gives 30,     then 8-30 -> free 0
```

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_cursor_invariants_over_a_live_ring.md](../invariant/001_cursor_invariants_over_a_live_ring.md) | D1 and D2 as standing restrictions rather than as measurements |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_one_observation_to_a_sequence.md](../lifecycle/001_from_one_observation_to_a_sequence.md) | P3 — the corruption that needs a previous observation to see |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_seqno/src/lib.rs`](../../../ring_seqno/src/lib.rs) | `free_slots`, `pending`, `may_claim` — the three readings measured above, and the stated assumption |
| [`ring_types/src/id.rs`](../../../ring_types/src/id.rs) | `Seq::distance_to`'s `saturating_sub` — the one line the asymmetry comes from |

### Tests

| File | Relationship |
|------|--------------|
| `tests/debug_test.rs` | The measurement, re-taken as an assertion — `the_arithmetic_reports_an_empty_ring_for_a_consumer_ahead_cursor` pins all three readings for D1, so a future change to `distance_to` that fixed the masking would fail here and be noticed rather than silently making this crate redundant |

### DB49 — the cost P4 defends against sits on a path neither ring backend takes

P4 rejects making `Seq::distance_to` a `checked_sub` because it would put *"a
branch and an error path on the gating read that every claim performs."*

Measured against the manifests: `ring_seqno` is named by five crates besides
itself — `ring_atomic`, `ring_batch`, `ring_consume`, `ring_cursor`,
`ring_gating` — and **neither of the family's two in-house ring backends is among
them.** `ring_spsc::Producer::free_capacity` is
`self.ring.capacity().get() - self.occupancy() as usize`;
`ring_mpsc::Producer::free_capacity` is `self.claimer.headroom()`. Neither reaches
`ring_seqno::free_slots`, so neither reaches `distance_to` through it. The claim
path of every ring `check_ends` can be handed does not execute the line P4
defends.

The argument is not empty — `ring_gating` gates on `free_slots` and `ring_consume`
reads `pending`, and both are hot. But "every claim" names the backends, and the
backends compute their own headroom in crates with no `ring_seqno` edge at all.

**The conclusion survives; the evidence offered for it points at the wrong
crates.** The stronger version is one measurement further on: `ring_spsc` reaches
the same answer with a raw `-`, which under this workspace's profiles wraps rather
than saturating (→ DB10, DB35). A `checked_sub` in `ring_seqno` would not have
protected the path P4 is worried about, because that path was never in `ring_seqno`.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F "neither of the family's two in-house backends' own claim path does" ring_debug/docs/pitfall/001_saturating_arithmetic_reports_health.md
command grep -m1 -F '| P4 |' ring_debug/docs/pitfall/001_saturating_arithmetic_reports_health.md
```

Live output:

```
neither of the family's two in-house backends' own claim path does — to handle
| P4 | Someone "fixes" this by making `distance_to` checked | `ring_gating` and `ring_consume`'s gating reads acquire a branch and an error path, for a state a correct program never reaches — neither in-house backend's own claim path reaches this line at all |
```

**Disposition:** applied — both the "Making the arithmetic defensive" paragraph
and the Failure table's P4 row no longer attribute the branch-and-error-path cost
to "every claim"; they now name `ring_gating` and `ring_consume` as the crates
that actually reach this line and state plainly that neither in-house backend's
own claim path does. Now prints: `neither of the family's two in-house backends' own claim path does`

**Correction (2026-09-28):** the "stronger version" above, and the quoted
`self.ring.capacity().get() - self.occupancy() as usize` for
`ring_spsc::Producer::free_capacity`, describe the crate as it stood when
written. `ring_spsc` has since fixed that line
(`Fix(free_capacity_underflow_on_a_precondition_violation)` in
`ring_spsc/src/lib.rs`) to `saturating_sub`, so it no longer "reaches the same
answer with a raw `-`" — it now matches `ring_seqno`'s own convention instead of
contrasting with it. The Regenerate output above is kept current; this paragraph
is the one that is now historical.
[`invariant/002`](../invariant/002_the_conservation_law_and_why_it_holds.md)'s
DB35 correction and
[`integration/001`](../integration/001_reaching_the_cursors_of_a_live_ring.md)'s
DB29 correction record the consequence for `check_ends`. DB49's own conclusion —
that P4's cost lands on `ring_gating` and `ring_consume`, not on either in-house
backend's own claim path — is unaffected, since it never depended on which
operator `free_capacity` used.

### DB50 — the asymmetry attributed to one line in another crate is half-produced here

*"The asymmetry comes from one line,"* and the line quoted is `Seq::distance_to`'s
`saturating_sub`, in `ring_types`. There is a second `saturating_sub` on the same
path, in `ring_seqno::free_slots` itself:

```rust
let in_flight = consumer.distance_to( producer );
( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
```

The two clamps fire in different corruptions, and never together:

- **D1** (producer 3, consumer 9): `distance_to` saturates — `3.saturating_sub( 9 )`
  is 0 — and the second clamp passes it through unchanged, `8 - 0`. `free_slots`
  reads its maximum.
- **D2** (producer 30, consumer 0): `distance_to` does not saturate; it returns 30.
  **The second clamp is the one that fires** — `8.saturating_sub( 30 )` is 0 — and
  `free_slots` reads zero, which is the reading of a legitimately full ring.

So the table above is right that D2 is distinguishable, and right that `pending` is
what distinguishes it. What it does not say is that the other two readings are not:
`free_slots = 0` and `may_claim = false` are exactly what a full ring reports, and
they are produced by a clamp inside the crate the section *"Why `ring_seqno` is right
and should not change"* is defending.

That section's verdict is unaffected — the clamp is correct under the stated
assumption, same as the first. The finding is that a reader is told the masking
lives in one line in `ring_types` and can stop looking, when half of it is one
function call away in the crate they were just reading about.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F 'and that second clamp is the one D2 hits: it is what turns' ring_debug/docs/pitfall/001_saturating_arithmetic_reports_health.md
```

Live output:

```
`saturating_sub`, and that second clamp is the one D2 hits: it is what turns
```

**Disposition:** applied — the "asymmetry comes from one line" claim is
corrected to name both clamps: `Seq::distance_to`'s own `saturating_sub`, and
the second `saturating_sub` inside `ring_seqno::free_slots` itself; the paragraph
no longer states D2's distance is untouched by any clamp, and instead traces
D2's large distance through to the second clamp that turns it into
`free_slots = 0`. Now prints: `and that second clamp is the one D2 hits: it is what turns`
