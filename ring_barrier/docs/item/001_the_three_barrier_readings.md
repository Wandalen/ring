# Item: The Three Barrier Readings

### Scope

- **Purpose**: Give the per-method contract and coverage for the three methods that read cursors, and account for who calls each.
- **Responsibility**: `frontier`, `available`, `admits` — signature, empty-set answer, callers, tests, and what each test can actually falsify.
- **In Scope**: The reading tier.
- **Out of Scope**: The shape tier and `wait_for` — see [`002`](002_the_five_accessors_and_the_wait.md).

### The Three

| | `frontier` | `available` | `admits` |
|--|-----------|-------------|----------|
| Signature | `( &self ) -> Option< Seq >` | `( &self, from : Seq ) -> u64` | `( &self, from : Seq, count : u64 ) -> bool` |
| Answers | *where are my dependencies* | *how much may I read* | *may I read this much* |
| Empty barrier | `None` | `0` | `count == 0` |
| Body | `ring_cursor::slowest( self.dependencies )` | `frontier().map_or( 0, … )` | `count <= self.available( from )` |
| Allocations per call | 1 | 1 | 1 |
| `const` | no | no | no |
| `#[ must_use ]` | ✔ | ✔ | ✔ |

Each is the previous one plus one operation. That laddering is the family's
standard shape ([`pattern/002`](../pattern/002_the_quantity_the_predicate_and_the_wait.md))
and it is what makes the call-site census below lopsided.

### BR3 — Who Calls Each, in All of `src/`

```sh
cd "$(git rev-parse --show-toplevel)"
for m in frontier available admits; do
  echo "== .$m"
  for f in ring_*/src/*.rs; do
    n=$( grep -vE "^[[:space:]]*//" "$f" | grep -c "\.$m(" )
    [ "$n" != 0 ] && printf '  %-42s %s\n' "$f" "$n"
  done
done
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
== .frontier
  ring_barrier/src/lib.rs                    2
  ring_consume/src/lib.rs                    1
== .available
  ring_barrier/src/lib.rs                    1
  ring_consume/src/lib.rs                    3
  ring_core/src/lib.rs                       2
  ring_mpsc/src/lib.rs                       1
== .admits
  ring_barrier/src/lib.rs                    1
```

| Method | Library callers | Which |
|--------|----------------:|-------|
| `frontier` | 3 | `Barrier::available`, `Barrier::wait_for`, `ring_consume::Consumer::available` |
| `available` | **1** | `Barrier::admits` — in this crate |
| `admits` | **1** | `Barrier::wait_for` — in this crate |

The bottom two rungs of the ladder have exactly one caller each, and both are
internal. `frontier` is the only reading anything outside this crate performs —
and `ring_consume` reaches it in order to recompute `available` by hand
([`integration/001`](../integration/001_three_dependencies_and_one_dependent.md) § BR2).

So the public value of `available` and `admits` is entirely prospective. That is
a reasonable state for a crate whose consumer half is still being wired, and it
is worth knowing which methods are load-bearing today and which are a surface
waiting for a caller.

### Coverage

| Method | Tests | Doctests | Notable |
|--------|------:|---------:|---------|
| `frontier` | 16 call sites | 1 | Every set size 1–8 with the minimum at every index |
| `available` | 8 | 1 | The 4-versus-1,000 assertion; saturation at and past the frontier |
| `admits` | 3 | 1 | 20,736 combinations — and see below |

### BR18 — `admits_and_available_never_disagree` Restates the Body

```rust
// tests/barrier_test.rs:212-216
assert_eq!(
  barrier.admits( Seq( from ), count ),
  count <= barrier.available( Seq( from ) ),
  "deps {first}/{second}, from {from}, count {count}"
);
```

versus

```rust
// ring_barrier/src/lib.rs:238-241
pub fn admits( &self, from : Seq, count : u64 ) -> bool
{
  count <= self.available( from )
}
```

The right-hand side of the assertion is the body of the method on its left. For
the implementation as written, the test **cannot fail** — twelve-cubed
dependency/position/count combinations across two moving cursors, all of them
comparing an expression to itself.

That is the same shape as the producer half's `headroom <= CAPACITY`
([`invariant/001`](../invariant/001_the_frontier_never_exceeds_a_dependency.md)),
but with a much better justification, so it is worth separating the two:

| | `headroom <= CAPACITY` | `admits == ( count <= available )` |
|--|------------------------|-------------------------------------|
| True by construction today | ✔ | ✔ |
| Would still be true after the obvious optimization | ✔ | **✘** |
| Guards a plausible future change | ✘ | ✔ |

The obvious optimization here is real and already motivated: `admits` allocates
because `available` allocates, and `wait_for` calls `admits` once per spin
([`non_functional_requirement/001`](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md)).
The natural fix is to hoist the frontier read and compare against it directly —
at which point `admits` stops being defined in terms of `available`, the
assertion stops being a tautology, and 20,736 cases start doing work. Written
before the optimization, it is a regression guard for a change nobody has made
yet.

It should be read as that, not as evidence about the current code. Today it
asserts nothing.

### Two of the Three Names Are Taken — [BR11](../pattern/002_the_quantity_the_predicate_and_the_wait.md)

Recorded as a finding in
[`pattern/002`](../pattern/002_the_quantity_the_predicate_and_the_wait.md),
which is BR11's home; this section is the same name census read from the three
readings' side, and adds the call-site risk below rather than a second finding.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'fn admits\|fn available(\|fn frontier' ring_*/src/*.rs
```

Live output:

```
ring_barrier/src/lib.rs:  pub fn frontier( &self ) -> Option< Seq >
ring_barrier/src/lib.rs:  pub fn available( &self, from : Seq ) -> u64
ring_barrier/src/lib.rs:  pub fn admits( &self, from : Seq, count : u64 ) -> bool
ring_bench/src/lib.rs:  pub const fn admits( self, producers : usize ) -> bool
ring_consume/src/lib.rs:  pub fn available( &self ) -> Available
ring_gating/src/lib.rs:  pub fn admits( &self, producer : Seq, count : usize ) -> bool
ring_mpsc/src/lib.rs:  pub fn available( &self ) -> usize
ring_spsc/src/lib.rs:  pub fn available( &self ) -> usize
```

| Name | Definitions | Signatures |
|------|------------:|------------|
| `available` | **4** | `Barrier -> u64`, `ring_consume::Consumer -> Available`, `ring_mpsc -> usize`, `ring_spsc -> usize` |
| `admits` | **3** | `Barrier( Seq, u64 ) -> bool`, `ring_gating::GatingSet( Seq, usize ) -> bool`, `ring_bench::Candidate( usize ) -> bool` |
| `frontier` | **1** | this crate only |

`available` is the worse of the two: four methods, three different return types,
and one of them (`ring_consume`'s) computed *from* another one of them
(`Barrier`'s) while returning something entirely different.

The `admits` pair is the more dangerous at a call site, though, because the two
gating variants differ only in the width of their second parameter:

```rust
set.admits( producer, n )       // GatingSet — n : usize
barrier.admits( from, n )       // Barrier   — n : u64
```

An integer literal type-checks against both. Nothing but the receiver's type
tells a reader which question is being asked — *may the producer claim n slots*
or *may this consumer read n sequences* — and those have opposite relationships
to capacity ([`invariant/002`](../invariant/002_capacity_never_enters_the_arithmetic.md)).

The one name with a single definition is `frontier`, which is the one this crate
invented rather than inherited from the family's shared vocabulary. The
vocabulary is what collided; the new word did not.

### BR38 — The Three Readings Are One Reading Called Three Times

`frontier` folds. `available` calls `frontier`. `admits` calls `available`.
Each adds exactly one operation — a fold, a subtraction, a comparison — and each
is a separate public method with its own doctest.

The layering means the cheapest question a caller can ask costs the most
expensive one plus two arithmetic steps: `admits( from, 0 )`, which is a
constant `true`, still folds every cursor in the slice. There is no way to ask
the comparison without paying the fold, and the batching caller that wants both
the frontier and the admission answer must either take the fold twice or reach
past `admits` to `frontier` and re-derive the comparison itself — which is what
`ring_consume::available` does.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A25 -F '  pub fn available( &self, from : Seq ) -> u64' ring_barrier/src/lib.rs | grep -E "self\.|count <="
# the downstream crate that re-derives rather than paying twice
grep -A6 "pub fn available( &self ) -> Available" ring_consume/src/lib.rs
```

Live output:

```
    self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
    count <= self.available( from )
  pub fn available( &self ) -> Available
  {
    let position = self.position();
    let readable = self
      .barrier
      .frontier()
      .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );
```

### Items

| File | Relationship |
|------|--------------|
| [002_the_five_accessors_and_the_wait.md](002_the_five_accessors_and_the_wait.md) | The other six methods |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_frontier_in_two_delegations.md](../algorithm/001_the_frontier_in_two_delegations.md) | The chain all three descend |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | The three in the context of nine |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_zero_for_a_barrier_over_nothing.md](../decisions/001_zero_for_a_barrier_over_nothing.md) | The empty-barrier row of the table above |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_three_dependencies_and_one_dependent.md](../integration/001_three_dependencies_and_one_dependent.md) | `ring_consume` reaching `frontier` past `available` |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_frontier_never_exceeds_a_dependency.md](../invariant/001_the_frontier_never_exceeds_a_dependency.md) | What all three guarantee |
| [../invariant/002_capacity_never_enters_the_arithmetic.md](../invariant/002_capacity_never_enters_the_arithmetic.md) | The difference the `admits` collision hides |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_frontier_read_allocates_nothing.md](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) | The optimization BR18 is a guard for |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_quantity_the_predicate_and_the_wait.md](../pattern/002_the_quantity_the_predicate_and_the_wait.md) | The ladder, and its inconsistent naming across the family |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:169-241` | The three methods |
| `ring_gating/src/lib.rs:242` | The colliding `admits` |
| `ring_consume/src/lib.rs:336` | The colliding `available` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:71-128` | `frontier` — sizes, positions, a lagging dependency |
| `tests/barrier_test.rs:152-194` | `available` — range, saturation, and the capacity contrast |
| `tests/barrier_test.rs:196-230` | `admits` — the tautology, and the zero-count case |
