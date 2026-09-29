# Invariant: The Setters Commute, and One Absence Is Why

### Scope

**Purpose:** Record that the four setters may be applied in any order, the
structural reason the property holds, and what the test that carries its name
actually exercises.

**Responsibility:** The one cross-field read in the crate, the immutability that
makes it safe, the absent setter that would end it, and the coverage gap between
the property and its test.

**In Scope:** `ring_config/src/lib.rs:44`, `:71`, `:91`, `:106`, `:124`,
`:142-143`, `:154-156`; `ring_config/tests/config_test.rs:85-102`.

**Out of Scope:** The two ranges the setters preserve are
[`invariant/001`](001_two_ranges_that_cannot_be_violated.md). Why `capacity` is a
constructor argument rather than a setter is
[`decisions/001`](../decisions/001_capacity_is_the_one_parameter_that_is_not_a_with.md).

---

## The One Cross-Field Read, and the Field It Reads

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every setter body: three write one field, the fourth reads a second --'
command grep 'self\.[a-z_]* = \|let capped' ring_config/src/lib.rs
echo '  -- every code line naming the capacity field: one declaration, one write, two reads --'
command grep 'capacity' ring_config/src/lib.rs | command grep -v '///\|//!'
echo '  -- the setter that would make that read mutable, and does not exist --'
command grep -c 'with_capacity' ring_config/src/lib.rs || true
echo '  -- and what the commutation test feeds them --'
command grep -m1 -A12 -F '  let forward = RingConfig::new( 32 ).unwrap()' ring_config/tests/config_test.rs
```

Live output:

```
  -- every setter body: three write one field, the fourth reads a second --
    self.wait = wait;
    self.overflow = overflow;
    self.producers = if producers == 0 { 1 } else { producers };
    let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };
    self.batch = if capped == 0 { 1 } else { capped };
  -- every code line naming the capacity field: one declaration, one write, two reads --
  capacity : Capacity,
        capacity : Capacity::new( slots )?,
    let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };
  pub const fn capacity( &self ) -> Capacity
    self.capacity
  -- the setter that would make that read mutable, and does not exist --
0
  -- and what the commutation test feeds them --
  let forward = RingConfig::new( 32 ).unwrap()
    .with_wait( WaitKind::None )
    .with_overflow( OverflowPolicy::Fail )
    .with_producers( 2 )
    .with_batch( 8 );

  let backward = RingConfig::new( 32 ).unwrap()
    .with_batch( 8 )
    .with_producers( 2 )
    .with_overflow( OverflowPolicy::Fail )
    .with_wait( WaitKind::None );

  assert_eq!( forward, backward );
```

---

### RC23 — Commutation Holds Because One Field Has No Setter, and Nothing Records That

Three of the four setters read nothing but their own argument: `with_wait` and
`with_overflow` are plain assignments, and `with_producers` branches on the value
it was passed. The fourth reads a second field — `with_batch` caps against
`self.capacity.get()` — and it is the only cross-field read in the crate.

The census settles what that read can observe. `capacity` appears on five code
lines: the declaration at `:44`, one write at `:71` inside `new`, the read at
`:142`, and the getter at `:154-156`. `with_capacity` appears zero times. So the
one field any setter reads is written exactly once, before any setter can run, and
never again.

**Finding.** Order-independence is therefore not a property the setters maintain.
It is a consequence of a field being unreachable: every setter's effect is a
function of its own argument and a constant, and each writes a field no other
setter touches, so applying them in any sequence lands in the same place.

Nothing says so. The declaration at `:41-49` carries no note that `capacity`'s
immutability is load-bearing, `with_batch`'s doc explains the clamp without
mentioning that the value it clamps against cannot move, and `setters_commute`'s
own comment states the property — "the same five values in any order produce the
same record" — without the reason.

The absence is worth recording because the missing setter is a plausible addition.
`capacity` is the one field with no `with_`, and the only way to change it is to
build a new record; a caller wanting to resize would reasonably ask for one. A
`with_capacity` would make `with_batch`'s read observe a mutable field, and
`RingConfig::new( 8 ).unwrap().with_batch( 8 ).with_capacity( 2 )` would hold
`batch = 8` against `capacity = 2` — breaking commutation and the range
[`invariant/001`](001_two_ranges_that_cannot_be_violated.md) records in one step,
with nothing at either site warning that it would.

---

### RC24 — The Test Named for the Property Feeds It Only Values No Clamp Touches

`setters_commute` builds two records from capacity 32 with `with_producers( 2 )`
and `with_batch( 8 )`, in opposite orders, and asserts them equal. Both values are
already inside their ranges: `2` is not zero, and `8` is not above `32`. Neither
clamp fires in either chain.

So the test demonstrates that four assignments to four disjoint fields commute,
which is the easy half. The setter whose result depends on another field is
exercised only on the path where that dependence has no visible effect.

Twenty-four orderings with both clamps firing — the probe below, run against this
crate — land on one identical record:

```
base                RingConfig::new( 8 ), producers 1, batch 1
applied per order   with_wait, with_overflow, with_producers( 0 ), with_batch( 999 )
orders run          24
equal to the first  24
distinct results    1
  capacity 8   producers 1   batch 8
```

```rust
#[ derive( Clone, Copy ) ]
enum Step { Wait, Overflow, Producers, Batch }

fn apply( cfg : RingConfig, step : Step ) -> RingConfig
{
  match step
  {
    Step::Wait      => cfg.with_wait( WaitKind::None ),
    Step::Overflow  => cfg.with_overflow( OverflowPolicy::Fail ),
    // clamps up to 1
    Step::Producers => cfg.with_producers( 0 ),
    // clamps down to the capacity
    Step::Batch     => cfg.with_batch( 999 ),
  }
}
```

**Finding.** The property is real and it survives both clamps — every one of the
twenty-four orders produces `producers 1, batch 8` and compares equal. The suite
does not show that. It shows the two-permutation case on unclamped values, and the
interesting case is the other one: a clamp is where a cross-field read is
observable, and a cross-field read is the only way commutation could fail.

The gap is narrow and the suite covers the pieces elsewhere —
`zero_producers_clamps_to_one` and `batch_clamps_into_one_through_capacity` each
walk their own clamp, they just never walk one in two orders. Recorded because the
test carries the property's name and stops just short of the case that would break
if the property ever did.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](001_two_ranges_that_cannot_be_violated.md) | The two ranges every ordering preserves |
| [`decisions/001`](../decisions/001_capacity_is_the_one_parameter_that_is_not_a_with.md) | Why `capacity` has no setter in the first place |
| [`pitfall/002`](../pitfall/002_the_setter_that_would_break_commutation.md) | The addition this property could not survive |
| [`api/001`](../api/001_twelve_functions_eleven_of_them_const.md) | The four setters as a surface |

### Sources

| Fact | Where |
|------|-------|
| Three single-field setter bodies | `ring_config/src/lib.rs:91`, `:106`, `:124` |
| The one cross-field read | `ring_config/src/lib.rs:142` |
| `capacity` declared, written once, never after | `ring_config/src/lib.rs:44`, `:71` |
| No `with_capacity` | Census above, zero occurrences |
| The commutation test's inputs | `ring_config/tests/config_test.rs:89-101` |
| Twenty-four clamped orderings agreeing | Probe above, source inlined |

### Tests

| Test | Covers |
|------|--------|
| `setters_commute` | Two orderings, both on unclamped values |
| `each_setter_is_independent` | That each setter leaves the other fields alone |
| `batch_clamps_into_one_through_capacity` | The clamp the commutation test never orders around |
