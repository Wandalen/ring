# Workaround: The Diagnostic That Reimplements the Readings

### Scope

- **Purpose**: Record that `ring_debug` cannot use this crate's readings, show what it wrote instead, and name the coupling that substitution created.
- **Responsibility**: Establish why the bypass is necessary rather than careless, give the two guards in the order they must stay, and state the undocumented complement between `may_claim` and `check_seqs`.
- **In Scope**: `ring_debug::check_seqs` as a deliberate reimplementation.
- **Out of Scope**: The saturation decision itself — see [`decisions/002`](../decisions/002_saturating_rather_than_signed.md).

### The Bypass

```rust
// ring_debug/src/lib.rs:282-312
fn check_seqs( producer : Seq, consumer : Seq, capacity : Capacity ) -> Result< (), Violation >
{
  if consumer.0 > producer.0
  {
    return Err( Violation::ConsumerAheadOfProducer { producer, consumer } );
  }

  if producer.0 - consumer.0 > capacity.get() as u64
  {
    return Err
    (
      Violation::ProducerLappedConsumer { producer, consumer, capacity : capacity.get() }
    );
  }

  Ok( () )
}
```

Raw field access, raw comparison, raw subtraction. `ring_debug` reaches past
`ring_cursor` and `ring_types` to the `u64` inside `Seq`, and does not call into
`ring_seqno` at all — it does not even depend on it:

```sh
cd "$(git rev-parse --show-toplevel)"
# ring_debug reimplements the readings rather than depending on them
grep -c 'ring_seqno' ring_debug/Cargo.toml || true
# control — the identical expression over a manifest that does declare it
grep -c 'ring_seqno' ring_cursor/Cargo.toml
```

Live output:

```
0
1
```

Its four dependencies are `ring_core`, `ring_cursor`, `ring_types`,
`ring_atomic`. The manifest states the omission as deliberate, in a comment
beside the two it *does* take beyond its assigned edges:

> Reading the derived values instead is what this crate exists because you
> cannot do (`docs/pitfall/001`).

### Why It Has To

Because this crate's readings are total, and totality is achieved by erasing
exactly the two states a diagnostic is looking for. `ring_debug` says so itself:

> Reads both cursors once, `Acquire`, and compares them directly rather than
> through `ring_seqno` — whose saturating arithmetic is what makes the first of the
> two invisible.

Trace both violations through the readings:

| Violation | Condition | What `may_claim` says | What `free_slots` says | Visible? |
|-----------|-----------|-----------------------|------------------------|:--------:|
| `ConsumerAheadOfProducer` | `consumer > producer` | `true` — a healthy, empty ring | `capacity` — every slot free | ❌ |
| `ProducerLappedConsumer` | `producer − consumer > capacity` | `false` | `0` | ✅ |

`distance_to` saturates to `0` for a backward pair, so a corrupted ring where the
consumer has overtaken the producer produces *the identical reading as a brand-new
one*. There is no threshold to test and no value to inspect — the information is
gone before `ring_seqno` returns.

The second row is visible, so `check_seqs` could have used `free_slots( … ) == 0`
for it. It does not, and the reason is the first row: a function that must
hand-roll one of its two checks gains nothing by routing the other through a
dependency.

**This is a workaround around a decision that is correct.** Saturation is right
for the gates — see [`decisions/002`](../decisions/002_saturating_rather_than_signed.md)
— and wrong for a diagnostic, and one crate cannot be both. Recording it here
rather than as a pitfall, because nothing needs fixing.

### The Ordering Is Load-Bearing and Undocumented

```rust
if consumer.0 > producer.0 { return Err( … ); }   // guard
if producer.0 - consumer.0 > capacity.get() as u64 { … }   // depends on it
```

`producer.0 - consumer.0` is plain `u64` subtraction. It is total **only** because
the guard above already returned for every pair where `consumer > producer`.

| Change | Consequence |
|--------|-------------|
| Swap the two blocks | The subtraction runs on a backward pair: panic in a debug build, wrap to ~2⁶⁴ in a release build — which then reports `ProducerLappedConsumer` instead of `ConsumerAheadOfProducer` |
| Delete the first guard | Same, plus D1 becomes undetectable |
| Add a third check above them | Fine, provided it does not subtract |

Confirm the exposure with the workspace profile:

```sh
cd "$(git rev-parse --show-toplevel)"
# no override — debug panics, release wraps
grep 'overflow-checks' Cargo.toml \
  || echo '(no matches — no override in the workspace manifest)'
# control — the identical expression over a manifest that does set it
printf '[profile.release]\noverflow-checks = true\n' > /tmp/-ovf_control.toml
grep 'overflow-checks' /tmp/-ovf_control.toml
rm -f /tmp/-ovf_control.toml
```

Live output:

```
(no matches — no override in the workspace manifest)
overflow-checks = true
```

`distance_to` would have made the ordering irrelevant: `consumer.distance_to(
producer )` is saturating and total for any pair, so the two checks would commute.
Giving that up is the price of seeing D1 at all, and it is a fair price — but the
resulting coupling is currently held by nothing except the order of two statements.

**Nothing pins it.** No comment marks the guard as load-bearing, and no test
covers the swapped order (a test cannot, directly — but a `debug_assert!(
consumer.0 <= producer.0 )` between the two blocks would document the requirement
where it holds). Not applied here: it is a source change to another crate.

### An Undocumented Complement

The two crates draw the same boundary from opposite sides:

| Crate | Expression | Verdict at `d == capacity` | Verdict at `d == capacity + 1` |
|-------|------------|:--------------------------:|:------------------------------:|
| `ring_seqno::may_claim` | `d < capacity` | `false` — refuse the claim | `false` |
| `ring_debug::check_seqs` | `d > capacity` | `Ok` — legal, merely full | `ProducerLappedConsumer` |

They agree exactly, and the agreement is not an accident: `d == capacity` is a
ring that is *exactly full*, which is a legal state that no producer may add to.
`may_claim` refuses to extend it; `check_seqs` declines to call it corrupt. One
crate uses `<`, the other `>`, and between them they partition the range with no
gap and no overlap.

That is precisely the kind of relationship that decays silently. If `may_claim`'s
boundary were ever changed to `<=` — the off-by-one this crate's own vocabulary
calls the lap bug — `check_seqs` would keep passing, because a ring at `d ==
capacity + 1` is still not `> capacity + 1`. The gate would admit a claim that
overwrites a slot the consumer is reading, and the diagnostic built to catch
exactly that would report `Ok`.

Both sides are individually tested — `tests/seq_test.rs:57-67` for `may_claim`'s
boundary, `ring_debug`'s own suite for the violation — and **nothing tests them
together**. A single assertion in `ring_debug` would:

```rust
// not present
let d = capacity.get() as u64;
assert!( !ring_seqno::may_claim( Seq( d ), Seq( 0 ), capacity ) );
assert!( check_seqs( Seq( d ), Seq( 0 ), capacity ).is_ok() );
assert!( check_seqs( Seq( d + 1 ), Seq( 0 ), capacity ).is_err() );
```

Not added here: it belongs in `ring_debug/tests/`, and `ring_debug` does not
currently depend on `ring_seqno` at all — adding the assertion means adding the
dependency, which is a judgement about that crate's shape rather than this one's.

### What Would Delete the Workaround

| Change | Effect | Verdict |
|--------|--------|---------|
| Give `ring_seqno` a signed or `Option` reading | D1 becomes visible through the crate | Rejected — see [`decisions/002`](../decisions/002_saturating_rather_than_signed.md) § *The Alternatives* |
| Export a `ring_seqno::ordering( a, b ) -> Ordering` | D1 visible, saturation untouched | Plausible, and it is just `a.0.cmp( &b.0 )` — the wrapper earns nothing |
| Nothing | The bypass stays | ✅ Current, and correct |

The middle row is the honest assessment: a shared helper here would be one line
long and would save `ring_debug` one line. The workaround should stay; what is
missing is the *pin* on the ordering and the *cross-check* on the boundary, both
of which live in `ring_debug`.

### SQ51 — Two Guards Whose Order Is Load-Bearing

The coupling is invisible in the source and fatal to a reorder:

```
guard 1   rejects the backward pair
guard 2   subtracts, and underflows if the pair is backward

Reorder them and guard 2 panics on the input guard 1 exists to reject.
```

**Finding.** `check_seqs`'s two guards must stay in their current order or the second one panics — an undocumented coupling created by bypassing `distance_to`.

**Disposition:** declined — the fix is a `debug_assert!` (or a comment marking the order load-bearing) inside `check_seqs`, which lives in `ring_debug/src/lib.rs:282-312`. That file is outside this batch's assigned crates (`ring_seqno`, `ring_tls`, `ring_trace`); `ring_debug` does not even depend on `ring_seqno` today (see the dependency census above), so pinning the guard order there is a `ring_debug`-side change with its own verification run, not something this crate's own source or tests can express.

---

### SQ52 — One Boundary, Two Crates, Opposite Operators

The same threshold is written twice with complementary operators and no cross-reference:

```
ring_seqno   may_claim    distance <  capacity   -> may publish
ring_debug check_seqs   distance >  capacity   -> report a problem

No comment, doc link or test connects the two.
```

**Finding.** `may_claim`'s `<` and `check_seqs`'s `>` are exact complements around one boundary, in different crates, with nothing linking them.

---

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_saturating_rather_than_signed.md](../decisions/002_saturating_rather_than_signed.md) | The decision this works around, and why it is right anyway |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_every_reading_is_total.md](../invariant/002_every_reading_is_total.md) | Totality — bought by erasing what the diagnostic wants |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_capacity_readings.md](../item/001_the_three_capacity_readings.md) | `may_claim`'s side of the complement |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_one_pair_across_one_lap.md](../lifecycle/001_one_pair_across_one_lap.md) | The transition no reading detects, walked position by position |

### Workarounds

| File | Relationship |
|------|--------------|
| [002_laps_between_has_no_caller.md](002_laps_between_has_no_caller.md) | The same story from the other end — a reading nobody wanted |

### Sources

| File | Relationship |
|------|--------------|
| `ring_debug/src/lib.rs:282-312` | The reimplementation, with its load-bearing guard order |
| `ring_debug/Cargo.toml:5-15` | Four dependencies, `ring_seqno` deliberately not among them |
| `ring_debug/src/lib.rs:250-252` | The stated reason for bypassing `ring_seqno` |
| `ring_debug/src/lib.rs:134-144` | D1 described as the dangerous one |
| `ring_types/src/id.rs:70-85` | `distance_to` — total, saturating, and `const` |
| `ring_seqno/src/lib.rs:73-76` | `may_claim`'s `<`, the complement of `check_seqs`'s `>` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:57-67` | `may_claim` at the boundary — one half of an untested pair |
| `tests/seq_test.rs:106-110` | `pending( Seq( 2 ), Seq( 5 ) ) == 0` — the saturation that hides D1, asserted as intended behaviour |
