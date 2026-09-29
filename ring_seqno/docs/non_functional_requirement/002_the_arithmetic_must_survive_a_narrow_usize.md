# Non-Functional Requirement: The Arithmetic Must Survive a Narrow `usize`

### Scope

- **Purpose**: Record that the three capacity readings once stopped agreeing on any target where `usize` is narrower than 64 bits, how reachable that was, and the one-line change that removed it.
- **Responsibility**: Give the divergence with concrete inputs, show why no existing test could reach it, separate this hazard from the superficially similar `Seq` overflow, and record that the requirement is now met by construction rather than by testing.
- **In Scope**: Width-dependence of `ring_seqno`'s casts.
- **Out of Scope**: How a reader fails to notice it in review — see [`pitfall/002`](../pitfall/002_reading_free_slots_on_a_narrow_target.md).

### The Requirement

> `laps_between`, `may_claim` and `free_slots` are provably equivalent at the
> claim boundary ([`algorithm/001`](../algorithm/001_four_readings_of_one_subtraction.md)).
> They must remain so on every target the family builds for.

**The crate meets this today, and did not when this file was written.** The
divergence below was real: on a 64-bit host the three agreed, on a target with a
32-bit `usize` they did not. SQ37's fix removed the one narrowing that caused it,
so the equivalence now holds on every target by construction — it depends on no
test, no CI job and no target triple.

The argument is kept in its original shape because it is the reason the change
was made, and because the failure it describes is worth recognising elsewhere.
Every statement about the *live* code is written in the past.

### The Divergence

A `Seq` distance is `u64`. A `Capacity` is `usize`. The three readings used to
reconcile those two widths differently — the recipe below is what the fix
achieved, and shows all three widening:

```sh
cd "$(git rev-parse --show-toplevel)"
grep ' as u64\| as usize' ring_seqno/src/lib.rs
```

Live output:

```
  earlier.distance_to( later ) / capacity.get() as u64
  consumer.distance_to( producer ) < capacity.get() as u64
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
```

| Line | Function | Cast | Direction |
|-----:|----------|------|-----------|
| 52 | `laps_between` | `capacity.get() as u64` | `usize → u64`, **widening** — always exact |
| 75 | `may_claim` | `capacity.get() as u64` | `usize → u64`, **widening** — always exact |
| 98 | `free_slots` | `( … as u64 ).saturating_sub( … ) as usize` | both directions on one line — widen, saturate, then narrow the already-bounded result |

**Finding SQ37.** Two of the three lifted the small value up. The third pushed
the large value down. Row 98 above is the *post-fix* line: it widens first and
narrows a result that cannot exceed `capacity`, so all three now lift. What
follows is the case that argued for that change.

### A Worked Case

Target with `usize = u32`. Capacity 8. Consumer at `Seq( 0 )`, producer at
`Seq( 4_294_967_300 )` — that is `2³² + 4`. Against the **pre-fix** body,
`let in_flight = consumer.distance_to( producer ) as usize;`:

| Step | Value |
|------|-------|
| `consumer.distance_to( producer )` | `4_294_967_300` (`u64`, exact) |
| `… as usize` on a 32-bit target | `4` — the high bit is gone |
| `free_slots` = `8.saturating_sub( 4 )` | **`4`** |
| `may_claim` = `4_294_967_300 < 8` | **`false`** |
| `laps_between` = `4_294_967_300 / 8` | **`536_870_912`** |

Three readings of one state: *four slots free*, *no room at all*, and *half a
billion laps behind*.

The worst part is the first. `free_slots` did not return an obviously-broken
value like `0` or `usize::MAX` — it returned **4**, an ordinary mid-range answer
for a capacity-8 ring. Nothing in a log, a metric or a debugger would have looked
wrong.

And `free_slots` is the reading the family gates writes with — which is what made
the truncation worth removing rather than documenting:

| Caller | What it decides |
|--------|-----------------|
| `ring_batch:323` | `if free_slots( … ) < count { return Err( RingError::Full ) }` — would have passed, so the batch is claimed |
| `ring_gating:222` | `headroom` — would have reported four slots of room to `ring_claim`'s retry loop |
| `ring_cursor:368` | `CursorPair::free_slots`, which `ring_spsc` and `ring_wait` read |

So on such a target a producer more than 2³² publications ahead of its consumer
would have been told it may write, and would have written over slots that had not
been read. All three callers are unchanged; they read a `free_slots` that can no
longer lose the high bits.

### Reachability: This Is Not the `Seq` Overflow

The two hazards look alike — both are integer range problems on the same
quantity — and they are four orders of magnitude apart in how reachable they are.

| | `Seq` overflow (SQ44) | `usize` truncation (SQ37) |
|---|---|---|
| Threshold | 2⁶⁴ ≈ 1.8 × 10¹⁹ publications | 2³² ≈ 4.3 × 10⁹ publications |
| At 1 M publications/s | ~584,000 years | **~72 minutes** |
| At 10 M publications/s | ~58,000 years | **~7 minutes** |
| Verdict | unreachable | was reachable within one session |

A ring is a high-throughput structure; 2³² messages is what a busy one moves
before lunch. On a 32-bit target this was an operational hazard, not a theoretical
one — and that gap is the whole reason the two were not treated alike. SQ44 is
still carried as unreachable and untouched; SQ37 was fixed.

### Why No Test Reached It

This is the half that made the one-line fix the right answer rather than a test.
Two independent reasons, either of which alone was sufficient:

**1. The sweep's range is too small.** The one test pinning `free_slots` to
`may_claim` iterates:

```rust
// tests/seq_test.rs:77-80
for consumer in 0..8u64
{
  for producer in consumer..consumer + 20
  {
```

Maximum distance: 20. It would have needed to reach 4,294,967,296 — and cannot be
extended to, since that is 4.3 billion iterations.

**2. The host is 64-bit.** Even a test using the exact input above passes on
`x86_64`, where `usize` is `u64` and the cast was a no-op. The bug was invisible
on the machine the suite runs on.

The one test in this crate using large positions —
`positions_many_laps_apart_stay_comparable` — tops out at `Seq( 800 )`.

**The family's largest test position lands one short of the threshold, and
missed it anyway.** `ring_gating`'s `an_ungated_ring_has_a_full_capacity_of_headroom`
sweeps producers up to `u32::MAX as u64` — that is `4_294_967_295`, exactly one
below the 2³² where the truncation began:

```rust
// ring_gating/tests/gating_test.rs:196-202
let ungated = GatingSet::new( cap( 4 ), 0 );

assert_eq!( ungated.slowest(), None );
assert_eq!( ungated.limit(), None );
for producer in [ 0u64, 4, 1_000, u32::MAX as u64 ]
{
  assert_eq!( ungated.headroom( Seq( producer ) ), 4, "at producer {producer}" );
```

Two independent reasons it would not have caught the bug even at `2u64.pow( 32 )`:
the set is **empty**, so `headroom` returns `capacity.get()` from its `map_or`
without ever calling `free_slots`; and the host is 64-bit, so the cast was a no-op.
The one test in the family that reaches into the right numeric neighbourhood
takes a code path that skips the arithmetic entirely.

Both reasons are still true of the suite today — which is the point. The suite
did not change; the arithmetic did. Nothing added a test that would have failed
before the fix, because no such test is writable on a 64-bit host.

### What Would Have Caught It

| Approach | Cost | Catches it? | Taken? |
|----------|------|:-----------:|:------:|
| One assertion at `Seq( 2u64.pow( 32 ) + 4 )` in the existing sweep | one line | Only when *run on* a 32-bit target | no |
| `cargo check --target i686-unknown-linux-gnu` in CI | a toolchain target | No — it compiles fine; the cast is legal | no |
| `cargo test --target wasm32-*` or `i686-*` with the assertion above | a runner per target | **Yes** | no |
| Remove the narrowing instead: compute in `u64`, narrow only the result | one line, no CI change | **Yes**, by construction | **yes** |

The last was the real answer, and is the one taken. `free_slots`'s result is
bounded by `capacity`, which is a `usize` by definition, so narrowing at the end
is always exact:

```rust
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );              // stays u64
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize // narrow the result
}
```

That makes all three readings widen, identically, and the equivalence holds on
every target with no test and no CI job required. It is one line, and it **is**
what the crate now does — see the `Live output:` under SQ37 below, which is this
exact body read out of `src/lib.rs`.

### What Was at Risk

Honestly stated: **nothing, ever, in practice.** There is no `.cargo/config.toml`
in the workspace and no target configured anywhere, so everything builds for the
64-bit host, where the three readings agreed exactly even before the fix.

```sh
cd "$(git rev-parse --show-toplevel)"
ls .cargo/ 2>/dev/null || echo "no target configuration"
```

Live output:

```
no target configuration
```

It was recorded as a latent hazard, not a live defect: it would have become live
the first time anyone built the family for `wasm32`, `armv7`, `i686` or
`riscv32` — and `wasm32` is not far-fetched for a project of this kind. That was
the case for fixing it rather than filing it. The failure it produced was a
silent overwrite reported as a plausible-looking free count, which is close to
the hardest kind of bug to trace back to a cast; and the fix cost one line and no
CI target, so the cheapest correct move was to make the hazard unreachable
instead of waiting for a target that could observe it.

The `ls .cargo/` reading above is unchanged and still says *no target
configuration* — the point is that it no longer matters what it says.

### SQ37 — The Only Reading That Could Lose Information

The narrowing was one cast, and the two readings disagreed the moment it bit.
The line below is the **pre-fix** body, kept for the shape of the failure; the
`Live output:` further down is what `src/lib.rs` reads today:

```
free_slots   let in_flight = consumer.distance_to( producer ) as usize;   [ pre-fix ]
             on a 32-bit target, distance 2^32 + 4 truncates to 4
             so free_slots reports 4 while may_claim reports false
```

**Finding.** `free_slots` truncated on any target where `usize` is under 64 bits while `may_claim` and `laps_between` did not, and `free_slots` is the one two other crates gate writes with.

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^pub fn free_slots/,/^}$/p' ring_seqno/src/lib.rs
```

Live output:

```
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
}
```

**Disposition:** applied — took the "real answer" this file already names: `free_slots` now computes `in_flight` in `u64`, saturating-subtracts it from the widened capacity, and narrows only the already-bounded `usize`-sized result, so all three readings widen identically and the divergence this finding describes cannot occur on any target. Verified via `cargo test -p ring_seqno --all-features`, 2026-09-04 — ring_seqno's 11 unit tests plus 5 doctests all pass, including `free_slots_agrees_with_may_claim_across_two_laps` and `free_slots_spans_zero_to_capacity`, unchanged and still green against the new body. Now prints:
`let in_flight = consumer.distance_to( producer );`

---

### SQ38 — Seventy Minutes, Not Half a Million Years

The two overflow arguments in this crate's documentation are not the same argument:

```
2^32 publications at 1,000,000 msg/s   ~ 72 minutes    (free_slots truncation)
2^64 publications at 1,000,000 msg/s   ~ 584,000 years (Seq overflow)
```

**Finding.** The truncation was reachable in roughly seventy minutes at a modest publication rate, unlike the `Seq` overflow it superficially resembles — which is the whole reason the two were dispositioned differently.

```sh
cd "$(git rev-parse --show-toplevel)"
echo "distance_to(...) as usize occurrences: $( command grep -c 'distance_to( .*) as usize' ring_seqno/src/lib.rs || true )"
```

Live output:

```
distance_to(...) as usize occurrences: 0
```

**Disposition:** applied — same fix as SQ37 (`free_slots` widens, saturates, then narrows), which removes the reachable hazard rather than merely documenting how soon it would bite: the seventy-minute clock this finding measures has no truncation left to reach at the end of it, on any target width. Verified via `cargo test -p ring_seqno --all-features`, 2026-09-04 — ring_seqno's 11 unit tests plus 5 doctests all pass. Now prints:
`distance_to(...) as usize occurrences: 0`

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_four_readings_of_one_subtraction.md](../algorithm/001_four_readings_of_one_subtraction.md) | The equivalence that holds in arithmetic and once broke in `usize` |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_every_reading_is_total.md](../invariant/002_every_reading_is_total.md) | SQ44, the unreachable overflow this is not |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_capacity_readings.md](../item/001_the_three_capacity_readings.md) | The cast column, per function |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_every_reading_is_allocation_free.md](001_every_reading_is_allocation_free.md) | The requirement the crate does meet |

### Pitfalls

| File | Relationship |
|------|--------------|
| [002_reading_free_slots_on_a_narrow_target.md](../pitfall/002_reading_free_slots_on_a_narrow_target.md) | Why review did not catch it, and the redundant cast that would have hidden the fix |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:52, 75, 98` | The three casts — `free_slots` now widens, saturates, then narrows on one line |
| `ring_types/src/capacity.rs:60-63` | `Capacity::get` returning `usize` — the origin of the mismatch |
| `ring_types/src/id.rs:82-85` | `distance_to` returning `u64` — the other side |
| `ring_batch/src/lib.rs:323` | The write gate that would have passed |
| `ring_gating/src/lib.rs:222-228` | `headroom`, which would have reported the truncated figure onward |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:73-90` | The sweep, whose range is 20 and would need 2³² |
| `tests/seq_test.rs:136-148` | The largest positions used anywhere — `Seq( 800 )` |
| — | No test targets a narrow `usize`, and none could on a 64-bit host — which is why the fix is structural, not a test |
