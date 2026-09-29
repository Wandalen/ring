# Pitfall: The Number the Borrow Checker Hides

### Scope

- **Purpose**: Record why the only check a family caller can reach takes its capacity as data, what that costs the caller, and what happens when the number is wrong.
- **Responsibility**: Where each check gets the capacity it compares against, why one of them cannot get it the same way, and how a wrong one is reported.
- **In Scope**: `check_ends`'s third argument; `ring_core`'s borrow shape; the four call sites in the suite; `Violation::ReadingsDisagree`'s two causes.
- **Out of Scope**: What `check_ends` cannot detect at all (→ [`pitfall/001`](001_saturating_arithmetic_reports_health.md)'s P1, and [`integration/001`](../integration/001_reaching_the_cursors_of_a_live_ring.md)'s J4); the quiescence precondition (→ [`api/001`](../api/001_the_check_surface.md)'s B1).

### Abstract

**`check_ends` compares two readings against a capacity it is told, and the caller
cannot obtain that capacity from either value it passes.** The object that knows it
is the `Ring`, and the `Ring` is mutably borrowed for exactly as long as the ends
exist. So the number is re-typed by hand at every call site — and when it is wrong,
the crate reports the same `Violation` variant it uses for a genuinely inconsistent
ring.

### Where each check gets its capacity

The crate has three entry points and they do not agree.

| # | Entry point | Capacity from | Can the caller get it wrong? |
|---|---|---|---|
| K1 | `check` | `pair.capacity()` — the object | No. There is no argument to get wrong |
| K2 | `Watch::new` | `pair.capacity()`, stored in the baseline | No |
| K3 | `check_ends` | The caller, as a `Capacity` argument | **Yes** |

K1 and K2 take a `&CursorPair`, and `CursorPair` carries its own capacity and hands
it out through a `const fn`. K3 takes a `&Producer` and a `&Consumer`, and neither
of those exposes a capacity at all — `ring_core` puts `capacity()` on the `Ring`.

### Why K3 cannot do what K1 does

Reaching the ends of a `ring_core::Ring` goes through two borrows:

```rust
pub fn ends( &mut self ) -> Ends< '_, T >
pub fn split( &'a mut self ) -> ( Producer< 'a, T >, Consumer< 'a, T > )
```

`ends` takes `&mut self`. While the returned `Ends` — and therefore the `Producer`
and `Consumer` split out of it — are alive, the `Ring` is exclusively borrowed, and
`capacity( &self )` is a shared borrow that cannot be taken at the same time.

**The number has to be captured before the ends exist, or re-typed.** In this
crate's own suite it used to be re-typed at all four call sites; three of them now
capture, and the fourth keeps its literal because passing a wrong one by hand is
the thing that test asserts about (DB51).

### The two causes of one report

`Violation::ReadingsDisagree` has a single construction site, at the end of
`check_ends`, and two disjoint causes:

| # | Cause | What it means | What the report says |
|---|---|---|---|
| R1 | The ring's two ends genuinely do not sum to its capacity | Something is wrong with the ring — or it was read mid-flight | *"pending P plus free F is not the capacity C"* |
| R2 | The ring is fine and the third argument is not its capacity | Something is wrong with the call | The same sentence |

R2 is a real mistake worth catching, and the crate says so in the test that covers
it. The cost is that the report cannot separate the two, and only one of them is
about the ring the sentence describes.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| P5 | The capacity is captured before `ends()` and the ring is later rebuilt at a different size | R2, reported as R1 — an operator reads "the ring is inconsistent" and goes looking at the ring |
| P6 | The capacity is re-typed and mistyped | R2, reported as R1 |
| P7 | The caller passes the right capacity and the ring is genuinely inconsistent | R1 — the case the check exists for, and the one the suite never produces |
| P8 | The caller works around P5/P6 by dropping the ends to read `capacity()` | The readings are taken at a different moment from the check, which is the quiescence hazard by another route |

**P8 is the pitfall aimed at a reader of this instance.** Having seen that the
argument can be wrong, the obvious response is to fetch it from the ring — which
means dropping the ends, reading, re-splitting, and checking readings taken across
that gap. The precondition the crate states is that the ring be quiescent; a caller
who has held it quiescent can take the number at any point and it does not matter,
and a caller who has not is already outside the contract.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- the three entry points and where each gets a capacity --'
command grep 'pub fn check(\|pub fn new( pair\|pub fn check_ends' ring_debug/src/lib.rs | sed 's/^ */  /'
command grep 'pair.capacity()' ring_debug/src/lib.rs | sed 's/^ */  /'
echo '-- which type carries capacity() in each crate --'
awk '/^impl/{ last = $0 } /pub const fn capacity|pub fn capacity/{ print "  " FILENAME "  in: " last }' \
  ring_core/src/lib.rs ring_cursor/src/lib.rs
echo '-- the borrows between a Ring and its ends --'
command grep 'pub fn ends(\|pub fn split(\|pub fn capacity(' ring_core/src/lib.rs | sed 's/^ */  /'
echo '-- capacity accessors on the two values check_ends receives --'
printf '  on Producer: %s\n' "$( awk '/^impl.*Producer/{ f = 1 ; next } f && /^impl/{ exit } f && /fn capacity/{ n++ } END{ print n + 0 }' ring_core/src/lib.rs )"
printf '  on Consumer: %s\n' "$( awk '/^impl.*Consumer/{ f = 1 ; next } f && /^impl/{ exit } f && /fn capacity/{ n++ } END{ print n + 0 }' ring_core/src/lib.rs )"
echo '-- every check_ends call site in the suite, and how each gets its number --'
command grep 'check_ends(' ring_debug/tests/debug_test.rs | cut -c1-93 | sed 's/^ */  /'
printf '  sites deriving the capacity from the ring: %s\n' \
  "$( command grep -c 'check_ends( capacity,' ring_debug/tests/debug_test.rs )"
printf '  sites re-typing it as a literal:           %s\n' \
  "$( command grep -c 'check_ends( cap( ' ring_debug/tests/debug_test.rs )"
printf '  rustdoc sections naming where it comes from: %s\n' \
  "$( command grep -c '# Where the capacity comes from' ring_debug/src/lib.rs )"
echo '-- ReadingsDisagree: constructed once, produced by one test --'
printf '  construction sites in src:              %s\n' "$( command grep -c 'Err( Violation::ReadingsDisagree' ring_debug/src/lib.rs || true )"
printf '  tests asserting it as a produced value: %s\n' "$( command grep -c 'check_ends(.*),$' ring_debug/tests/debug_test.rs || true )"
command grep 'ReadingsDisagree {' ring_debug/tests/debug_test.rs | cut -c1-93 | sed 's/^ */  /'
```

Live output:

```
-- the three entry points and where each gets a capacity --
  pub fn check( pair : &CursorPair ) -> Result< (), Violation >
  pub fn new( pair : &CursorPair ) -> Result< Self, Violation >
  pub fn check_ends< T >
  check_seqs( producer, consumer, pair.capacity() )
  let capacity = pair.capacity();
-- which type carries capacity() in each crate --
  ring_core/src/lib.rs  in: impl< T : Send > Ring< T >
  ring_cursor/src/lib.rs  in: impl CursorPair
-- the borrows between a Ring and its ends --
  pub fn capacity( &self ) -> Capacity
  pub fn ends( &mut self ) -> Ends< '_, T >
  pub fn split( &'a mut self ) -> ( Producer< 'a, T >, Consumer< 'a, T > )
-- capacity accessors on the two values check_ends receives --
  on Producer: 0
  on Consumer: 0
-- every check_ends call site in the suite, and how each gets its number --
  assert!( check_ends( capacity, &producer, &consumer ).is_ok(), "an empty ring disagreed" );
  assert!( check_ends( capacity, &producer, &consumer ).is_ok(), "a partly-filled ring disagr
  assert!( check_ends( capacity, &producer, &consumer ).is_ok(), "a drained ring disagreed" )
  assert!( check_ends( capacity, &producer, &consumer ).is_ok(), "a legitimately full ring di
  check_ends( cap( 8 ), &producer, &consumer ),
  sites deriving the capacity from the ring: 4
  sites re-typing it as a literal:           1
  rustdoc sections naming where it comes from: 1
-- ReadingsDisagree: constructed once, produced by one test --
  construction sites in src:              1
  tests asserting it as a produced value: 1
  Err( Violation::ReadingsDisagree { pending : 0, free : 16, capacity : 8 } )
  Violation::ReadingsDisagree { pending : 2, free : 3, capacity : 16 },
```

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_saturating_arithmetic_reports_health.md](001_saturating_arithmetic_reports_health.md) | The other half of the same check's limits — what it cannot see even when given the right capacity |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | B1 — the quiescence precondition P8 routes back into |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_reaching_the_cursors_of_a_live_ring.md](../integration/001_reaching_the_cursors_of_a_live_ring.md) | Why `check_ends` is the only entry point a family caller can reach |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_violation.md](../type/001_violation.md) | `ReadingsDisagree`'s payload, and the argument that a variant carrying numbers is a better report than a message |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_debug/src/lib.rs`](../../src/lib.rs) | `check_ends`'s signature and the single `ReadingsDisagree` construction |
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | `capacity`, `ends` and `split` — the three signatures the borrow argument rests on |
| [`ring_cursor/src/lib.rs`](../../../ring_cursor/src/lib.rs) | `CursorPair::capacity` — the accessor K1 and K2 use instead of an argument |

### Tests

| Test | Relationship |
|------|--------------|
| `a_ring_measured_against_the_wrong_capacity_disagrees` | R2, asserted as a whole `Violation` — the only production of `ReadingsDisagree` in the suite |
| `the_two_ends_of_a_live_ring_agree` | The passing side, with the capacity re-typed as a literal at each of its three calls |
| `a_violation_reports_the_numbers_it_was_derived_from` | The other `ReadingsDisagree` in the file, hand-built as a `Display` fixture rather than produced |

### DB51 — the one reachable check takes a number the caller cannot derive from anything it passes

`check_ends( capacity, &producer, &consumer )` compares `consumer.len()` and
`producer.free_capacity()` against a `Capacity` supplied by the caller. Neither
`ring_core::Producer` nor `ring_core::Consumer` has a capacity accessor: the one
`capacity()` in `ring_core` is on `Ring`, and `Ring::ends` takes `&mut self`, so
for as long as the two ends exist the `Ring` is exclusively borrowed and
`capacity( &self )` cannot be called.

The other two entry points do not have this problem. `check` and `Watch::new` take
a `&CursorPair` and read `pair.capacity()` off the object; there is no argument to
get wrong. **The check that can be reached from a `ring_core::Ring` is the one
whose capacity is a parameter, and it is a parameter because of a borrow, not
because a caller was expected to choose it.**

All four call sites in this crate's own suite passed a literal — `cap( 16 )` three
times, `cap( 8 )` once — rather than reading it from anything. That is the shape a
downstream caller copies, and a re-typed number is one a refactor of the ring's
size does not follow. The hazard was not that the argument exists; it is that the
crate documented it as a feature — *"a caller passing the wrong capacity is a real
mistake and the resulting `ReadingsDisagree` is exactly the right report for it"* —
without recording how a caller is supposed to get it right.

**And the finding overstated it by one word.** The number is not derivable from
anything `check_ends` is *passed*, which is what makes K3 different from K1 and
K2 — but it is derivable one statement earlier, off the `Ring`, before `ends()`
takes the `&mut`. That distinction is the whole of the advice, and it was in
neither the rustdoc nor the suite.

**Disposition:** applied — `check_ends`' rustdoc carries a *Where the capacity
comes from* section stating that the parameter is a consequence of `Ring::ends`
taking `&mut self` rather than a choice offered to the caller, and showing the one
correct shape: bind `ring.capacity()` before the split and pass the binding. The
suite now demonstrates it instead of contradicting it — the three healthy-ring
assertions in `the_two_ends_of_a_live_ring_agree` derive the capacity and then
assert the derived value equals the configured one, while
`a_ring_measured_against_the_wrong_capacity_disagrees` keeps its literal on
purpose, because a hand-typed wrong number is exactly what it exists to catch —
a fourth site has since joined the derived column; see the correction below.
Now prints: `sites deriving the capacity from the ring: 4`

**Correction (2026-09-28):** `check_ends_on_a_genuinely_full_ring` was added
after this disposition was written, pinning the fourth legitimate state
`the_two_ends_of_a_live_ring_agree` does not reach — a ring genuinely filled to
capacity, `free_capacity() == 0` with nothing corrupt about it. It derives the
capacity off the ring exactly as the other three sites do, so the census the
recipe above prints has moved from three to four;
`a_ring_measured_against_the_wrong_capacity_disagrees` remains the sole site
keeping a literal, unchanged.

### DB52 — the variant is produced in the suite only by the cause it is not named for

`Violation::ReadingsDisagree` is constructed at exactly one place in the crate, the
final line of `check_ends`, and reaches it from two causes that have nothing to do
with each other: the ring's ends genuinely disagree, or the third argument was not
the ring's capacity. Its `Display` — *"pending {pending} plus free {free} is not
the capacity {capacity}"* — is a sentence about the ring, and it is what both
causes print.

Measured over the suite, the variant appears as a value exactly twice. One —
`ReadingsDisagree { pending : 2, free : 3, capacity : 16 }` — is assembled from
literals inside `a_violation_reports_the_numbers_it_was_derived_from`, a `Display`
fixture that never goes near a ring. The other — `ReadingsDisagree { pending : 0,
free : 16, capacity : 8 }` — is the only one `check_ends` is ever made to produce,
and it comes from a healthy 16-slot ring measured against a capacity of 8. **The
caller-error cause is the only one the suite ever produces.**

The cause the check exists for has no test, and it is not obvious that it could
have one through the public surface: making a real `ring_core::Ring`'s two ends
disagree requires reaching cursors that `ring_core` does not hand out, which is the
same boundary [`integration/001`](../integration/001_reaching_the_cursors_of_a_live_ring.md)
measures from the other side. Recorded as coverage rather than as a defect for that
reason — the gap is a consequence of the export Contract, not of the suite.
