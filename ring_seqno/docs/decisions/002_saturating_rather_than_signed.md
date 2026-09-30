# Decision: Saturating Rather Than Signed

### Scope

- **Purpose**: Record why a backward pair of positions reads `0` rather than a negative number, an error, or a panic — and where that choice costs something.
- **Responsibility**: State the alternatives, give the argument, and identify the two readings for which the saturated value is a permissive answer rather than an inert one.
- **In Scope**: Direction handling across all four binary readings.
- **Out of Scope**: The `Option` from `slowest` — see [`001`](001_none_rather_than_zero_for_an_empty_set.md).

### The Decision

Every span in this crate comes from one method, and it saturates:

```rust
// ring_types/src/id.rs:82-85
pub const fn distance_to( self, later : Self ) -> u64
{
  later.0.saturating_sub( self.0 )
}
```

So `Seq( 10 ).distance_to( Seq( 4 ) )` is `0`, not `-6` and not an error. The
decision is made in `ring_types` and inherited wholesale here; `free_slots` then
saturates a second time on its own account (`( capacity.get() as u64 ).saturating_sub( … )`).

### The Argument

> Saturating rather than signed: the caller that needs the direction has already
> compared the two, and every caller that does not wants a count.

That is precise and it holds up. Check it against every call site in the family:

| Caller | Needs direction? | Why not |
|--------|:----------------:|---------|
| `ring_cursor:368` `free_slots` | ❌ | A producer behind its consumer is unreachable by construction |
| `ring_cursor:387` `pending` | ❌ | Same |
| `ring_cursor:417` `may_claim` | ❌ | Same |
| `ring_gating:221` `headroom` | ❌ | `slowest` is by definition at or behind the producer |
| `ring_batch:323` gate | ❌ | Compares against `count`, a positive quantity |
| `ring_consume:342` `pending` | ❌ | The barrier frontier is ahead of the position or equal |

Six sites, none of which wants a sign. A signed return would have forced all six
to handle a case none of them can act on — and the only honest handling of "the
consumer is ahead of the producer" is *panic or log*, which is a policy decision
that does not belong in an arithmetic crate.

### The Alternatives

| Option | Backward pair yields | Why not |
|--------|---------------------|---------|
| `i64` | `-6` | Six call sites gain an impossible branch; `free_slots` would need a cast back to `usize` anyway |
| `Option< u64 >` | `None` | Same, plus it collides with `slowest`'s `Option`, which means something entirely different |
| `Result< u64, _ >` | `Err` | Makes an arithmetic crate the place where a family-wide invariant violation is *reported*, which is `ring_debug`'s job |
| Panic | — | Turns a corrupted-state read into a crash inside a lock-free retry loop |
| Saturate ✅ | `0` | Total, branch-free, and the value is inert *for most readings* — see below |

### Where the Saturated Value Is Not Inert

**Finding, and the one real cost of the decision.** The four readings do not
degrade equally. Given a backward pair — a consumer somehow ahead of its producer
— they return:

| Reading | Value | Reads as | Effect on a caller acting on it |
|---------|-------|----------|---------------------------------|
| `pending` | `0` | "nothing to read" | **Inert** — the consumer waits |
| `laps_between` | `0` | "same lap" | **Inert** — no claim is refused on this basis |
| `may_claim` | `true` | "there is room" | **Permissive** — the producer writes |
| `free_slots` | `capacity` | "the ring is empty" | **Permissive** — a full batch is claimed |

The first two fail closed. The last two fail **open**.

That distinction is drawn nowhere in the source. `distance_to`'s doc argues for
saturation generally; `free_slots`'s doc says "never negative, because a producer
that appears to be behind its consumer is a state the family's monotonic
sequences exclude" — which is true, and is the reason the permissive answer is
acceptable, but does not note that the answer *is* permissive.

The manual plan's M3 reads the saturation as a safety property:

> The design choice is to return zero rather than a huge number, so a caller sees
> "nothing to do" instead of a plausible-looking figure.

"Nothing to do" is right for `pending` and `laps_between`. For `may_claim` and
`free_slots` the caller sees "go ahead" — the opposite of nothing to do. M3's
reasoning is correct for the two functions it names and does not extend to the
two it does not. See [`api/002`](../api/002_the_argument_order_split.md), where
the same asymmetry shows up as an argument-swap hazard.

### Why It Is Still the Right Decision

The permissive readings only appear in a state that cannot be reached through
correct use. Sequences are monotonic; a consumer passes its producer only if
memory is corrupted, an ordering is wrong, or a cursor is shared between two
rings. In each of those cases the ring is already broken and the arithmetic's
answer is not what saves anyone.

What matters is that the state is **detected somewhere**, and it is — outside
this crate, by a crate whose job is checking:

```rust
// ring_debug — a dedicated variant for exactly this
Violation::ConsumerAheadOfProducer
```

That is the right division. `ring_seqno` is a total function library on the hot
path; `ring_debug` is a checker that runs when someone asks. Putting the check
here would cost a branch in `ring_wait`'s spin predicate to catch a state that
implies the process is already unsound.

### The Double Saturation Has a Consequence Worth Naming

`free_slots` saturates twice — once in `distance_to`, once in `saturating_sub` —
and the two compose so that a corrupted ring reads *exactly* like a fresh one:

| Reading | Corrupted (consumer ahead) | Fresh (nothing published) |
|---------|---------------------------|---------------------------|
| `pending` | `0` | `0` |
| `free_slots` | `capacity` | `capacity` |
| `may_claim` | `true` | `true` |
| `laps_between` | `0` | `0` |

**All four readings agree, and all four are wrong, and there is no combination of
them that distinguishes the two states.** A caller cannot detect the corruption
by asking more questions — the information is gone at the first `saturating_sub`.

`ring_debug` reached the same conclusion independently, and says so in the
strongest terms available to it. Its D1 variant carries the warning in its own
doc comment:

> **The dangerous one.** The family's arithmetic saturates here, so this state
> reads as an empty, healthy ring and `may_claim` returns `true`.

And its `check` function documents *bypassing this crate* as a design
requirement rather than an implementation detail:

> Reads both cursors once, `Acquire`, and compares them directly rather than
> through `ring_seqno` — whose saturating arithmetic is what makes the first of the
> two invisible.

**The contrast with its sibling violation is the sharpest statement of the
cost.** `ring_debug` carries two single-observation violations, and only one of
them is invisible to the arithmetic:

| | D1 `ConsumerAheadOfProducer` | D2 `ProducerLappedConsumer` |
|---|---|---|
| What happened | consumer read past the producer | producer overwrote unread slots |
| Visible through `ring_seqno`? | **No** — every reading saturates to a healthy value | **Yes** — `pending` exceeds `capacity`, which no valid state can |
| `ring_debug`'s own words | "the one that reads as healthy and is therefore the one a reader has no other way to learn about" | "Less dangerous […] only because it leaves evidence" |
| Reported first when both hold | ✅ | — |

So saturation does not blind the arithmetic to every corruption — an over-full
ring still shows `pending > capacity` and is caught. It blinds it to exactly one
direction, the one where the saturating floor is `0`. That is the precise scope
of the cost, and it is why `check` reports D1 in preference to D2.

This is documented from the other side in
[`ring_cursor` `lifecycle/002`](../../../ring_cursor/docs/lifecycle/002_a_pair_across_a_full_lap.md),
and traced here in [`lifecycle/001`](../lifecycle/001_one_pair_across_one_lap.md).

### What a Signed Distance Would Have Changed

Nothing about the corruption case — an `i64` distance of `-6` is just as
undetectable to a caller that does not check the sign, and the six call sites
above would not have checked it. What it would have changed is the *swap* hazard:

```rust
free_slots( consumer, producer, capacity )   // arguments swapped
// saturating: capacity  — "empty ring", permissive
// signed:    capacity + 6 — larger than the ring, obviously wrong
```

A signed intermediate makes an argument swap produce an impossible value rather
than a plausible one. That is a real benefit and it is not the benefit the
decision was made for.

**The cheaper mitigation is types rather than signs.** `Producer( Seq )` and
`Consumer( Seq )` newtypes make every swap a compile error across all four
functions, keep the saturation, and cost a wrapper at each call site. That is a
family-wide question — it touches `ring_types` and every crate that names a `Seq`
— and is recorded here rather than proposed.

### SQ14 — Two Kinds of Wrong Answer

Saturation turns every backward pair into a defined answer, and the answers are not equally safe:

```
swapped may_claim   -> true   permissive: the caller publishes
swapped free_slots  -> capacity permissive: the caller publishes
swapped pending     -> 0       inert: the caller does nothing
swapped laps_between-> 0       permissive by equivalence (see SQ2)
```

**Finding.** Saturation makes a swapped `may_claim` or `free_slots` return a permissive answer while a swapped `pending` or `laps_between` returns an inert one — a distinction the source draws nowhere.

---

### SQ15 — The Test That Reassures About the Wrong Thing

The rationale and the equivalence one section up cannot both be right:

```
seq_test.rs:46-48
  /// A backward pair reads zero rather than an enormous number, so a caller that
  /// swapped its arguments gets an obviously-wrong answer instead of a plausible
  /// one.

but  laps_between == 0   <->   may_claim == true   (SQ2)
```

**Finding.** The test's rationale says a swapped caller "gets an obviously-wrong answer instead of a plausible one", but zero laps is exactly the value that reads as room to publish — it documents the permissive failure as though it were the safe one.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A5 'A backward pair reads zero rather than an enormous number' ring_seqno/tests/seq_test.rs
```

Live output:

```
/// A backward pair reads zero rather than an enormous number — but that zero
/// is not an obviously-wrong value: it is the same reading `may_claim` treats
/// as "room to publish" (see `docs/decisions/002_saturating_rather_than_signed.md`),
/// so a caller that swapped its arguments here gets a plausible, permissive
/// answer, not a visibly broken one.
#[test]
```

**Disposition:** applied — `laps_backward_read_zero`'s doc comment in
`tests/seq_test.rs` no longer claims the swapped-argument case is "obviously
wrong"; it now states the zero reads as the same permissive "room to
publish" value `may_claim` would give, matching SQ2's equivalence rather
than contradicting it. Now prints: `is not an obviously-wrong value: it is
the same reading`

---

### SQ16 — One Boundary, Two Integer Types

The pair the sweep test holds together used to be computed in two different widths:

```
may_claim   consumer.distance_to( producer ) < capacity.get() as u64      u64 space
free_slots  capacity.get().saturating_sub( … as usize )                   usize space  [ pre-fix ]
```

**Finding.** `may_claim` compared in `u64` and `free_slots` subtracted in `usize`, so the two readings that had to agree did their arithmetic in different types — the mechanism that made a narrow-target divergence possible at all.

`free_slots` now reads:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^pub fn free_slots/,/^}$/p' ring_seqno/src/lib.rs
```

Live output:

```
pub fn free_slots(producer: Seq, consumer: Seq, capacity: Capacity) -> usize {
    let in_flight = consumer.distance_to(producer);
    (capacity.get() as u64).saturating_sub(in_flight) as usize
}
```

**Disposition:** applied — `free_slots` now widens `capacity.get()` to `u64` before subtracting and narrows only the already-`capacity`-bounded result afterward, so both readings compute in `u64` space and the type mismatch this finding named is gone. Now prints: `(capacity.get() as u64).saturating_sub(in_flight) as usize`

---

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_argument_order_split.md](../api/002_the_argument_order_split.md) | The swap hazard the saturation masks |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_four_readings_of_one_subtraction.md](../algorithm/001_four_readings_of_one_subtraction.md) | The degenerate-case table, from the arithmetic side |

### Decisions

| File | Relationship |
|------|--------------|
| [001_none_rather_than_zero_for_an_empty_set.md](001_none_rather_than_zero_for_an_empty_set.md) | The other refusal to invent a value |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_every_reading_is_total.md](../invariant/002_every_reading_is_total.md) | Totality, of which saturation is the mechanism |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_one_pair_across_one_lap.md](../lifecycle/001_one_pair_across_one_lap.md) | The four readings tracked position by position, including past the boundary |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_shared_fold_that_declines_an_identity.md](../pattern/002_the_shared_fold_that_declines_an_identity.md) | The contrasting case — where the crate refuses to supply a value instead of saturating to one |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/id.rs:70-85` | The decision, and its argument |
| `ring_seqno/src/lib.rs:78-99` | `free_slots`'s second saturation |
| `ring_debug/src/lib.rs:134-144` | D1, and why it reads as healthy |
| `ring_debug/src/lib.rs:146-159` | D2, the contrast — visible through the arithmetic |
| `ring_debug/src/lib.rs:247-275` | `check`, which bypasses this crate deliberately |
| `ring_wait/src/lib.rs:241` | The spin predicate a check here would have cost |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:49-53` | A backward pair reads zero — the inert case |
| `tests/seq_test.rs:108` | `pending( Seq( 2 ), Seq( 5 ) )` is `0`, "a consumer cannot be ahead" |
| `tests/seq_test.rs:99` | `free_slots` saturates rather than wrapping |
| — | No test covers a backward pair through `may_claim` or `free_slots` — the two permissive readings |
