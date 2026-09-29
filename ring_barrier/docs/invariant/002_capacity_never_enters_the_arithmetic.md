# Invariant: Capacity Never Enters the Arithmetic

### Scope

- **Purpose**: State the crate's defining absence as a property, show what it protects, and be precise about which instrument is actually holding it.
- **Responsibility**: Give the property, the grep that used to carry it, the assertion that carries it now, and what a violation would look like.
- **In Scope**: The absence of `Capacity` from every signature, body, and doc example.
- **Out of Scope**: The manual check's own weakening — see [`workaround/001`](../workaround/001_the_check_that_capacity_stays_out.md).

### The Property

> A barrier answer is bounded by what dependencies have finished, never by how
> many slots the ring has.

Stated as an absence, because that is how it is enforced: `Capacity` is not
imported, not a parameter, not a field, and not named anywhere in the crate.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs | grep -E "capacity|Capacity" \
  || echo '(capacity appears nowhere in this crate outside comments)'
# control: the identical expression over the crate that owns the bound
grep -vE "^[[:space:]]*//" ring_gating/src/lib.rs | grep -cE "capacity|Capacity"
```

Live output:

```
(capacity appears nowhere in this crate outside comments)
12
```

### Why It Is Not Obvious

`Barrier` and `GatingSet` look interchangeable. Both hold cursors, both take
a minimum, both answer "how far may I go". The module documentation is blunt
about where they part:

> A gating answer is bounded by the ring's capacity: the producer may run one
> full lap ahead of the slowest consumer and no further […] Capacity is in the
> answer.
>
> A barrier answer has no capacity in it at all. A consumer may read up to
> whatever its dependencies have finished […] and the number of slots the ring
> happens to have does not enter into it.
>
> — `ring_barrier/src/lib.rs:14-25`

And the difference is invisible in every test where the frontier happens to sit
under one lap — which is most of them, because most tests use small numbers.

### The Assertion That Makes It Visible

```rust
// tests/barrier_test.rs:185-193
let set = GatingSet::new( cap( 4 ), 1 );
set.cursor( 0 ).unwrap().store( Seq( 1_000 ), Ordering::Release );

assert_eq!( set.headroom( Seq( 1_000 ) ), 4, "the producer is clamped to one lap" );
assert_eq!(
  Barrier::over( set.cursors() ).available( Seq::ZERO ),
  1_000,
  "the consumer is not clamped at all"
);
```

One `GatingSet`, capacity 4, one cursor at 1,000 — read from both sides in one
test. The producer is told 4. The consumer is told 1,000. **250× apart, from the
same cursors**, and the gap grows without bound as the frontier advances.

That is the property made falsifiable. A `Barrier::available` that clamped —
however it acquired a capacity to clamp with — would answer 4 here and fail.

### Why the Grep Is No Longer the Instrument

Manual check B1 runs the grep above and expects no output. It passes, and its
own note says why that is weaker than it sounds:

> `Capacity` is not imported and does not appear at all, not even in a doc
> example — which is a weaker result than it was before the signature changed
> […] While `Barrier::over` took a `&GatingSet`, every doc example here had to
> build one, so a capacity was in the file and this check was reading real
> restraint. Now the constructor takes a bare `&[ PaddedCursor ]` and there is
> nothing to build; the check passes because capacity has no way in, not because
> it was kept out.
>
> — `tests/manual/readme.md` § B1

A check that cannot fail is not evidence. B1 remains worth running — it is the
tripwire for a capacity being *added* — but the property itself is carried by
`available_ignores_capacity_entirely`, and the manual plan says so in the same
paragraph. That honesty about an instrument's own weakening is
[`workaround/001`](../workaround/001_the_check_that_capacity_stays_out.md).

### What a Violation Would Look Like

| Violation | Symptom | Caught by |
|-----------|---------|-----------|
| `available` clamped to a capacity | A consumer stops reading at one lap while data is available beyond it | `available_ignores_capacity_entirely` |
| `Capacity` added as a field | The type grows past 16 bytes; `Copy` survives; nothing else changes | B1, plus [`data_structure/001`](../data_structure/001_one_field_and_a_sixteen_byte_view.md)'s size |
| `over` gains a capacity parameter | Every one of the 63 call sites changes | The compiler |
| A doc example builds a `GatingSet` | Capacity re-enters the file harmlessly | B1 — and this is the false positive B1 will one day produce |

The last row is worth naming: B1 greps the whole file with a comment filter that
drops `///`, so a doc example *would* be caught, and a doc example that built a
gating set to demonstrate the contrast would be a legitimate thing to write. The
check would then be failing on correct code, which is how a source reading gets
deleted rather than fixed. If that day comes, the fix is to narrow B1's scope to
signatures and bodies, not to delete it.

### The Consequence Callers Must Absorb

`available` can exceed the ring's capacity, by any amount. A consumer that uses
it to size a read must clamp it itself:

| Caller | Clamps how |
|--------|-----------|
| `ring_consume::Consumer::available_up_to( max )` | The caller passes the bound |
| `ring_consume::Consumer::available` | Does not clamp — returns the full run |
| A caller indexing a fixed buffer | Must clamp, or index out of range |

That is the correct division. The barrier reports what is *readable*; how much
of it fits anywhere is a fact about the reader's storage, which the barrier has
never been told about and should not guess.

### BR37 — The Crate's Twenty-Three Tests Are Compiled Out of Every Loom Run

`tests/barrier_test.rs` is gated off under `--cfg loom`, with a comment
explaining that the cfg swaps `ring_atomic`'s atomics for loom's and that
ordinary tests would die outside a `loom::model` closure. The gate is correct
and every crate in the family needs one.

What follows from it is that the crate whose invariant is *a consumer never
reads past a dependency* — a property about two threads racing on the same
cursors — contributes twenty-three tests to ordinary runs and a model to none.
The one test that does run two threads,
`a_consumer_waiting_on_a_producer_thread_makes_progress`, is exactly the test a
loom model would replace, and it asserts progress rather than ordering.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A8 -F '//! it against nothing.' ring_barrier/tests/barrier_test.rs
grep -c "^fn \|#\[ test \]" ring_barrier/tests/barrier_test.rs
# any loom model in this crate
grep "loom::model" ring_barrier/tests/barrier_test.rs \
  || echo '(no loom model in this crate)'
# control: the identical expression over a crate that has one
grep -rc "loom::model" ring_cursor/tests/ 2>/dev/null | grep -v ':0$' | head -2
```

Live output:

```
//! it against nothing.

// Ordinary tests, compiled out under `--cfg loom`. That cfg swaps
// `ring_atomic`'s atomics for loom's instrumented ones across the whole
// family, and those panic the moment they are touched outside a
// `loom::model` closure — so without this gate a family-wide loom run dies
// here instead of reaching the models in `ring_spsc`, `ring_mpsc`,
// `ring_publish` and `ring_testkit`.
#![ cfg( not( loom ) ) ]
49
// `loom::model` closure — so without this gate a family-wide loom run dies
ring_cursor/tests/cursor_test.rs:1
```

### Invariants

| File | Relationship |
|------|--------------|
| [001_the_frontier_never_exceeds_a_dependency.md](001_the_frontier_never_exceeds_a_dependency.md) | The crate's other property, and its own instrument problem |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_frontier_in_two_delegations.md](../algorithm/001_the_frontier_in_two_delegations.md) | The missing clamp, as a missing step |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | Nine signatures, none naming a capacity |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_field_and_a_sixteen_byte_view.md](../data_structure/001_one_field_and_a_sixteen_byte_view.md) | The field a capacity would have to join |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_zero_for_a_barrier_over_nothing.md](../decisions/001_zero_for_a_barrier_over_nothing.md) | The empty-case alternative this absence rules out |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_dependency_that_is_not_ring_seq.md](../integration/002_the_dependency_that_is_not_ring_seq.md) | The crate the clamp would have cost |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_u64_distance_and_a_usize_headroom.md](../type/001_a_u64_distance_and_a_usize_headroom.md) | The width the missing clamp leaves behind |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_check_that_capacity_stays_out.md](../workaround/001_the_check_that_capacity_stays_out.md) | B1, and what it can and cannot read |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:12-28` | The module's own statement |
| `ring_barrier/src/lib.rs:196-219` | `available`, with nothing to clamp with |
| `ring_gating/src/lib.rs` | `headroom`, with everything to clamp with |
| `tests/manual/readme.md` § B1 | The grep, and its own account of its weakening |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:185-193` | The 4-versus-1,000 assertion |
| `tests/barrier_test.rs:164-177` | `available` at and past the frontier — saturation without a clamp |
