# Invariant: `None` Looks Exactly Once

### Scope

- **Purpose**: State the crate's behavioural invariant — `WaitKind::None` evaluates the predicate exactly once, whatever the budget — show the single line that implements it, and separate the assertion that would catch a regression from the one that would not.
- **Responsibility**: Give the property, the mechanism, both assertions, and the boundary the guarantee stops at.
- **In Scope**: `WaitKind::None`'s behaviour through `pause` and `wait_until`.
- **Out of Scope**: Why the tick path needs it — see [`integration/001`](../integration/001_two_dependencies_two_dependents_and_a_roster.md).

### The Property

> Under `WaitKind::None`, `wait_until` evaluates `ready` exactly once and
> returns, for every value of `spins` from `0` to `usize::MAX`.

Not *"at most once"*, and not *"quickly"*. Exactly once, and the *exactly* is
what the test asserts.

### The Mechanism Is One `false`

```rust
// ring_wait/src/lib.rs:144
WaitKind::None => false,
```

```rust
// ring_wait/src/lib.rs:189-192
if !pause( kind, attempt )
{
  break;
}
```

That is the whole implementation. There is no timeout, no budget-of-one special
case, and no `if kind == WaitKind::None` anywhere in `wait_until`. W3 checks
exactly that:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs | grep -E "WaitKind::None"
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs | grep -E "if[[:space:]]+!pause|break"
```

Live output:

```
    WaitKind::Park | WaitKind::None => None,
    WaitKind::None => false,
    if !pause( kind, attempt )
      break;
```

Two hits from the first — `escalation_hint`'s arm returning `None`, and `pause`'s
arm returning `false`. From the second, `if !pause` and its `break`, on
consecutive lines of the stripped stream. W3's own note states the regression it
guards:

> If `wait_until` ever gains a `if kind == WaitKind::None` special case, the
> non-blocking guarantee has moved out of `pause` and into the loop, and
> `pause`'s own contract — documented as "returns whether the caller should try
> again at all" — has quietly become advisory.

### The Two Assertions, and Which One Matters

`tests/wait_test.rs` asserts the property twice, and its module documentation is
unusually direct about the difference (`:21-34`):

| | The stopwatch (`:112-126`) | The count (`:128-144`) |
|--|---------------------------|------------------------|
| Asserts | elapsed < 1 s | `looks == 1` |
| Catches a `None` that blocks | yes | yes |
| Catches a `None` that spins the full 1024 | **no** | yes |
| Flakes on a loaded CI box | possible | never |
| Bound chosen to be | absurd, not tight | exact |

The stopwatch exists because the property genuinely is temporal — `None` must
not block — and its bound is *"a whole second for an operation whose honest cost
is a single predicate evaluation. A run that exceeds it has not been slow, it
has blocked."*

The count is the assertion that would actually catch a regression. A `None` that
looped the full 1024 times before giving up costs **1.0–2.4 µs** on this machine
(release, 3 runs) — it passes the one-second bound with four hundred thousand
times the headroom. Only the count says otherwise.

The invariant survives at both ends of the budget range:

```
probe: wait_until( None, usize::MAX, || { looks += 1; false } )
       None with usize::MAX budget produced 1 look(s)
```

### `None` Still Looks — the Mirror Assertion

*"Returns immediately"* must not degrade into *"returns without looking"*, which
would pass every timing assertion and always report empty.
`tests/wait_test.rs:146-157` pins the other side by publishing first and
requiring success:

```rust
pair.producer().store( Seq( 2 ), Ordering::Release );
assert_eq!( for_data( &pair, 2, WaitKind::None, DEFAULT_SPINS ), Ok( 0 ) );
assert_eq!( for_space( &pair, WaitKind::None, DEFAULT_SPINS ), Ok( 0 ) );
```

The module documentation at `:151-153` makes the same point in prose: `ready` is
evaluated at least once under every strategy including `None` — *"the
non-blocking variant returns whatever is available, which requires looking."*

Together the three tests bracket the property from every side: it looks at least
once, at most once, and does not block.

### And the Mirror for the Other Three

`tests/wait_test.rs:159-175` is the assertion that keeps the enum meaningful: if
`None`'s single look were the behaviour of all four, the enum would carry no
information. Each of `Spin`, `Yield`, `Park` at a budget of 3 must make exactly
3 looks, and each does.

`tests/wait_test.rs:102-108` states the count directly — exactly one of the four
discriminants is non-blocking, computed by filtering `WaitKind::ALL` through
`pause` rather than by reading `is_non_blocking`. That is the two-crate
agreement check from the behavioural side
([`decisions/002`](../decisions/002_the_discriminants_live_in_ring_types.md)).

### Where the Guarantee Stops

At the closure boundary, and completely:

```rust
// nothing prevents this, and nothing catches it
wait_until( WaitKind::None, 1, || { std::thread::sleep( a_second ); false } )
```

One look, `None` honoured, one second elapsed. The invariant is over *this
crate's* pause, not over the caller's predicate, and no type expresses the
difference ([`api/002`](../api/002_the_predicate_is_the_parameter.md)).

This is why the tick-path guarantee's enforcement is a dependency ban rather than a
constraint on `WaitKind`: the safe subset of this crate cannot be expressed in
its own types, so `ring_poll` excludes the crate entirely and writes its own
loop ([`integration/001`](../integration/001_two_dependencies_two_dependents_and_a_roster.md) § WT12).


### WT35 — The Crate That Is a Retry Loop Contributes No Interleaving Model

The family runs `loom` broadly. This crate's entire test file opts out.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A6 -F '// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps' ring_wait/tests/wait_test.rs
# crates whose tests are loom-aware, and whether this one has a loom-only module
grep -rl 'loom' --include=*.rs */tests/ | wc -l
grep -c 'cfg( loom )' ring_wait/tests/wait_test.rs || echo '0 — no loom-gated module'
```

Live output:

```
// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure — so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![cfg(not(loom))]
29
0
0 — no loom-gated module
```

The gate is `#![ cfg( not( loom ) ) ]` at file scope, with a comment giving a
sound reason: under `--cfg loom` the family swaps `ring_atomic`'s atomics for
loom's instrumented ones, and those may only be touched inside a `loom::model`
closure, so an ungated file would break a family-wide loom run. Twenty-one
crates' test files are loom-aware; this one's opts out wholesale.

The asymmetry is that several of those crates pair the gate with a
`#[ cfg( loom ) ] mod exhaustive` that runs the interleavings under the model.
This crate has none — the second grep returns zero. So under a family loom run,
`ring_wait` contributes nothing.

This is defensible on the same ground as everything else here: the crate owns no
atomic, no `Ordering`, and no shared state. Its loop reads whatever the caller's
closure reads, and the interleavings that matter belong to `ring_cursor`, which
does have them. There is nothing here for a model to explore that is not already
explored where the atomics live.

It is worth recording because of what the crate *is*. `ring_wait` is the family's
answer to "what does a thread do while another thread has not finished yet", and
the family's tool for exactly that question does not run against it — for a
reason that is about where the atomics are declared rather than about where the
waiting happens.


### WT36 — Two Independent Definitions of Non-Blocking, Never Compared

`None` looks exactly once because `pause`'s fourth arm returns `false`. The family
also has a predicate for the same property, and this crate never calls it.

```sh
cd "$(git rev-parse --show-toplevel)"
# how the invariant is implemented here
grep 'WaitKind::None => false' ring_wait/src/lib.rs
# and every occurrence of the family's predicate in this crate's source
grep 'is_non_blocking' ring_wait/src/lib.rs
# what it is defined as
grep -A4 'fn is_non_blocking' ring_types/src/policy.rs
```

Live output:

```
    WaitKind::None => false,
//! [`ring_types::WaitKind::is_non_blocking`] is true for exactly that variant.
  pub const fn is_non_blocking( self ) -> bool
  {
    match self
    {
      Self::None => true,
```

`ring_types::WaitKind::is_non_blocking` is a wildcard-free `match` over all four
variants (`Self::None => true`, the other three `=> false`), not the
single-pattern `matches!` it used to be. This crate's non-blocking behaviour is a
separate `match` arm returning `false`, also wildcard-free. Both say "None and
nothing else", written twice, in two crates.

`is_non_blocking` occurs once in this crate's source and the occurrence is a
rustdoc link in the module header, not a call: the prose says the predicate *"is
true for exactly that variant"*. The tests name it twice, in
`exactly_one_discriminant_is_non_blocking`, which asserts the two agree today.

That test is the only thing connecting them, and it is in this crate rather than
in `ring_types` where the predicate lives.

**Disposition:** applied, on the `ring_types` side. `is_non_blocking` used to be
`matches!( self, Self::None )`, so a fifth `WaitKind` variant would have silently
read `false` — blocking, like the other three — with nothing forcing a second
look. `Fix( wait_kind_is_non_blocking_classification_not_exhaustive )`
(`ring_types/src/policy.rs:67-79`) replaced it with the exhaustive match shown
above. `pause`'s own match was already wildcard-free (W3, above), so today a
fifth variant fails to compile in *both* crates rather than diverging silently in
either. What the fix does not touch is the duplication itself: the property is
still asserted in two match statements in two crates, held in step only by the
test named above rather than by a shared source of truth. WT22 records that the
identically-ruled sibling enum has already grown a second handler under exactly
this arrangement.

### Invariants

| File | Relationship |
|------|--------------|
| [001_every_repetition_is_a_counted_for.md](001_every_repetition_is_a_counted_for.md) | The structural invariant on the same loop |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_predicate_is_the_parameter.md](../api/002_the_predicate_is_the_parameter.md) | The boundary the guarantee stops at |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | The `break` this invariant travels through |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_the_discriminants_live_in_ring_types.md](../decisions/002_the_discriminants_live_in_ring_types.md) | `is_non_blocking`, the same fact in the other crate |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | Why the enforcement is a ban rather than a type |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | The arm that returns `false` |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | The 40 ns that makes the stopwatch a poor discriminator |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_one_return_type_and_the_one_must_use.md](../type/002_one_return_type_and_the_one_must_use.md) | WT8 — the `bool` that carries this guarantee, silently discardable |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/policy.rs:32-34,81-88` | `None`'s definition and `is_non_blocking` |
| `ring_poll/src/lib.rs:20-23` | The ban this invariant is not strong enough to replace |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:21-34` | Why the stopwatch is not the assertion that matters |
| `tests/wait_test.rs:112-126` | The temporal half — an absurd bound, not a tight one |
| `tests/wait_test.rs:128-144` | The count — exactly one look at a budget of 1024 |
| `tests/wait_test.rs:146-157` | The mirror — `None` still looks, and can succeed |
| `tests/wait_test.rs:159-175` | The other three do loop, three times at a budget of three |
| `tests/wait_test.rs:374-381` | `pause( None, n )` is false at every attempt index |
| `tests/manual/readme.md` § W3 | The single `false`, and the special case that must not appear |
