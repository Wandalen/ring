# Pitfall: The Criterion Grades the Clamped Value

### Scope

- **Purpose**: Record that two of `RingConfig`'s five fields are silently corrected before `build` ever sees them, so this crate's own acceptance criterion — "observable behaviour matches every field" — is satisfiable without the ring behaving as the caller asked.
- **Responsibility**: Name the trap, the failures it produces, and the mitigations, including which ones are not this crate's to apply.
- **In Scope**: `with_batch`'s cap; `with_producers`' floor; what the acceptance criterion can and cannot see.
- **Out of Scope**: Whether clamping is the right choice, which is `ring_config`'s decision (→ [`ring_config`](../../../ring_config/readme.md)); `Capacity`'s rejection, which is loud and therefore not a trap.

### Trap

**This crate's own criterion reads as a strong guarantee:**

> `Factory::build(cfg)` returns a handle pair whose observable behaviour
> matches every field, asserted one field at a time

**It is a weaker guarantee than it sounds, because two fields are not
necessarily the ones the caller wrote.** `RingConfig`'s setters correct rather
than reject — verify against the crate's own doctests, which assert exactly
this:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- with_batch: floor and cap --'
command grep -A2 'with_batch(0).batch(), 1' ring_config/src/lib.rs
echo '  -- with_producers: floor --'
command grep 'with_producers(0).producers(), 1' ring_config/src/lib.rs
```

Live output:

```
  -- with_batch: floor and cap --
    /// assert_eq!(cfg.with_batch(0).batch(), 1);
    /// assert_eq!(cfg.with_batch(8).batch(), 8);
    /// assert_eq!(cfg.with_batch(999).batch(), 16);
  -- with_producers: floor --
    /// assert_eq!(RingConfig::new(8).unwrap().with_producers(0).producers(), 1);
```

So a caller who asks for a batch of 999 on a sixteen-slot ring gets a batch of
16, and `build` receives a record in which `batch == 16`. The factory then
constructs a ring whose observable batching is 16, and an assertion comparing
observed behaviour against `cfg.batch()` passes.

**Both sides of the comparison are the corrected value.** The number the caller
actually wrote is not in the record, is not passed to `build`, and is
unrecoverable by the time this crate runs. The criterion compares the ring
against the config; it never compares the config against the request.

**This is not a defect in `ring_config`.** Clamping keeps the setters
infallible so a builder chain needs no `?` in its middle, which is a defensible
trade and is stated as such in that crate's own doc comments. The trap is what
happens at *this* crate's acceptance boundary, where a criterion that sounds
like end-to-end fidelity is in fact a two-hop comparison with the first hop
missing.

**`Capacity` is the contrast that shows the shape.** `RingConfig::new( 63 )`
returns `Err` — a non-power-of-two capacity is refused, loudly, at the point of
request. That field cannot silently differ from what was asked for. Two of the
five behave one way and one behaves the other, and nothing in the acceptance
row distinguishes them.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| C1 | A sweep configuration requests a batch larger than capacity | The run is recorded under the requested batch and executes at the capped one. **The benchmark's axis label is wrong** |
| C2 | Two sweep points clamp to the same value | Two rows in the results table that differ in their label and not in what ran. The comparison between them is noise |
| C3 | A caller reads back `cfg.batch()` and sees their own value | They will not — `with_batch` returns the corrected `Self`, so a caller who inspects gets the truth. **This is the failure that does not happen**, and it is why the trap is at the acceptance boundary rather than in the API |
| C4 | `with_producers( 0 )` on a config built from a manifest field left empty | Silently becomes a single-producer ring. The SPSC backend is selected (→ [`algorithm/001`](../algorithm/001_selecting_a_backend_from_one_boolean.md)) and the run measures the wrong primitive entirely |
| C5 | A future field is added with clamping semantics | Inherits the same gap with no new test failing |

**C1 and C2 are the ones that matter, because this family exists to produce
measurements.** This family's own benchmark sweep produces a comparison table across
configurations. A configuration axis that silently collapses at the edges
produces a table whose rows are not what they say they are, and no test in
either crate goes red.

**C4 is the sharpest instance**, because the clamp changes not a parameter but
the *backend*. `producers: 0` and `producers: 1` are the same ring;
`producers: 1` and `producers: 2` are different implementations. A manifest
that omits the field gets SPSC, and the sweep records it under whatever the
manifest said.

**C3 is listed because it is the reassuring case and it is genuinely
reassuring.** The setters return the corrected value, so nothing lies to a
caller who looks. The gap is only between *what was requested* and *what is
inspectable*, and requests are not values anything holds.

### Mitigation

**What does not work:**

| Attempt | Why it fails |
|---------|--------------|
| Assert in `ring_factory`'s test that `cfg.batch() == 999` | The config never carried 999. There is nothing to assert against |
| Have `build` reject a clamped config | `build` cannot tell. A config with `batch == 16` on a 16-slot ring is indistinguishable from one that asked for 16 |
| Make the setters fallible | `ring_config`'s decision, already made, with a stated reason. Reversing it is that crate's call and would put a `?` in the middle of every builder chain |
| Widen this crate's own acceptance criterion here | The acceptance table is shared and family-scoped (→ [`bench_harness`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md)). One crate does not amend it |

**What works:**

1. **The sweep records the config, not the request.** Whatever drives the
   benchmark matrix reads `cfg.batch()` and `cfg.producers()` back out *after*
   construction and labels the run with those, so C1 and C2 become impossible
   by construction rather than by discipline. This is `ring_bench`'s to do and
   is recorded here because this instance is where the need is visible.
2. **A construction-time report.** `build` returning what it built with — or
   the factory exposing `cfg` on the handle pair — makes the effective
   configuration inspectable downstream. This is cheap and is not currently in
   this crate's own acceptance criterion.
3. **The one assertion that is available here:** that `build` honours the
   config it is given, exactly. That is what "one field at a time" tests, and
   it is worth having; it is simply a narrower claim than the sentence reads.

**None of the three closes C4.** A manifest field left empty produces a valid
single-producer config that no layer can distinguish from an intentional one.
Closing it means the manifest layer distinguishing absent from zero, which is
`lang_channel`'s problem and does not exist yet.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_selecting_a_backend_from_one_boolean.md](../algorithm/001_selecting_a_backend_from_one_boolean.md) | C4's mechanism — the clamp reaches the branch |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_configuration_record_as_input.md](../data_structure/001_the_configuration_record_as_input.md) | The record's five fields and which of them are corrected on the way in |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_configuration_fully_determines_the_ring.md](../invariant/001_configuration_fully_determines_the_ring.md) | Holds regardless — equal configs build equal rings whether or not either was clamped |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md](../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md) | The criterion this trap is about, stated as a measurable requirement |

### Pitfalls

| File | Relationship |
|------|--------------|
| [002_a_wait_strategy_it_can_read_and_cannot_honour.md](002_a_wait_strategy_it_can_read_and_cannot_honour.md) | The other trap — a field readable and unhonourable, rather than a field silently altered |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_config_state_through_a_build.md](../lifecycle/003_config_state_through_a_build.md) | The state reachable only because validation happens in another crate |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | Row 180's criterion, quoted above |
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | `with_batch` and `with_producers`, whose doctests are this instance's evidence |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | ⚠️ Mitigation 3 asked for an assertion that `build` honours the config exactly, with no over-claiming in the test's documentation. **The assertion cannot be written for three of the five fields**, so what the suite records instead is precisely which two it *can* honour observably — `only_two_of_five_config_fields_are_observable_through_the_factory`. That is the non-over-claiming version of the requested test, arrived at by trying to write the requested one |

### FC41 — The Two Clamped Fields Are Exactly the Two Nothing Reads, So the Pitfall Is Dormant Rather Than Absent

The correction is silent and it currently cannot change a ring:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two silently corrected fields --'
command grep 'self.producers = \|self.batch = ' ring_config/src/lib.rs
echo '  -- and what reads them on the build arc --'
for c in ring_factory ring_core; do
  printf '  %-13s producers(): %s  batch(): %s\n' "$c" \
    "$( command grep -vE '^\s*(///|//!)' $c/src/lib.rs | command grep -c 'producers()' )" \
    "$( command grep -vE '^\s*(///|//!)' $c/src/lib.rs | command grep -c 'batch()' )"
done
```

Live output:

```
  -- the two silently corrected fields --
        self.producers = if producers == 0 { 1 } else { producers };
        self.batch = if capped == 0 { 1 } else { capped };
  -- and what reads them on the build arc --
  ring_factory  producers(): 0  batch(): 0
  ring_core     producers(): 0  batch(): 0
```

Zero readers of either accessor. `producers` reaches the branch only through the
derived `is_multi_producer()`, and `batch` reaches nothing at all
(→ [`algorithm/002`](../algorithm/002_assembling_a_ring_from_a_validated_record.md)
FC4).

So today: `with_batch( 999 )` on an 8-slot ring clamps to 8, and the ring is
identical to one built without the call, because the clamped value is discarded
either way. The caller's request was corrected *and* ignored, and the two errors
cancel into a correct ring.

That is worse than it sounds for the pitfall's own purposes. The failure mode
this instance warns about — a criterion graded against a value the caller did
not write — cannot currently fire, so any test written for it passes for the
wrong reason, and it will start firing the day `batch` becomes load-bearing,
which is a change in a different crate that nothing here will observe. The
mitigation this instance needs is not a test; it is that whoever wires `batch` to
a ring reads this file, and nothing routes them here.

### FC42 — There Is a Legality Check on `RingConfig` That No Build Path Calls

`is_tick_safe` exists, is `const`, and is reachable from every crate that can
name a `RingConfig`:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the check --'
command grep 'pub const fn is_tick_safe' ring_config/src/lib.rs
echo '  -- every caller in the family --'
command grep -r 'is_tick_safe' --include=*.rs ring_*/src/ ring_*/tests/ \
  | sed 's|ring/||' | command grep -v 'ring_config/src/lib.rs'
echo '  -- for contrast, the derived reading that is used --'
command grep -rl 'is_multi_producer' --include=*.rs ring_*/src/ | sed 's|ring/||;s|/src/.*||' | tr '\n' ' '
echo
```

Live output:

```
  -- the check --
  pub const fn is_tick_safe( &self ) -> bool
  -- every caller in the family --
ring_config/tests/config_test.rs:      cfg.with_wait( kind ).is_tick_safe(),
ring_config/tests/config_test.rs:  assert!( cfg.with_wait( WaitKind::None ).is_tick_safe() );
ring_config/tests/config_test.rs:  assert!( !cfg.with_wait( WaitKind::Spin ).is_tick_safe() );
  -- for contrast, the derived reading that is used --
ring_config ring_core 
```

`RingConfig` offers two derived readings. One decides the backend and is called
by `ring_core`; the other reports whether a configuration is safe for the tick
path and is called by nothing on any build arc.

This is the same shape as the clamping pitfall from the opposite side. There, a
value the caller wrote is silently replaced; here, a judgement the record can
make about itself is silently unused. Both leave `build` returning `Ok` for a
configuration that is not what the caller believes it is, and neither is visible
in the result.

The distinction worth keeping is that this one is cheap to close and the clamp is
not. `is_tick_safe` returning `false` could be a `BuildError` variant in three
lines — it would be the first refusal this crate *owned* on the unnamed path
(→ [`type/002`](../type/002_build_error.md)) — and nothing has ruled either way,
because no pending records the question.
