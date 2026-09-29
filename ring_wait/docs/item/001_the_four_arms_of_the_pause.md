# Item: The Four Arms of the Pause

### Scope

- **Purpose**: Read `pause` arm by arm — the crate's single dispatch point, where the entire behavioural difference between the four strategies lives.
- **Responsibility**: Give each arm's code, its measured cost, what it returns, and what it does or does not read from its own parameters.
- **In Scope**: `ring_wait/src/lib.rs:112-146`.
- **Out of Scope**: The loop that calls it — see [`item/002`](002_the_loop_the_wrapper_and_the_two_questions.md).

### One Function Holds All Four

The doc comment says it plainly (`:95-96`):

> The whole behavioural difference between the four variants is in this one
> function; [`wait_until`] is the same loop for all of them.

That is a strong claim, and it is checkable: in `wait_until`'s whole body,
`kind` appears exactly once — as the value forwarded to `pause` — and never in a
comparison.

```sh
cd "$(git rev-parse --show-toplevel)"

# the body, excluding the signature that necessarily names the parameter
command grep -m1 -A11 -F '  for attempt in 0..spins.max( 1 )' ring_wait/src/lib.rs | grep "kind"

# any dispatch at all below `pause`
awk -v n1="$(( $( command grep -n -m1 -F '    WaitKind::None => false,' ring_wait/src/lib.rs | cut -d: -f1 ) + 2 ))" 'NR>n1' ring_wait/src/lib.rs | grep -vE "^[[:space:]]*//" \
  | grep -E "match|if[[:space:]]+kind|WaitKind::"
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
    if !pause( kind, attempt )
```

One hit from the first, at `if !pause( kind, attempt )`. **No output from the
second** — nothing below line 146 matches on the enum, compares against a
variant, or even names one. The file contains exactly two `match` statements,
and both are above that line: this one, and `escalation_hint`'s.

### The Four Arms

| Arm | Lines | Code lines | Comment lines | Returns | Reads `attempt` | Measured cost/attempt |
|-----|-------|-----------:|--------------:|:-------:|:---------------:|----------------------:|
| `Spin` | `:116-126` | 8 | 3 | `true` | **yes** | 52–61 ns |
| `Yield` | `:127-131` | 5 | 0 | `true` | no | 531–574 ns |
| `Park` | `:132-143` | 5 | **7** | `true` | no | 112 652–126 170 ns |
| `None` | `:144` | 1 | 0 | **`false`** | no | ~0 |

```sh
cd "$(git rev-parse --show-toplevel)"
for r in "116 126 Spin" "127 131 Yield" "132 143 Park" "144 144 None"; do
  set -- $r
  c=$( awk -v a=$1 -v b=$2 'NR>=a&&NR<=b' ring_wait/src/lib.rs \
       | grep -cE "^[[:space:]]*//" )
  t=$(( $2 - $1 + 1 ))
  printf '%-6s total=%2d  comment=%2d  code=%2d\n' "$3" "$t" "$c" "$(( t - c ))"
done
```

Live output:

```
Spin   total=11  comment= 3  code= 8
Yield  total= 5  comment= 0  code= 5
Park   total=12  comment= 7  code= 5
None   total= 1  comment= 0  code= 1
```

Two columns are worth stopping on.

### WT21 — Only One Arm Reads `attempt`

```sh
cd "$(git rev-parse --show-toplevel)"
awk -v n1="$(( $( command grep -n -m1 -F 'pub fn pause( kind : WaitKind, attempt : usize ) -> bool' ring_wait/src/lib.rs | cut -d: -f1 ) + 2 ))" -v n2="$(( $( command grep -n -m1 -F '    WaitKind::None => false,' ring_wait/src/lib.rs | cut -d: -f1 ) + 1 ))" 'NR>=n1 && NR<=n2' ring_wait/src/lib.rs | grep -n "attempt" | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
      for _ in 0..=( attempt % 8 )
```

**One hit**, in the `Spin` arm's loop header. Three of the four arms take a
parameter they never look at.

The signature's own justification is at `:98-100`:

> `attempt` is the zero-based index of the pause about to happen, so a strategy
> can behave differently early and late — [`WaitKind::Spin`] uses it to emit a
> CPU pause hint rather than a bare busy loop.

*"Can behave differently early and late"* is the general capability; exactly one
strategy exercises it, and even that one uses `attempt % 8`, which discards
everything above the low three bits
([`pitfall/002`](../pitfall/002_the_backoff_that_resets_every_eight_attempts.md)).
So the parameter's real information content, as consumed today, is three bits
delivered to one of four arms.

The parameter is right to be there anyway. `Yield` could plausibly want to yield
harder late, and `Park` could want to lengthen its sleep — both are edits inside
this function that change no signature and no caller. Removing `attempt` would
make either of those a breaking change to a public function, which is a
substantially worse trade than one unused parameter in three arms.

### `Park` Is the Only Arm With More Comment Than Code

Seven lines of explanation over five lines of code, and the explanation is not
about what the code does — `sleep( 50 µs )` needs none — but about what it is
*not*:

```rust
// ring_wait/src/lib.rs:134-140
// Sleeping rather than `thread::park` on purpose. Parking requires the
// publisher to hold the waiter's handle and unpark it, which is a
// registration relationship this crate deliberately does not have —
// `ring_handle` owns who-knows-whom. A short sleep is the same
// cost profile (idle rather than spinning) without inventing that
// relationship here, and the sleep length is what a real unpark would
// make unnecessary.
```

The variant is named `Park` and does not park, so the comment is load-bearing
rather than decorative — and it is the only comment in the family that a manual
check requires to *exist*, by name
([`decisions/001`](../decisions/001_park_sleeps_rather_than_parking.md) § WT20).

Note the last clause. *"The sleep length is what a real unpark would make
unnecessary"* names the 50 µs as the cost of the missing relationship, not as a
tuned constant. Nothing measures it, nothing asserts it, and the measured
delivery is 2.2×–4.1× the request
([`non_functional_requirement/002`](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md)
§ WT7).

### `None` Is One Line and Carries the Guarantee

```rust
// ring_wait/src/lib.rs:144
WaitKind::None => false,
```

The other three arms return `true` after doing something. This one returns
`false` after doing nothing, and that single `bool` is the entire non-blocking
guarantee ([`invariant/002`](../invariant/002_none_looks_exactly_once.md)) — a
value the caller may discard without a warning, because it is not `#[ must_use ]`
([`type/002`](../type/002_one_return_type_and_the_one_must_use.md) § WT8).

The asymmetry is worth naming: the arm doing the least work carries the property
the crate is most careful about, and does so in a return type that expresses none
of it.

### What the Arms Have in Common

| | `Spin` | `Yield` | `Park` | `None` |
|--|:------:|:-------:|:------:|:------:|
| Blocks the thread | no | no | **yes** | no |
| Costs a core while waiting | **yes** | partly | no | no |
| Uses `std` | no | yes | yes | no |
| A name `ring_handle` forbids | `spin_loop` — **not** forbidden | `yield_now` | `thread::sleep`, `Duration` | — |
| `is_non_blocking()` | false | false | false | **true** |
| Escalates to | `Yield` | `Park` | — | — |

The fifth row is the family's own answer to *"is this arm safe on the tick
path?"*, and it says **one** — not two. `WaitKind::is_non_blocking`
(`ring_types/src/policy.rs:81-88`) is true for `None` alone, and
`ring_config::is_tick_safe` (`ring_config/src/lib.rs:238-241`) is a
one-line forward to it.

So a predicate does exist, contrary to what the dependency-ban framing suggests.
What it is not is a *type* distinction: `is_non_blocking` classifies a value at
runtime, so nothing prevents a `WaitKind::Park` from arriving at a tick-path call
site — only a check somebody has to remember to perform. A build-time ban on the
crate needs no one to remember anything, which is why the tick-path guarantee uses one
([`integration/001`](../integration/001_two_dependencies_two_dependents_and_a_roster.md)
§ WT12, WT23).

The fifth row is `escalation_hint`, whose whole body is a second four-arm match
over the same enum — and whose two `None` results mean two different things:
`Park` has nothing more expensive to escalate to, and `None` never waited in the
first place ([`lifecycle/002`](../lifecycle/002_the_escalation_ladder_nobody_climbs.md)).


### WT37 — Const-ness Marks the One Item With Nothing to Do

One of the crate's six functions is a `const fn`, and it is the one with no
callers and no runtime effect.

```sh
cd "$(git rev-parse --show-toplevel)"
grep '^pub \(const \)\?fn ' ring_wait/src/lib.rs
# what stops the others from being const
grep 'yield_now\|thread::sleep\|spin_loop' ring_wait/src/lib.rs
```

Live output:

```
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
pub fn pause( kind : WaitKind, attempt : usize ) -> bool
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
        core::hint::spin_loop();
      std::thread::yield_now();
      std::thread::sleep( std::time::Duration::from_micros( 50 ) );
```

`escalation_hint` is `const`. `pause` cannot be — it calls `yield_now` and
`sleep`, neither available in a const context — and `wait_until`, `wait`,
`for_space` and `for_data` all reach `pause`, so const-ness is unavailable to the
whole reachable half of the surface.

`escalation_hint` is const because it is a pure four-into-three mapping over a
one-byte enum: no atomics, no scheduler, no clock. The same properties are why it
is the crate's only function that does nothing observable, and why nothing calls
it ([`lifecycle/002`](../lifecycle/002_the_escalation_ladder_nobody_climbs.md)).

So the annotation that promises compile-time evaluability sits on the item no
caller has asked to evaluate at any time, and the five items callers do reach are
all excluded from it by the same two lines of `std`. This is the crate's design
compressed into a keyword: the part that decides is pure and unused, the part
that waits is impure and is the whole product.

### Items

| File | Relationship |
|------|--------------|
| [002_the_loop_the_wrapper_and_the_two_questions.md](002_the_loop_the_wrapper_and_the_two_questions.md) | The other four public items, and who calls them |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | The loop this function is dispatched from |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_park_sleeps_rather_than_parking.md](../decisions/001_park_sleeps_rather_than_parking.md) | The seven-line comment, in full, with the alternatives it rejects |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | WT13 — three of these arms name something `ring_handle` forbids |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_none_looks_exactly_once.md](../invariant/002_none_looks_exactly_once.md) | The one-line arm that implements it |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_escalation_ladder_nobody_climbs.md](../lifecycle/002_the_escalation_ladder_nobody_climbs.md) | The second four-arm match over the same enum |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | Where the cost column was measured |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_the_backoff_that_resets_every_eight_attempts.md](../pitfall/002_the_backoff_that_resets_every_eight_attempts.md) | What `attempt % 8` discards |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_one_return_type_and_the_one_must_use.md](../type/002_one_return_type_and_the_one_must_use.md) | WT8 — the discardable `bool` |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/policy.rs:21-34` | The four discriminants these arms exhaust |
| `ring_handle/tests/handle_test.rs:493-494` | The seven names, three of which are here |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:56-65` | Exactly four discriminants exist, in the order these arms exhaust |
| `tests/wait_test.rs:69-84` | Every arm runs, and returns `true` exactly when the kind is blocking — swept over `WaitKind::ALL` |
| `tests/wait_test.rs:102-108` | Exactly one of four is non-blocking, computed rather than read |
| `tests/wait_test.rs:363-372` | The `Spin` arm keeps returning `true` across 32 attempt indices |
| `tests/wait_test.rs:374-381` | `None` stays false at every attempt index |
| `tests/manual/readme.md` § W3 | The single `false` |
| `tests/manual/readme.md` § W4 | `Park` sleeps, and must explain why it is not parking |
| `tests/manual/readme.md` § W5 | The spin hint is inside a bounded `for` |
