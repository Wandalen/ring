# Pitfall: A Clamp With No Way to Detect It

### Scope

**Purpose:** Record what the two clamps cost a caller who did not intend the
value they got, and place them against the family's own stated position on
silent correction.

**Responsibility:** `with_batch`'s clamp and the error variant named for exactly
its condition, the principle `ring_types` states two variants further down, and
the family's three different answers to a zero.

**In Scope:** `ring_config/src/lib.rs:112-125`, `:128-144`;
`ring_types/src/error.rs:63-79`; `ring_types/src/capacity.rs:44`;
`ring_wait/src/lib.rs:148-149`, `:183`.

**Out of Scope:** Why the setters are infallible at all is
[`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md).
The ranges the clamps guarantee are
[`invariant/001`](../invariant/001_two_ranges_that_cannot_be_violated.md).

---

## What the Family Does With the Same Condition

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what with_batch clamps, and what it says about clamping it --'
sed -n '/^  \/\/\/ Set the batch size, clamped to at least `1` and at most the capacity —$/,/^  \/\/\/$/p;/^  pub const fn with_batch( mut self, batch : usize ) -> Self$/,/^    self$/p' ring_config/src/lib.rs
echo '  -- the error variant named for exactly that condition --'
command grep -m1 -A8 -F '  /// A batch of the requested length cannot be served — the ring'"'"'s whole' ring_types/src/error.rs
echo '  -- every crate that returns it rather than correcting --'
command grep -r 'return Err( RingError::BatchTooLarge' --include=*.rs */src | sed 's|^ring/||;s|/src/lib.rs||'
echo '  -- and the principle ring_types states one variant further down --'
command grep -m1 -A3 -F '  /// guaranteeing exactly-once delivery: evicting an unread record to make room' ring_types/src/error.rs
echo '  -- three zeros in the family, three answers --'
command grep -r 'return Err( RingError::CapacityZero )\|if [a-z_.]* == 0 { 1 }\|\.max( 1 )' --include=*.rs ring_*/src | command grep -v '///\|//!' | sed 's|^ring/||;s|/src/lib.rs||;s|/src/capacity.rs||'
command grep -m1 -A1 -F '/// Ask `ready` until it answers true, pausing per `kind` between askings, for' ring_wait/src/lib.rs
```

Live output:

```
  -- what with_batch clamps, and what it says about clamping it --
  /// Set the batch size, clamped to at least `1` and at most the capacity —
  /// a batch larger than the ring can never be served however much draining
  /// happens, so it is corrected here rather than failing at first publish.
  ///
  pub const fn with_batch( mut self, batch : usize ) -> Self
  {
    let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };
    self.batch = if capped == 0 { 1 } else { capped };
    self
  -- the error variant named for exactly that condition --
  /// A batch of the requested length cannot be served — the ring's whole
  /// capacity is smaller than the request, so no amount of draining helps.
  BatchTooLarge
  {
    /// Slots asked for.
    requested : usize,
    /// Slots the ring has in total.
    capacity : usize,
  },
  -- every crate that returns it rather than correcting --
ring_batch:    return Err( RingError::BatchTooLarge { requested : count, capacity : capacity.get() } );
ring_claim:      return Err( RingError::BatchTooLarge
ring_gating:      return Err( RingError::BatchTooLarge { requested : count, capacity : self.capacity.get() } );
ring_slot:      return Err( RingError::BatchTooLarge { requested : payload.len(), capacity : N } );
  -- and the principle ring_types states one variant further down --
  /// guaranteeing exactly-once delivery: evicting an unread record to make room
  /// contradicts the guarantee, so the ring refuses to be built rather than
  /// silently degrading to `DropNewest`. A caller who never learns their policy
  /// was not applied is worse off than one whose construction failed.
  -- three zeros in the family, three answers --
ring_config:    self.producers = if producers == 0 { 1 } else { producers };
ring_config:    self.batch = if capped == 0 { 1 } else { capped };
ring_shutdown:    // `budget.max( 1 )` matches `ring_wait::wait_until`'s reading of its own
ring_shutdown:    for _ in 0..budget.max( 1 )
ring_types:      return Err( RingError::CapacityZero );
ring_wait:  for attempt in 0..spins.max( 1 )
/// Ask `ready` until it answers true, pausing per `kind` between askings, for
/// at most `spins` attempts.
```

---

## What a Zero Spin Budget Does

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order. Both extracts are
# anchored on their own syntax rather than on a line window, so neither
# truncates if the file above it grows.
echo '  -- the whole loop a zero budget enters --'
sed -n '/^pub fn wait_until< F >/,/^}/p' ring_wait/src/lib.rs
echo '  -- the Park arm that single pause reaches, and for how long --'
command grep 'from_micros' ring_wait/src/lib.rs
echo '  -- the test pinning the one look, and what it settles --'
command grep 'fn a_zero_budget_still_looks_once' ring_wait/tests/wait_test.rs
sed -n '/^fn a_zero_budget_still_looks_once/,/^}/p' ring_wait/tests/wait_test.rs | command grep 'wait_until\|assert_eq'
```

Live output:

```
  -- the whole loop a zero budget enters --
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
where
  F : FnMut() -> bool,
{
  for attempt in 0..spins.max( 1 )
  {
    if ready()
    {
      return Ok( attempt );
    }
    if !pause( kind, attempt )
    {
      break;
    }
  }
  Err( RingError::Empty )
}
  -- the Park arm that single pause reaches, and for how long --
      std::thread::sleep( std::time::Duration::from_micros( 50 ) );
  -- the test pinning the one look, and what it settles --
fn a_zero_budget_still_looks_once()
  let outcome = wait_until( WaitKind::Spin, 0, ||
  assert_eq!( outcome, Ok( 0 ) );
  assert_eq!( looks, 1 );
```

At `spins = 0` the range is `0..1`, so the body runs exactly once: `ready` is
asked, and on a false answer `pause( kind, 0 )` runs before the range is
exhausted and `Err( RingError::Empty )` is returned. Under `WaitKind::Park`
that one pause is `sleep( 50µs )`, unconditionally and by construction. The
existing test settles the look count on the *true* answer; the loop above is
what settles the false one, which is the path a zero budget is actually taken
on.

**This section used to quote a scratch crate, and the numbers it quoted were
never a measurement of this family.** It ran a four-strategy probe from
`-cfg_probe/src/bin/zero_budget.rs` — a directory since swept — and reported
`elapsed 146us` and `elapsed 132us` for `Park` across two runs. Those are real
readings of a real clock, and they are still the wrong evidence: `pause` asks
for 50µs and `std::thread::sleep` guarantees *at least* that, so everything
above 50µs is this platform's wakeup latency, not something `wait_until` did.
A probe that times a fixed sleep measures the scheduler. Reading the bound and
the arm settles the same three facts — one look, one pause, `Err( Empty )` —
exactly, on any machine, and with nothing left to attribute to the wrong
component.

---

### RC41 — The Family Has an Error Variant for the Exact Condition `with_batch` Corrects, and States the Opposing Principle in the Same File

`with_batch` caps a batch at the capacity and explains why: "a batch larger than
the ring can never be served however much draining happens, so it is corrected
here rather than failing at first publish."

`RingError::BatchTooLarge` is documented as "A batch of the requested length
cannot be served — the ring's whole capacity is smaller than the request, so no
amount of draining helps." That is the same sentence about the same condition,
and the variant carries both numbers, `requested` and `capacity`. Four crates
return it rather than correcting: `ring_batch:318`, `ring_claim:425`,
`ring_gating:269`, `ring_slot:351`.

So the two documents agree completely on the facts and take opposite positions on
what to do about them. And the family has written down which position it holds.
Two variants below `BatchTooLarge`, `PolicyUnsupported`'s doc explains why a ring
refuses to be built rather than degrade a `DropOldest` policy to `DropNewest`:
"A caller who never learns their policy was not applied is worse off than one
whose construction failed."

**Finding.** `with_batch` does the thing that sentence condemns, to a different
field, and nothing anywhere notes the divergence. A caller who writes
`RingConfig::new( 16 )?.with_batch( 999 )` has asked for something the family
considers an error worth naming, carrying both numbers, in four separate crates —
and gets `16` with no return value to inspect, no counter, and no log line.

The correction is documented at the setter, so this is not a hidden behaviour. It
is an unreportable one: the only way to learn it happened is to keep the value
you passed and compare it to `batch()` afterwards, which nothing in the family
does and no test asserts.

That gap matters most in the case the record was designed for. `with_batch`'s
rationale is about a hand-written builder chain, where the author can see both
numbers on adjacent lines. The crate's own module comment says the record's
purpose is that a manifest will describe it later
([`non_functional_requirement/002`](../non_functional_requirement/002_a_record_sized_for_a_design_not_yet_built.md)),
and a manifest is precisely where `batch = 999` beside `capacity = 16` is a typo
the author cannot see and wants told about. The rationale does not cover the case
the type exists for.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F 'fn a_caller_can_detect_a_clamp_by_comparing_what_they_asked_for()' ring_config/tests/config_test.rs
```

Live output:

```
fn a_caller_can_detect_a_clamp_by_comparing_what_they_asked_for()
{
  let requested_batch = 999;
  let cfg = RingConfig::new( 16 ).unwrap().with_batch( requested_batch );
```

**Disposition:** applied — `ring_config/tests/config_test.rs` now
carries `a_caller_can_detect_a_clamp_by_comparing_what_they_asked_for`, which
asserts the detection technique the finding names for both clamps
(`with_batch` and `with_producers`): keep the value passed in and compare it
to what the getter returns afterward. Making the setters fallible instead
would contradict `decisions/002`'s recorded infallibility decision and is out
of scope for this pass. The crate's 11 tests (`tests/config_test.rs`) plus 13
doctests re-verified passing (`cargo test --all-features`, 2026-09-04). Now
prints:
`fn a_caller_can_detect_a_clamp_by_comparing_what_they_asked_for()`

---

### RC42 — Three Zeros in the Family Get Three Different Answers, and the Undocumented One Sleeps

The same family answers a zero three ways. `Capacity::new( 0 )` returns
`Err( RingError::CapacityZero )` — a zero capacity is an error.
`with_producers( 0 )` returns `1` and says so at the setter, with a doctest
pinning it. `ring_wait::wait_until( kind, 0, ready )` runs `spins.max( 1 )`, and
says nothing.

The reading above settles what that third one does. Under every strategy the
budget of zero asks `ready` once — consistent with the function's own second
sentence, that `ready` is evaluated at least once — and then, because the answer
was false, it pauses. Under `WaitKind::Park` that pause is `sleep( 50µs )`.

**Finding.** The function's summary line says "for at most `spins` attempts". At
`spins = 0` it performs one attempt and one pause, so the stated bound is wrong
for that input, and wrong in the direction that costs: a caller passing a
computed budget that can reach zero, meaning *do not wait*, waits anyway. The
cost is 50µs by construction, plus whatever the platform adds on top of a sleep
of that length — which on the machine this instance was written on was another
80–100µs, though that figure belongs to the scheduler and not to this family.

`WaitKind::None` exists for exactly the "check and return" intent, and pairs with
`RingConfig::is_tick_safe` ([`item/002`](../item/002_the_two_derived_readings.md))
to let a caller express it in the record. A zero budget is the other way to
reach for it, it is the way that compiles without consulting either, and it is
the one with no documentation and a measurable cost.

Recorded here rather than in `ring_wait`'s own corpus because it is the third
point of the comparison RC41 rests on: the family has an error, a documented
clamp, and an undocumented clamp for the same shape of input, and no rule
anywhere for choosing among them.

**Disposition:** declined — the misleading "for at most `spins` attempts"
wording belongs to `ring_wait::wait_until`'s own doc comment at
`ring_wait/src/lib.rs:148-149`; this instance's own text says the
finding is recorded here only as the third point of RC41's comparison, not
as a fix for this crate to make in a corpus disposition pass.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/002`](002_the_setter_that_would_break_commutation.md) | The other hazard in the same two setters |
| [`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md) | The infallibility choice these clamps implement |
| [`invariant/001`](../invariant/001_two_ranges_that_cannot_be_violated.md) | What the clamps guarantee, from the other side |
| [`non_functional_requirement/002`](../non_functional_requirement/002_a_record_sized_for_a_design_not_yet_built.md) | The manifest future where the rationale does not hold |
| [`item/002`](../item/002_the_two_derived_readings.md) | `is_tick_safe`, the documented way to express the intent a zero budget reaches for |

### Sources

| Fact | Where |
|------|-------|
| `with_batch`'s clamp and its stated reason | `ring_config/src/lib.rs:128-131`, `:140-144` |
| `with_producers`' documented clamp | `ring_config/src/lib.rs:112-125` |
| `BatchTooLarge` and the two numbers it carries | `ring_types/src/error.rs:63-71` |
| The four crates that return it | Census above |
| "A caller who never learns their policy was not applied is worse off" | `ring_types/src/error.rs:78-79` |
| A zero capacity being an error | `ring_types/src/capacity.rs:44` |
| "at most `spins` attempts", against `spins.max( 1 )` | `ring_wait/src/lib.rs:148-149`, `:183` |
| One attempt and one 50µs sleep at a zero budget | `ring_wait/src/lib.rs` loop bound and `Park` arm, read above |

### Tests

| Test | Covers |
|------|--------|
| `batch_clamps_into_one_through_capacity` | The correction, but not that a caller asked for something else |
| `zero_producers_clamps_to_one` | The same for the producer count |
| `capacity_is_validated_at_construction` | The one input of the three that is an error rather than a clamp |
