# Non-Functional Requirement: Five Fields, Asserted One at a Time

### Scope

- **Purpose**: Turn this crate's own acceptance row into a measurable requirement, field by field, and record that only three of the five admit the assertion the row describes.
- **Responsibility**: State the quality attribute, the statement, the measurement method, and the acceptance threshold.
- **In Scope**: What "observable behaviour matches every field" means per field; what a round-trip assertion does and does not establish.
- **Out of Scope**: Construction cost (→ [`non_functional_requirement/002`](002_construction_cost_is_paid_once.md)); whether the field holds the requested value (→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)).

### Quality Attribute

**Verifiability.** The requirement is not that the factory be fast, small, or
robust — it is that each configuration field's effect be *demonstrable* by an
assertion that fails when the field is ignored.

This is the attribute the whole family's acceptance table is built on: a feature
is Reached when a named test asserts it, and the value of the table depends
entirely on whether those assertions can fail for the right reason.

### Statement

This crate's own acceptance row, quoted exactly:

> `RingConfig` carries capacity, wait kind, overflow policy, producer count and
> batch size, and is the only constructor input; `Factory::build(cfg)` returns a
> handle pair whose observable behaviour matches every field, asserted one field
> at a time

**"One field at a time" is the load-bearing phrase and it is a real
constraint.** It rules out the cheap version — build one ring with all five
fields set unusually, observe that it behaves unusually, and declare the row
satisfied — which would pass while four of the five fields did nothing. Varying
one field with the other four held fixed is what makes each field's contribution
attributable.

**The requirement, restated per field:**

| Field | The assertion that would satisfy the row | Available |
|-------|------------------------------------------|-----------|
| `capacity` | A ring of 16 accepts 16 records and refuses the 17th; a ring of 32 accepts 32 | **Yes** |
| `overflow` | With `Fail`, the overflowing publish reports failure; with `DropNewest`, it does not and the record is gone; with `DropOldest`, the *oldest* is | **Yes** — three variants, three distinguishable outcomes |
| `batch` | A drain with batch 4 yields at most 4; with batch 16, up to 16 | **Yes** |
| `producers` | Two producer threads both publish successfully; **at one producer, the SPSC backend is selected** | **As a boolean only** (→ [`algorithm/002`](../algorithm/002_assembling_a_ring_from_a_validated_record.md)) |
| `wait` | A `None` ring's empty consume returns immediately; a `Park` ring's does not | **No.** No waiter is constructed (→ [`pitfall/002`](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md)) |

**Three of five are straightforward, one is degraded, one is unavailable.**
Stating that here is the point of this instance: the acceptance row reads as
five equivalent obligations and is three, plus one weaker one, plus one that
cannot currently be met at all.

### Measurement Method

**The assertion must be behavioural, not a round-trip.** The distinction
decides whether this requirement is real:

| Kind | Shape | What it proves |
|------|-------|----------------|
| Round-trip | `assert_eq!( cfg.batch(), 4 )` after building with batch 4 | **Nothing about the ring.** `RingConfig` is `Copy`; this asserts that a value the test itself wrote is still what it wrote |
| Behavioural | Publish 10, drain once, `assert_eq!( drained.len(), 4 )` | That the ring drains at most `batch` — which is the field's meaning |

**A round-trip assertion passes against a `build` that discards its argument
entirely**, which is precisely the failure the row exists to exclude. The test
must reach through the returned handles and observe the ring.

**Method, per field:**

1. **`capacity`** — build at 16; publish until refusal; assert the refusal came
   at 17, not earlier or later. Repeat at 32. Two points, because one is
   consistent with a hard-coded capacity.
2. **`overflow`** — build three rings differing only in policy; fill each; issue
   one more publish; assert three distinct outcomes. The three-way distinction
   is what makes this stronger than the others: a single ignored field cannot
   produce three different results.
3. **`batch`** — build at batch 4 and batch 16 on the same capacity; publish
   more than 16; drain once each; assert the yields differ and match.
4. **`producers`** — build at 1 and at 4; assert the 4-producer ring survives
   two concurrent publishing threads and the 1-producer ring is a different
   type or reports differently. **Do not assert anything distinguishing 4 from
   8** — nothing does.
5. **`wait`** — **not measurable today.** The honest test is one that does not
   exist, and its absence must be visible rather than papered over with a
   round-trip.

**Method 2 is the strongest available and is worth copying.** Three variants
producing three distinguishable outcomes cannot be satisfied by accident; a
two-valued field always can be, by a coin flip.

### Acceptance Threshold

**The row is Reached when four of the five fields have a behavioural
assertion, and the fifth's absence is recorded rather than faked.**

| # | Threshold | Rationale |
|---|-----------|-----------|
| A1 | `capacity`, `overflow`, `batch` each have a behavioural assertion at two or more distinct values | One value is consistent with a constant |
| A2 | `producers` has a behavioural assertion at 1 and at ≥2, and its test documents that above 1 the count is not observable | Prevents a later reader from mistaking the boolean check for a count check |
| A3 | `wait` has **no** passing assertion, and this is stated in the test file | A round-trip assertion here would convert a known gap into an apparent pass |
| A4 | No assertion in the file is a round-trip on a field | Round-trips are the specific failure this requirement guards |
| A5 | The test file's own header names the requirement it verifies | The family's one crate→feature edge |

**A3 is the threshold that will be argued with, and it is the important one.**
The pressure to write `assert_eq!( cfg.wait(), WaitKind::Park )` and call
this row Reached will be real, because it makes the row green. It would make
the acceptance table say the wait field's behaviour was verified when no wait
behaviour exists.

**The threshold is deliberately not "all five behavioural".** That would make
this row unreachable until `ring_wait` is in the closure, blocking a crate on
a dependency question that has not been ruled
(→ [`decisions/`](../decisions/readme.md)). Four-plus-a-recorded-gap is
achievable now and honest; five is the eventual target.

**What this threshold does not cover:** whether any field holds the value the
caller requested. Every assertion above compares the ring against `cfg`, and
`cfg` is what `build` received — not what was asked for
(→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)).
That gap is outside this requirement by construction and cannot be closed from
inside it.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_assembling_a_ring_from_a_validated_record.md](../algorithm/002_assembling_a_ring_from_a_validated_record.md) | Why `producers` is a boolean and `wait` is nothing |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | The surface every assertion goes through |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_configuration_record_as_input.md](../data_structure/001_the_configuration_record_as_input.md) | U1–U8, and which of them a behavioural assertion can reach |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_configuration_fully_determines_the_ring.md](../invariant/001_configuration_fully_determines_the_ring.md) | V5 — the invariant and this requirement fail independently |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_construction_cost_is_paid_once.md](002_construction_cost_is_paid_once.md) | The companion requirement, which this crate's own acceptance criteria do not state at all |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_configuration_as_data.md](../pattern/001_configuration_as_data.md) | Cost 3 — a field readable and unhonoured; this requirement is the discipline that answers it |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_criterion_grades_the_clamped_value.md](../pitfall/001_the_criterion_grades_the_clamped_value.md) | The gap this threshold explicitly does not cover |
| [../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md) | A3's subject |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | Row 180, quoted under Statement |
| [`ring_types/src/policy.rs`](../../../ring_types/src/policy.rs) | `OverflowPolicy`'s three variants — method 2's strength |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | A1–A5 written one field at a time as required, and **three of the five assert the field's *absence* from the observable surface rather than its honouring** — `assert_eq!` between two profiles that differ only in that field. That is a real, failing-if-wrong assertion, not a comment standing in for one: if `producers`, `wait` or `batch` ever reaches the output, the test breaks and says so. The one guard the shape needs is anti-vacuity — a saturated profile compares equal to everything — which is why the helper refuses a capacity that cannot overflow |

### FC33 — The Requirement Is Capped at Two Fields by the Return Type, Not by the Tests

Three of the five assertions assert absence. That is usually a sign the tests
need more work; here it is a ceiling the return type imposes:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what a built ring exposes of its config --'
command grep 'overflow : config.overflow()\|storage,' ring_core/src/lib.rs
echo '  -- and the fields the record carries --'
sed -n '/^pub struct RingConfig/,/^}/p' ring_config/src/lib.rs | command grep -E '^  [a-z_]+ :'
```

Live output:

```
  -- what a built ring exposes of its config --
/// The storage, owning whichever backend the configuration selected.
    Ok( Self { storage, overflow : config.overflow() } )
        overflow : config.overflow(),
  -- and the fields the record carries --
  capacity : Capacity,
  wait : WaitKind,
  overflow : OverflowPolicy,
  producers : usize,
  batch : usize,
```

`Ring` stores the storage and `overflow`. The other four fields are consumed
during construction and not retained — `capacity` survives only as the buffer's
length, `producers` only as which variant `storage` holds, and `wait` and
`batch` not at all.

So "observable behaviour matches every field" is measurable for two fields and
unmeasurable for three, and no amount of test-writing changes that. Raising the
number means changing what a ring retains or what a `Split` exposes, which is a
design change to two other crates.

Stating it this way fixes what the three absence-assertions are worth. They are
not weaker versions of the assertions that could not be written — they are the
strongest true statement available, and their value is that they will *fail* the
day one of those three fields becomes observable, which is exactly when this
requirement should be revisited. An absence assertion that would break on
progress is a good absence assertion.

### FC34 — Two Doctests Are the Only Tests of the Documented Call Shape

Nineteen `#[ test ]` functions and two doctests, and the split matters more than
the counts:

```sh
cd "$(git rev-parse --show-toplevel)"
printf '  #[ test ] functions:  %s\n' \
  "$( command grep -c '^#\[ test \]' ring_factory/tests/factory_test.rs )"
printf '  fn declarations:      %s\n' \
  "$( command grep -cE '^fn [a-z_0-9]+\(' ring_factory/tests/factory_test.rs )"
printf '  doctest fences in src: %s\n' \
  "$( command grep -c '/// ```' ring_factory/src/lib.rs )"
echo '  -- fence lines, then the method each block sits above --'
command grep -n '/// ```' ring_factory/src/lib.rs | sed 's/:.*//' | paste -sd' ' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
command grep -n '^  pub fn ' ring_factory/src/lib.rs | sed 's/(.*//;s/< S.*//' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  #[ test ] functions:  19
  fn declarations:      21
  doctest fences in src: 4
  -- fence lines, then the method each block sits above --
139 149 181 193
  pub fn build
  pub fn build_named
  pub fn build_crossbeam
```

Twenty-one `fn` declarations, nineteen of them tests — the other two are the
shared `observable_profile` helper and its one-line wrapper, which is worth
knowing before reading `21` as a test count.

Four fences are two blocks, at 133–143 and 170–182, sitting immediately above
`build` (144) and `build_named` (183). **`build_crossbeam` (226) has none** — the
one public method whose call shape a reader is least likely to guess, because it
is feature-gated and its name is the only thing distinguishing it from `build`
(→ [`decisions/002`](../decisions/002_two_doors_not_one_that_routes.md) FC15), is
the one with no worked example.

The two blocks that exist are the only executable statements of how a caller is
meant to write the call. Every one of the nineteen integration tests reaches for
an assertion rather than a shape, so a change that kept behaviour and broke
ergonomics — a fourth argument, a different receiver — fails the doctests and
passes the suite.

That makes the doctests load-bearing for a property nothing else covers, and
they run only under `cargo test --doc`, which is a separate command from the one
the suite's nineteen use. A run of `cargo nextest run` alone reports green
without compiling either.
