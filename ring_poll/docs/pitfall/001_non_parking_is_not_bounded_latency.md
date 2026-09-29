# Pitfall: Non-Parking Is Not Bounded Latency

### Scope

- **Purpose**: Record that `Budget::new( 1_000_000 )` is a perfectly legal, entirely non-parking way to miss your frame.
- **Responsibility**: The misreading of this crate's non-parking rule that produces it, the symptom it presents as, and why the surface makes the choice visible instead of preventing it.
- **In Scope**: Budget magnitude as a latency decision, and where that decision is recorded.
- **Out of Scope**: The relationship between budget and batch size, a different over-reading of the same parameter (→ [`002_a_bigger_budget_is_not_a_bigger_batch.md`](002_a_bigger_budget_is_not_a_bigger_batch.md)).

### Trap

This crate's non-parking rule is about deadlock. Its whole argument is that a parking push "works
under light load and deadlocks the first time the ring is genuinely full during
a tick". Everything this crate does follows from taking that seriously.

The trap is reading the conclusion as *"so anything non-parking is safe on the
tick path"*. It does not follow. The helpers here are non-parking by
construction and their cost is set by an argument the caller passes:

| Call | Parks? | Worst case on a full ring |
|---|---|---|
| `push_within( .., Budget::once() )` | No | 1 ring operation |
| `push_within( .., Budget::new( 64 ) )` | No | 64 ring operations, a few microseconds |
| `push_within( .., Budget::new( 1_000_000 ) )` | No | ~400 ms of pure spinning, on a 16 ms frame |

Every row is compliant with the non-parking rule. The third row drops the frame — and
drops the next twenty-four with it.

That last figure is derived, not guessed: P3 measured 20 000 attempts at 0.008 s,
so an attempt costs about 0.4 µs and a million cost about 0.4 s. This row read
*~10 ms* until it was checked against the crate's own measurement → PL42.

### Failure

**The symptom is not a hang.** It is a frame-time spike that correlates with
ring saturation and disappears under light load — the mirror image of the
parking bug, which appears only under saturation and never goes away.

```text
# the shape to look for: budgets above 1 on a tick-path call site
grep -rn 'Budget::new' --include='*.rs' .
```

Every hit is a latency decision someone made. That is not a bug list; it is the
list of places where the answer needs to be *deliberate*, and the point of
grepping it is that `Budget::once()` never shows up in it.

Run across the family today it returns seventeen hits, every one of them inside
this crate — four in `src` doctests and thirteen in the suite. No caller has ever
made the decision this pitfall is about → PL41.

### Mitigation

**There is no ceiling this crate could pick.** The right maximum depends on the
caller's frame budget, how many rings the system touches, and what else runs in
the tick — none of which is knowable here. A hard cap would be a number invented
in the wrong crate, and it would be wrong for someone.

So the surface makes the choice *visible* instead:

- `Budget::once()` is the default, and `Tick::default()` uses it.
- `Budget::new` is the only way to get anything larger, and it takes the number
  explicitly — there is no `Budget::aggressive()` or `Budget::patient()` hiding a
  magnitude behind a word.
- [`../api/001`](../api/001_tick_path_surface.md)'s surface table carries a
  "who chose it" column, and every budgeted row says **the caller**.

#### What would actually fix it

Nothing in this crate. The measurement that would let a caller pick a number
honestly — how long a spin attempt costs on the target machine, and how that
scales with contention — belongs to `ring_bench`. Until then the number is
a guess, and the useful thing this crate can do is make sure it is a *stated*
guess rather than a default someone inherited.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
A=$( awk '/fn a_large_budget_spins/{f=1} f&&/^\}$/{exit} f' ring_poll/tests/poll_test.rs | command grep -oE 'Budget::new\( [0-9_]+ \)' | tr -cd '0-9' )
S=$( command grep -oE '0\.[0-9]+ s, including test setup' ring_poll/tests/manual/readme.md | command grep -oE '0\.[0-9]+' )
printf 'the whole Budget surface:    %s\n' "$( awk '/^impl Budget$/{f=1} f&&/^\}$/{exit} f' ring_poll/src/lib.rs | command grep -oE 'pub const fn [a-z_]+' | sed 's/pub const fn //' | tr '\n' ' ' )"
printf 'named-magnitude constructors:%s\n' "$( awk '/^impl Budget$/{f=1} f&&/^\}$/{exit} f' ring_poll/src/lib.rs | command grep -cE 'fn (aggressive|patient|small|large|fast|slow)' || true )"
printf 'any ceiling in the crate:    %s\n' "$( command grep -cE 'MAX_ATTEMPTS|max_attempts' ring_poll/src/lib.rs || true )"
printf 'Budget::new sites, family:   %s\n' "$( command grep -rn 'Budget::new' --include='*.rs' . 2>/dev/null | wc -l )"
printf 'in files:                    %s\n' "$( command grep -rln 'Budget::new' --include='*.rs' . 2>/dev/null | sed 's|^\./||' | tr '\n' ' ' )"
printf 'crates depending on ring_poll:%s\n' "$( for f in ring_*/Cargo.toml; do [ "$f" = ring_poll/Cargo.toml ] && continue; command grep -q 'ring_poll' "$f" && printf ' %s' "${f%/Cargo.toml}"; done | wc -w )"
printf 'P3 attempts, measured cost:  %s attempts, %s s\n' "$A" "$S"
printf 'implied per-attempt cost:    %s\n' "$( awk -v a="$A" -v s="$S" 'BEGIN{ printf "%.2f us", ( s / a ) * 1000000 }' )"
printf 'so 1e6 attempts cost about:  %s\n' "$( awk -v a="$A" -v s="$S" 'BEGIN{ printf "%.0f ms", ( s / a ) * 1000000000 }' )"
printf 'the trap table row says:     %s\n' "$( awk -F'\\|' '/^\| .push_within\( \.\., Budget::new\( 1_000_000 \) \)/ { gsub( /^ +| +$/, "", $4 ); print $4 ; exit }' ring_poll/docs/pitfall/001_non_parking_is_not_bounded_latency.md )"
printf 'in 16 ms frames, that is:    %s\n' "$( awk -v a="$A" -v s="$S" 'BEGIN{ printf "%.0f frames", ( ( s / a ) * 1000000000 ) / 16 }' )"
```

Live output:

```
the whole Budget surface:    once new attempts 
named-magnitude constructors:0
any ceiling in the crate:    0
Budget::new sites, family:   19
in files:                    ring_poll/tests/poll_test.rs ring_poll/src/lib.rs 
crates depending on ring_poll:0
P3 attempts, measured cost:  20000 attempts, 0.008 s
implied per-attempt cost:    0.40 us
so 1e6 attempts cost about:  400 ms
the trap table row says:     ~400 ms of pure spinning, on a 16 ms frame
in 16 ms frames, that is:    25 frames
```

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_tick_path_surface.md`](../api/001_tick_path_surface.md) | The "who chose it" column, which is the mitigation |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_a_budget_bounds_attempts_not_time.md`](../invariant/002_a_budget_bounds_attempts_not_time.md) | The property this pitfall is the underside of |
| [`../invariant/001_no_parking_operation_on_the_tick_path.md`](../invariant/001_no_parking_operation_on_the_tick_path.md) | The guarantee whose over-reading produces the trap |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`002_a_bigger_budget_is_not_a_bigger_batch.md`](002_a_bigger_budget_is_not_a_bigger_batch.md) | The other way a budget's meaning gets over-read |

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_budget_clamps_to_one.md`](../type/001_budget_clamps_to_one.md) | Why the constructor rules out zero but cannot rule out a million |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `Budget::once` and `Budget::new` — the two constructors, and the absence of any named-magnitude third |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `a_large_budget_spins_rather_than_sleeping` demonstrates the trap's cost directly: 20 000 attempts, bounded at 500 ms, entirely non-parking |

### PL41 — the mitigation is a grep, and today it returns only this crate's own fixtures

The Mitigation section is honest that no ceiling can be picked here, and settles
on making the choice visible instead: one default, one explicit constructor, no
named magnitudes, and a *who chose it* column in the API surface that reads **the
caller** on every budgeted row. The Failure section turns that into an action —
grep for `Budget::new`, and treat every hit as a latency decision someone made.

Run over the family, that grep returns seventeen hits in two files, both in this
crate: four in `src/lib.rs` doctests and thirteen in `tests/poll_test.rs`. No
other crate names `Budget::new`, and no other crate depends on `ring_poll` at
all.

So every element of the mitigation is in place and none of it has been exercised.
The *who chose it* column attributes to a caller that does not exist yet; the
grep-the-call-sites procedure has never been run against a call site; and the
choice the pitfall says must be deliberate has so far only ever been made by test
fixtures choosing numbers that make assertions convenient — `Budget::new( 3 )`
because two refusals are wanted, `Budget::new( 20_000 )` because a timing bound
needs headroom.

This is the ordinary state of a crate written before its consumers, and it is
worth recording for one reason: the mitigation's whole strategy is to shift a
decision onto the caller, and the strategy is untested precisely where it does
its work. The first real call site is the moment this document either works or
does not, and nothing currently distinguishes those two outcomes.

### PL42 — the warning understated the danger it warns about, by a factor of forty

The Trap table's third row is the one that carries the argument: a budget of a
million is fully compliant with the non-parking rule and still misses the frame. Until it
was checked against this crate's own measurement, that row said **~10 ms of pure
spinning, on a 16 ms frame**.

Ten milliseconds on a sixteen millisecond frame is bad and survivable. It reads
as *this would hurt* — a tight frame, dropped occasionally, under load.

The crate's own number says otherwise. P3 measured 20 000 attempts at 0.008 s,
which is 0.4 µs per attempt, so a million attempts cost about 400 ms — twenty-five
frames, not two-thirds of one. The row understated its own trap by forty times,
in the direction that makes the trap look affordable.

The likely origin is a plausible first-principles estimate: a `spin_loop` hint is
often quoted at around ten nanoseconds, and a million of those is ten
milliseconds. What that arithmetic leaves out is that an attempt is not a pause —
it is a `try_push` against a shared ring, then the pause. The measured figure
includes both, which is why it is forty times larger and why it is the one that
answers the question the table asks.

The number has been corrected and is now derived in the Regenerate block rather
than typed, so the two cannot diverge again. The finding is not the arithmetic
slip. It is that a document whose entire purpose is to make a cost vivid carried,
for as long as it existed, a figure that made the cost look survivable — while
the crate's own manual plan held the measurement that contradicts it, one
directory away, and nothing connected them.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F "~400 ms of pure spinning, on a 16 ms frame" ring_poll/docs/pitfall/001_non_parking_is_not_bounded_latency.md
```

Live output:

```
| `push_within( .., Budget::new( 1_000_000 ) )` | No | ~400 ms of pure spinning, on a 16 ms frame |
```

**Disposition:** applied — the Trap table's third row above now carries the
crate's own measured figure (~400 ms) rather than the unchecked ~10 ms
estimate, and the Regenerate block now derives that figure live from P3's
measurement so the two cannot diverge again. Now prints: `~400 ms of pure spinning, on a 16 ms frame`
