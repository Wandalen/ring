# Pattern: Configuration as Data

### Scope

- **Purpose**: State the practice this crate is built on — the shape of a ring is a value, not a choice of constructor — and account for what that buys, what it forbids, and the one thing it makes impossible.
- **Responsibility**: State the problem, solution, applicability, and consequences.
- **In Scope**: Why a record; what `Copy` and `PartialEq` enable; the type-level guarantees given up.
- **Out of Scope**: The record's fields (→ [`data_structure/001`](../data_structure/001_the_configuration_record_as_input.md)); the single-entry restriction (→ [`pattern/002`](002_one_way_in.md)).

### Problem

**A ring has five independent knobs, so there are more legal shapes than anyone
will write constructors for.** Capacity times four wait strategies times three
overflow policies times a producer count times a batch size is not a set that
can be enumerated as functions.

The conventional answers all fail differently:

| Answer | Failure |
|--------|---------|
| One constructor per combination someone needed | This crate's own design rationale: "the set of legal configurations is whatever happens to have been written" |
| One constructor with five parameters | `Ring::new( 1024, Spin, DropNewest, 4, 16 )` — positional, unreadable, and every call site must be edited when a sixth knob arrives |
| A builder that terminates in `.build()` | Better, but the half-built builder is not a value: it cannot be compared, stored in a table, or enumerated |
| Compile-time generics per shape | Moves the choice to the call site and out of any runtime input entirely |

**And the requirement that rules out all four is not ergonomics — it is that
this family exists to produce a sweep.** This family's own benchmark sweep produces a comparison
table across configurations. A sweep needs its configurations to be *things*:
enumerable, comparable, storable in a row, printable in a results column,
readable from a manifest. None of the four answers above produces a thing.

### Solution

**Make the configuration a value, and make construction a function of it.**

```rust
let cfg = RingConfig::new( 1024 )?
  .with_wait( WaitKind::None )
  .with_overflow( OverflowPolicy::Fail )
  .with_producers( 4 )
  .with_batch( 16 );

let ( producer, consumer ) = factory.build( cfg );
```

The chain reads like a builder and differs from one in exactly the way that
matters: **every intermediate is a complete, legal, comparable `RingConfig`.**
There is no half-built state. `cfg` can be stored, cloned, compared to another,
put in a `Vec`, logged, and passed to `build` twice.

**Four properties do the work, and each is load-bearing:**

| # | Property | What it enables |
|---|----------|-----------------|
| C1 | `Copy` | `build( cfg )` by value at no cost; the caller keeps their copy (→ [`lifecycle/003`](../lifecycle/003_config_state_through_a_build.md)'s B5) |
| C2 | `PartialEq` + `Eq` | [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md) is statable as an equation |
| C3 | Total setters | The chain never needs `?` in its middle — bought by clamping (→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)) |
| C4 | Private fields | Derived readings like `is_multi_producer()` are indistinguishable from stored ones at the call site |

**C4 is the quietest and it is what lets the design evolve.** `producers` is a
stored field today and `is_multi_producer()` is derived from it; either could
become the other without touching a single caller. A `pub` field would have
frozen that choice at the first use.

### Applicability

**Use this when the shape of a thing must outlive the decision to build it.**
The test is whether anything other than the constructor needs to hold the
configuration: a sweep row, a manifest parser, a results label, a comparison. If
the answer is "only the constructor", a builder or a parameter list is simpler
and this pattern is overhead.

| Condition | Present here |
|-----------|--------------|
| More legal shapes than reasonable constructors | Yes — five independent knobs |
| Something other than the constructor holds the shape | **Yes — the sweep.** The decisive one |
| Shapes must be comparable | Yes — two sweep rows must be distinguishable |
| The shape arrives from outside the source | Yes, eventually — a manifest or a `lang_channel` declaration |
| The constructor is on an export Contract | Yes, and this raises a problem: `RingConfig` is not (→ [`api/001`](../api/001_the_build_surface.md)'s guarantee 3) |

**The last row is where this pattern and the export Contract collide.** Making
configuration a value means the value's *type* is part of the public surface.
`ring_config` is not among the five exported names, so a consumer cannot
currently name the type they must construct. That is not a flaw in the pattern —
it is the pattern's cost surfacing as a Contract question that this family's own Contract ruling
did not reach.

### Consequences

**What it buys:**

1. **The sweep is possible at all.** A configuration is a row; a table of rows
   is a sweep; a results table joins on them.
2. **`build` takes one argument forever.** A sixth knob is a field, not a
   parameter, and no call site changes.
3. **Equal configs build equal rings** is a statement that can be written down
   and tested (→ [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)).
4. **The legal set is enumerable.** What `RingConfig::new` accepts and the
   setters permit is the whole space, in one file.

**What it costs:**

1. **Illegal states become runtime concerns, not compile errors.** A
   type-per-shape design could make "MPSC ring with one producer" unrepresentable;
   here it is representable and merely means SPSC. The type system stops
   helping at the boundary of `RingConfig`.
2. **Clamping follows from C3.** Total setters and validation are in tension,
   and the resolution was to correct rather than refuse — which is the whole of
   [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md).
3. **A field can be read and not honoured, silently.** `wait` is in the record,
   is accessible, and reaches nothing (→ [`pitfall/002`](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md)).
   A parameter list would have made the gap a compile error at the call site;
   a record absorbs it.
4. **The record's type joins the public surface**, per Applicability's last row.

**Costs 1 and 3 are the same cost from two directions**, and they are worth
seeing together: the record decouples *stating* a configuration from *acting on*
one, which is precisely what makes the sweep possible and precisely what allows
a stated field to have no action behind it. There is no version of this pattern
that keeps the first property and removes the second.

**The mitigation is not a type-system fix; it is a test discipline.** Feature
180's "asserted one field at a time" is exactly the right response to cost 3 —
it forces each field to demonstrate an observable consequence — and its
weakness is that a round-trip assertion satisfies the letter without doing so
(→ [`non_functional_requirement/001`](../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md)).

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | B1, and the Contract collision under Applicability |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_configuration_record_as_input.md](../data_structure/001_the_configuration_record_as_input.md) | C1–C4 as concrete properties of the record |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_configuration_fully_determines_the_ring.md](../invariant/001_configuration_fully_determines_the_ring.md) | Benefit 3, and what C2 makes statable |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md](../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md) | The discipline that answers cost 3 |

### Patterns

| File | Relationship |
|------|--------------|
| [002_one_way_in.md](002_one_way_in.md) | The companion — this makes the shape a value, that makes the value's use exclusive |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_criterion_grades_the_clamped_value.md](../pitfall/001_the_criterion_grades_the_clamped_value.md) | Cost 2 |
| [../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md) | Cost 3 |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_config_state_through_a_build.md](../lifecycle/003_config_state_through_a_build.md) | B5 — C1's consequence for the caller's copy |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | The pattern's implementation |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | ✅ Benefit 3 is asserted by `two_rings_from_one_config_behave_identically`. **Cost 3 was exposed rather than papered over, and it is larger than this instance estimated** — three of the five fields have no observable effect through the factory's own output, not one. → `only_two_of_five_config_fields_are_observable_through_the_factory` |

### FC37 — The Pattern Makes an Illegal Configuration Representable, and One Actually Is

Configuration-as-data buys enumerability and pays for it by moving legality from
the type system into a runtime check. The bill is one variant:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the policy a legal RingConfig can hold and no in-house ring accepts --'
command grep -A4 'pub fn new( config : &RingConfig )' ring_core/src/lib.rs | command grep -E 'DropOldest|PolicyUnsupported'
for e in WaitKind OverflowPolicy; do
  printf '  -- %s --\n' "$e"
  sed -n "/^pub enum $e/,/^}/p" ring_types/src/policy.rs \
    | command grep -E '^  [A-Z][A-Za-z]*,' | tr -d ' ,' | paste -sd' ' | sed 's/^/    /'
done
```

Live output:

```
  -- the policy a legal RingConfig can hold and no in-house ring accepts --
    if config.overflow() == OverflowPolicy::DropOldest
      return Err( RingError::PolicyUnsupported );
  -- WaitKind --
    Spin Yield Park None
  -- OverflowPolicy --
    DropNewest DropOldest Fail
```

`OverflowPolicy::DropOldest` is a legal value of a legal field of a fully
validated record, and constructing a ring from it fails. A constructor-per-shape
design — the thing this pattern displaces — would have had no
`with_overflow( DropOldest )` to call in the first place, so the error would have
been a missing function rather than a returned `Err`.

That is the trade taken knowingly and it has a second-order cost the pattern
argument does not mention: because the refusal is a runtime `Err` rather than an
absent constructor, it must be *relayed* by every layer above it, which is why
`build` returns a `Result` at all (→ [`api/001`](../api/001_the_build_surface.md)'s
B2) and why `BuildError` exists as a type. One representable-but-illegal
combination is responsible for the entire error surface of the crate.

**Disposition:** declined — the finding is analytical, not a gap: it names a
trade this pattern already documents about itself. This file's own
Consequences section states the general form ("Illegal states become runtime
concerns, not compile errors... The type system stops helping at the
boundary of `RingConfig`"), and `build`'s own doc comment
(`ring_factory/src/lib.rs:143-148`) already carries a doctest that
constructs `OverflowPolicy::DropOldest` and asserts it returns
`BuildError::Unsupported( RingError::PolicyUnsupported )` — the exact
variant this finding traces. There is no source change this crate can make:
the finding's own text locates the fix one layer down, at whether
`OverflowPolicy` should stay a single enum spanning backends that only
partially support it, which is `ring_types`'/`ring_core`'s question, not
this crate's.

### FC38 — A Built Ring Keeps One of Its Five Fields, So Nothing Can Report What It Was Built From

The record is `Copy`, so the caller still holds theirs. The ring does not:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- everything a Ring retains --'
sed -n '/^pub struct Ring/,/^}/p' ring_core/src/lib.rs
echo '  -- what it was handed --'
command grep 'config\.' ring_core/src/lib.rs | command grep -v '///'
```

Live output:

```
  -- everything a Ring retains --
pub struct Ring< T >
{
  storage : Storage< T >,
  overflow : OverflowPolicy,
}
  -- what it was handed --
    if config.overflow() == OverflowPolicy::DropOldest
    let storage = match config.is_multi_producer()
    Ok( Self { storage, overflow : config.overflow() } )
          crossbeam_queue::ArrayQueue::new( config.capacity().get() ),
          config.capacity(),
        overflow : config.overflow(),
```

Storage and `overflow`. Four fields go in and one comes out the other side as a
stored value; `capacity` survives as a buffer length, `producers` as a variant
tag, and `wait` and `batch` not at all.

So configuration-as-data holds up to the boundary and stops there. A `RingConfig`
is comparable, storable and enumerable; a ring built from it carries no way back
to the value that produced it, and no API in the family takes a ring and returns
its config.

The consequence is on the sweep rather than on correctness. A benchmark
harness must carry the config alongside every ring it measures, because losing
the pairing loses the identity of the data point — and it must do so by
convention, since nothing in the types keeps them together. It is also the reason
Pending 5 (who carries `WaitKind` to the call site) is open at all: the natural
answer, "ask the ring", is unavailable by construction, and this is the finding
that says so in bytes rather than in argument.
