# Pitfall: Reading `free_slots` on a Narrow Target

### Scope

- **Purpose**: Explain why the cast asymmetry in `free_slots` survived review, testing and CI for as long as it did, and what would have had to change for anyone to notice.
- **Responsibility**: Walk the four points at which someone could have caught it and show why each did not, and identify the construct that would have hidden the fix — still present, and still invisible to the one lint that ought to report it.
- **In Scope**: The reader's and reviewer's experience of the width hazard.
- **Out of Scope**: The arithmetic itself, worked with concrete values — see [`nfr/002`](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md).

**The narrowing described below is gone.** `free_slots` now keeps the distance in
`u64` and narrows only its already-`capacity`-bounded result, so the three
readings agree on every target by construction — the repair this file recorded
under *The One-Line Repair* is, its two explanatory comments aside, line for line
the one that was applied.

Two things outlive it and are why this file is kept rather than deleted. The
first is the walk itself: four independent checks that each had a specific,
nameable reason to look elsewhere is a description of how *any* width hazard
survives, not just this one. The second is **SQ45** — the redundant cast in
`ring_batch` is still there, and **SQ54** below shows the lint that should
report it does not. Statements about the *live* arithmetic are written in the
past; SQ45 and SQ54 are present tense because they are present facts.

### The Line

The body as it stood, and the two lines it turned on:

```rust
// ring_seqno/src/lib.rs:90-94  [ pre-fix ]
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer ) as usize;
  capacity.get().saturating_sub( in_flight )
}
```

`distance_to` returns `u64`. `as usize` truncated it on any target where `usize`
is under 64 bits. Its two sibling readings cast the other way — `capacity.get()
as u64`, at `src/lib.rs:52` and `:75` — and never truncated. The live body is at
`src/lib.rs:95-99` and is reproduced under *The One-Line Repair* below.

The consequence, worked out in
[`nfr/002`](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md):
on a 32-bit target a producer 2³² + 4 publications ahead of its consumer was
told **four slots are free**, while `may_claim` on the same state said no.

### Four Chances to Catch It, and Why Each Was Missed

**1. Reading the function.** The body was four lines and both statements were
readable at a glance. The cast sat on the line where the interesting thing
happens — the saturation — and the eye goes to `saturating_sub`, which is the
part with a comment-worthy design decision behind it. `as usize` sat before it
as punctuation.

The function also *looked* like the careful one of the three. It is still the
only one that saturates explicitly, and the crate's own test names that:

```rust
assert_eq!( free_slots( Seq( 100 ), Seq( 0 ), c ), 0, "saturates rather than wrapping" );
```

A reader who checked whether `free_slots` handled its edge cases found an
explicit, named, tested guard, and moved on. The guard was real. It was guarding
the wrong end — and it still is, which is why it survived the fix untouched.

**2. Running the tests.** They passed, on every machine anyone ran them on. The
sweep pinning `free_slots` to `may_claim` uses distances up to 20
(`seq_test.rs:77-80`), and the largest position anywhere in the suite is
`Seq( 800 )`. Neither could reach 2³² even if the host were 32-bit — the sweep
would need 4.3 billion iterations. **SQ46** below is this chance stated as a
finding; note that the suite is unchanged, because the repair was structural.

**3. Compiling for the target.** `cargo check --target i686-unknown-linux-gnu`
succeeded. `u64 as usize` is legal, intentional-looking, and warning-free; that
is what a cast *is* in Rust. `clippy::cast_possible_truncation` would have
flagged it, and it lives in the `pedantic` group, which the workspace does not
enable — the whole clippy table is still one line:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[workspace.lints.clippy\]/,/^\[[a-z]/p' Cargo.toml
# [workspace.lints.clippy]
# undocumented_unsafe_blocks = "deny"
```

Live output:

```
[workspace.lints.clippy]
undocumented_unsafe_blocks = "deny"
```

**4. Reviewing the diff that introduced it.** Nothing to see. Written on a
64-bit host, `usize` *is* `u64`, so the cast was genuinely a no-op there — the
generated code was identical with or without it. A reviewer testing their
understanding by deleting the cast found it did not compile, concluded it was
load-bearing, and restored it. Both conclusions were correct on that machine.

### The Construct That Would Have Hidden the Fix

```rust
// ring_batch/src/lib.rs:323
if ( free_slots( at, behind, capacity ) as usize ) < count
```

`free_slots` already returns `usize`. **Finding SQ45** — this cast is a no-op
today, and unlike everything above it, that sentence is still in the present
tense. The line is unchanged; the address is pinned by
[`pitfall/readme.md`](readme.md)'s own `### Regenerate` block, so it cannot go
stale silently.

It is harmless in isolation, and it is exactly the wrong construct to have here.
One clean repair for the hazard was to keep the arithmetic in `u64` and widen the
return type:

```rust
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> u64
```

Every other call site would then fail to compile and be fixed deliberately.
**This one would not.** `u64 as usize` is still legal, so `ring_batch:323` would
keep compiling — and the truncation the crate had just removed would reappear at
the call site, in a different crate, on the line that decides whether a batch may
be written.

That is not what happened, and the reason matters. The repair actually applied
kept the `-> usize` signature and narrowed the *result* instead (below), so no
call site had to change and this cast was never asked to absorb anything. The
construct was not defused; it simply never got its turn. It is still there, still
a no-op, and still loaded against the next person who reaches for the widening
repair — which is why SQ45 stays open rather than closing with the rest of this
file.

A redundant cast is not a style issue. It is a silenced type error waiting for
the type to change.

### Why It Was Recorded Before It Was Fixed

Nothing was broken at the time. There was no `.cargo/config.toml` and no target
configured anywhere in the workspace — there still is none — so every build was
for the 64-bit host, where all three readings agreed exactly. A defect that
cannot be observed on any machine the project builds on is easy to defer
forever, which is precisely why the case for writing it down had to be made
explicitly:

| | |
|---|---|
| The failure is a **silent overwrite** | A producer writes into slots a consumer has not read |
| The symptom is **data corruption at the consumer**, arbitrarily far from the cause | Nothing points back at a cast in a tier-1 arithmetic crate |
| The wrong value is **plausible** | `4` free slots in a capacity-8 ring, not `0` or `usize::MAX` |
| The trigger is **time, not input** | Any correct program reaches it by running long enough |

The last row is what separated this from an ordinary edge case, and it is the row
that eventually carried the argument. There was no malformed input to reject and
no unusual call to avoid — a completely correct producer and consumer, doing
exactly what they should, would arrive here after roughly seventy minutes at a
million messages a second. A hazard reachable by nothing but elapsed time cannot
be designed around at the call site, so it had to be removed at the source.

### The One-Line Repair, Now Applied

```rust
// ring_seqno/src/lib.rs:95-99
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );                // stays u64
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize   // narrow the result
}
```

The result is bounded by `capacity`, itself a `usize`, so the final narrowing is
always exact. All three readings now widen identically and the equivalence holds
on every target — with no new test, no CI target and no signature change, which
also means `ring_batch:323`'s redundant cast stays harmless for now.

Recorded before it was applied because `free_slots` is the reading three crates
gate writes with, and a change to it belonged to a run with its own verification.
That run happened; the body above is what `src/lib.rs` contains today, and
`nfr/002` carries the disposition. What did *not* change is the suite — no test
was added, because on a 64-bit host no test can tell the two bodies apart. That
is the same fact **SQ46** records, read from the other side.

### SQ45 — A Cast That Does Nothing, Until It Does

The cast is redundant against the signature `free_slots` has, and load-bearing against the one it might have had:

```
ring_batch/src/lib.rs:323
  if ( free_slots( at, behind, capacity ) as usize ) < count
                                          ^^^^^^^^
  free_slots already returns usize, so this compiles to nothing —
  and would keep compiling if free_slots were widened to u64.
```

**Finding.** `ring_batch:323` casts `free_slots( … ) as usize` when it already returns `usize` — a no-op that would silently absorb a widening repair. The repair taken narrowed the result instead, so this cast absorbed nothing and remains exactly as described; **SQ54** below is why no lint reports it.

---

### SQ46 — A Test That Passes on Both Targets for the Wrong Reason

The sweep is the only exhaustive test in the crate, and its domain missed the defect by nine orders of magnitude:

```
sweep domain:  consumer 0..8,  producer consumer..consumer+20
largest value tested: 27
smallest value at which usize and u64 diverge on a 32-bit target: 2^32
```

**Finding.** The sweep that would have caught the narrowing runs entirely inside values where `usize` and `u64` coincide, so it passed identically on a 32-bit target and the divergence at 2^32 had no test that could reach it. The sweep is unchanged — it could not be extended to cover the case, which is why the repair had to be structural rather than test-driven.

---

### SQ54 — The Lint That Should Report SQ45, and Does Not

`clippy::unnecessary_cast` is warn-by-default in the `complexity` group, so unlike
`cast_possible_truncation` it needs no `pedantic` opt-in and is already in force
here. It does not report `ring_batch:323`.

Measured by placing five redundant `as` casts in one throwaway test file in
`ring_batch` and running `cargo clippy -p ring_batch --all-targets
--all-features`:

```
flagged      local_args( 3, 4 ) as usize      local free fn, with arguments
flagged      local_usize() as usize           local free fn, no arguments
flagged      capacity.get() as usize          cross-crate inherent method
flagged      raw as usize                     local binding of a cross-crate value
NOT flagged  free_slots( … ) as usize         cross-crate free fn  (the :323 shape)
NOT flagged  pending( … ) as u64              cross-crate free fn, other crate item
```

The lint fires in this crate, in that file, on four shapes; the two it skips are
exactly the ones whose value came from a free function in another crate. The
skip is defensible in the abstract — a dependency may change its return type, so
the cast is arguably future-proofing rather than noise — but it means the one
construct this file names as dangerous is also the one construct the lint
declines to name.

**Finding.** `clippy::unnecessary_cast` is on by default and active in `ring_batch`, yet is silent on `ring_batch:323`: it skips redundant casts on values returned by free functions from other crates, while flagging the identical shape on local functions and on cross-crate inherent methods. SQ45 is therefore reachable by no automated check, only by reading.

---

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_capacity_readings.md](../item/001_the_three_capacity_readings.md) | The cast column, and `ring_batch`'s redundant one |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md](../non_functional_requirement/002_the_arithmetic_must_survive_a_narrow_usize.md) | The arithmetic, with the failing input and the reachability figures |

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_implementing_may_claim_with_laps_between.md](001_implementing_may_claim_with_laps_between.md) | The other hazard in these three functions |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_what_a_span_is_measured_in.md](../type/001_what_a_span_is_measured_in.md) | Why the three readings return three different types at all |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:98` | The narrowing, now applied to the already-`capacity`-bounded result |
| `ring_seqno/src/lib.rs:52, 75` | The two that widen |
| `ring_batch/src/lib.rs:323` | The redundant cast that would have absorbed a widening repair — SQ45 |
| `Cargo.toml` § `[workspace.lints]` | `clippy::pedantic` not enabled, so `cast_possible_truncation` is silent; `unnecessary_cast` needs no opt-in and is silent anyway — SQ54 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:101` | The saturation test — guarding the other end |
| `tests/seq_test.rs:73-90` | The sweep, range 20 |
| `tests/manual/readme.md` | No check covers target width |
