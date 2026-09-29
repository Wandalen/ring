# Decision: Why `is_closed` Is Absent

**Status:** settled, against the pre-implementation specification. Both
[`api/001`](../api/001_producer_surface.md) and
[`api/002`](../api/002_consumer_surface.md) specified an `is_closed()` row.
Neither handle has one, and the reason is a collision between two features that
did not exist at the same time when those instances were written.

### Scope

- **Purpose**: Record why both `api/` instances specify an `is_closed()` row that neither handle has — a collision between two features that did not exist at the same time when those instances were written.
- **Responsibility**: The collision, the four options considered, what composing `ring_shutdown` around a handle actually costs, and the condition that would reverse the ruling.
- **In Scope**: The absence of `is_closed` from `Producer` and `Consumer`.
- **Out of Scope**: What the crate does add (→ [`001_what_this_crate_is_for.md`](001_what_this_crate_is_for.md)); where the flag lives (→ [`ring_shutdown/docs/invariant/001`](../../../ring_shutdown/docs/invariant/001_exactly_one_liveness_flag.md)).

### The collision

| Feature | Says | Owned by |
|---|---|---|
| 179 / 184 | A handle should be able to ask whether the ring is closed, reading the authoritative flag rather than copying it | This crate reads; `ring_shutdown` owns |
| 183 | Nothing reachable from the tick path may park | Asserted in `ring_poll`'s suite |

Reading `ring_shutdown`'s flag means depending on `ring_shutdown`. And:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'ring_wait' ring_shutdown/Cargo.toml
```

Live output:

```
ring_wait = { path = "../ring_wait" }
```

`ring_shutdown` depends on `ring_wait`, for `wait_for_close` and
`for_space_or_close` — two functions that exist precisely to block a thread
*outside* the tick. Taking the dependency would make
`ring_wait::pause( WaitKind::Park, .. )` nameable from a crate on the tick path,
which is the exact condition
[`ring_poll::PARKING_CRATES`](../../../ring_poll/docs/invariant/001_no_parking_operation_on_the_tick_path.md)
asserts against — and `ring_poll`'s suite would fail on the next run.

**The failure would be loud, and in the right place.** That is worth noting: the
guard did its job before the mistake was made, which is the only time a guard is
cheap.

### The four options

| Option | Verdict |
|---|---|
| Add the dependency and widen `PARKING_CRATES` | **No.** The roster is a public constant precisely so that widening it is a decision rather than a fix. Widening it to make a convenience method compile is the failure mode `ring_poll`'s pattern doc names |
| Keep a copy of the flag on each handle | **No.** A second copy can disagree with the first, which is `ring_shutdown`'s own [`invariant/001`](../../../ring_shutdown/docs/invariant/001_exactly_one_liveness_flag.md) and this crate's [`algorithm/002`](../algorithm/002_delegating_to_the_backend.md) closed-check row, agreeing from two directions |
| Feature-gate `ring_shutdown`'s two waiting functions | **Not yet.** It is the structurally correct answer and it does not survive cargo's feature unification: a workspace build that enables the gate anywhere enables it everywhere, so the parking functions would be nameable again under exactly the build the gates run |
| **Compose instead of depend** | **Yes.** A caller who needs close-awareness wraps the producer in `ring_shutdown::Guarded`, which already exists and already checks the flag before every push |

### What composition costs, stated honestly

`ring_shutdown::Guarded` wraps a `ring_core::Producer`, not a
`ring_handle::Producer`. So a caller who wants both this crate's narrowings and
close-awareness cannot have both today — they take `ring_core`'s ends and lose
N2 (`try_clone` withheld), or they take this crate's and lose the flag.

That is a real gap and it is not papered over. It is also a gap with an obvious
shape: `Guarded` should be generic over "something that can `try_push`", which
is a trait this family does not have and has not needed twice before. **The
second need is now on record**, which is the threshold at which inventing the
trait stops being speculative.

### The condition under which this reverses

`ring_shutdown` losing its `ring_wait` dependency — most plausibly by
`wait_for_close` and `for_space_or_close` moving to `ring_barrier`, which is an
outside-the-tick crate that will depend on `ring_wait` anyway. Then
`PARKING_CRATES` drops to two entries, this crate can depend on `ring_shutdown`
freely, and the `is_closed()` rows can be implemented as originally specified.

```sh
cd "$(git rev-parse --show-toplevel)"
# the check that says the reversal is available
grep -c ring_wait ring_shutdown/Cargo.toml    # today: 1; when 0, reopen this
```

Live output:

```
1
```

Filed rather than done, because moving code into `ring_barrier` means
implementing a crate that belongs to a later stage, and doing it now to enable a
convenience method is the wrong order.

### Related

- [`api/001`](../api/001_producer_surface.md), [`api/002`](../api/002_consumer_surface.md) —
  where the rows were specified, each now marked absent with a pointer here
- [`ring_poll/docs/pattern/001`](../../../ring_poll/docs/pattern/001_enforcement_by_dependency_graph.md) —
  the guard that caught this, and its own account of how it degrades

### HD15 — The Chosen Option Composes Around a Type This Crate Does Not Produce

"Compose instead of depend" is the ruling. The composition does not typecheck
against this crate's handles:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what Shutdown::guard accepts --'
command grep -E '^  pub const fn guard<' ring_shutdown/src/lib.rs
echo '  -- which Producer that is --'
command grep -E '^use ring_core|^use ring_handle' ring_shutdown/src/lib.rs
echo '  -- and what this crate hands out --'
command grep -E '^  pub fn split\(' ring_handle/src/lib.rs
```

Live output:

```
  -- what Shutdown::guard accepts --
  pub const fn guard< 'a, T >( &'a self, producer : Producer< 'a, T > ) -> Guarded< 'a, T >
  -- which Producer that is --
use ring_core::{ Consumer, Producer };
  -- and what this crate hands out --
  pub fn split( &'a mut self ) -> ( Producer< 'a, T >, Consumer< 'a, T > )
```

`guard` takes a `ring_core::Producer`. `Ends::split` returns a
`ring_handle::Producer`. There is no conversion between them in either
direction — the wrapper's field is private.

**The instance already says this, in its own "What composition costs" section,
and then rules in favour of it anyway.** That is a defensible call: the
alternatives were a forbidden dependency, a duplicated flag, and a feature gate
that cargo unification defeats. Composition is the least bad of four bad
options.

What the ruling does not carry is the size of what it gives up. A caller
choosing close-awareness gives up all four narrowings N1–N4 — including the
`try_clone` withholding that
[`001`](001_what_this_crate_is_for.md) calls one of the two substantive ones. So
the two features this family most wants together are the two it cannot combine,
and the ADR records the fact without ranking it against the options it rejected.

### HD16 — The Coupling Is Named Seven Times Here and Executed Zero Times

The check that makes this decision safe reads `ring_handle`'s manifest from
`ring_poll`'s test binary. This crate knows that, repeatedly:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the roster, and which crates it scans --'
command grep -E 'PARKING_CRATES : |for tick_path in ' \
  ring_poll/src/lib.rs ring_poll/tests/poll_test.rs | sed 's|ring/||'
echo '  -- how often this crate names the guard, and where --'
command grep -rc 'ring_poll' ring_handle/Cargo.toml ring_handle/src/lib.rs \
  ring_handle/tests/handle_test.rs ring_handle/tests/manual/readme.md \
  | sed 's|ring_handle/|  |'
echo '  -- and how often its own suite reads a manifest --'
printf '  manifest reads in ring_handle tests: %s\n' \
  "$( command grep -hcE 'Cargo\.toml|CARGO_MANIFEST' ring_handle/tests/*.rs | paste -sd+ | bc )"
```

Live output:

```
  -- the roster, and which crates it scans --
ring_poll/src/lib.rs:pub const PARKING_CRATES : [ &str; 3 ] = [ "ring_barrier", "ring_shutdown", "ring_wait" ];
ring_poll/tests/poll_test.rs:  for tick_path in [ "ring_poll", "ring_handle", "ring_core" ]
ring_poll/tests/poll_test.rs:  for tick_path in [ "ring_poll", "ring_handle", "ring_core" ]
  -- how often this crate names the guard, and where --
  Cargo.toml:0
  src/lib.rs:1
  tests/handle_test.rs:3
  tests/manual/readme.md:3
  -- and how often its own suite reads a manifest --
  manifest reads in ring_handle tests: 0
```

`PARKING_CRATES` is a three-name const in `ring_poll`, and `poll_test.rs` walks
`[ "ring_poll", "ring_handle", "ring_core" ]` reading each one's manifest. This
crate names that guard seven times across three files — the module doc, three
doc comments in `handle_test.rs`, three lines of the manual plan — and its own
manifest names it zero times, correctly, since the edge runs the other way.

**Every one of the seven is prose. The executable count is zero.** No test in
this crate reads a manifest, its own or anyone's. The one local guard that comes
closest, `no_parking_shaped_name_appears_in_the_source`, says in its own doc
comment that it "reads the crate's own text rather than its manifest, which is a
different instrument answering a different question" — an accurate disclaimer
that assigns the manifest half to `ring_poll` and stops there.

So a maintainer inside this crate can add `ring_shutdown` to the manifest, run
`cargo test -p ring_handle`, see green, and have broken a documented family
invariant. Only a workspace run catches it. **The usual version of this
complaint — "nobody wrote it down" — does not apply**: it is written down seven
times, in three files, at least one of which the maintainer has open. The gap is
narrower and harder to close, because the seven mentions are all in places read
while thinking about parking, and the edit that breaks it is made while thinking
about a dependency.
