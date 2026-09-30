# API: The Five Classifier Predicates

### Scope

- **Purpose**: Record the crate's five `bool`-returning predicates as one surface, and the measurement that motivated grouping them: across all 33 crates, exactly one production call site exists for any of them — and it sits inside a method that is itself called only by tests.
- **Responsibility**: State the abstract, operations, error handling, and compatibility guarantees.
- **In Scope**: All five predicates; their call sites, separated into production code, doctests and tests; what a surface with no production consumer means.
- **Out of Scope**: The two error predicates' membership sets (→ [`algorithm/002`](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md)); the six exported types (→ [`api/001`](001_the_vocabulary_surface.md)).

### Abstract

**Five predicates over three types, answering questions the family does not
currently ask.** Each is a `const fn` taking `self` by value and returning
`bool`; each names a distinction its type's variants do not make on their own.

They are grouped into one instance because they share a property no single one
of them reveals: **the family's production code calls exactly one of the five,
once**, and that call is inside `ring_config::is_tick_safe`, whose own callers
are all tests. Every other reference to any of the five is in a doctest or a
test.

This is not a defect. It is what a vocabulary crate's classifier layer looks
like when it is written for consumers behind an export Contract who have not
arrived yet. It is worth measuring precisely, because "unused" and "written for
an absent consumer" look identical and have opposite remedies.

### Operations

| # | Predicate | Type | True for | Bodies |
|---|-----------|------|----------|--------|
| P1 | [`is_configuration`](../item/associated_function/004_ring_error_is_configuration.md) | `RingError` | `CapacityZero`, `CapacityNotPowerOfTwo`, `BatchTooLarge`, `PolicyUnsupported` | `match`, 2 arms (4-variant true, 5-variant false) |
| P2 | [`is_transient`](../item/associated_function/005_ring_error_is_transient.md) | `RingError` | `Full`, `Empty` | `match`, 2 arms (2-variant true, 7-variant false) |
| P3 | [`is_non_blocking`](../item/associated_function/011_wait_kind_is_non_blocking.md) | `WaitKind` | `None` | `match`, 2 arms (1-variant true, 3-variant false) |
| P4 | [`reports_failure`](../item/associated_function/012_overflow_policy_reports_failure.md) | `OverflowPolicy` | `Fail` | `match`, 2 arms (1-variant true, 2-variant false) |
| P5 | [`drops_silently`](../item/associated_function/013_overflow_policy_drops_silently.md) | `OverflowPolicy` | `DropNewest`, `DropOldest` | `match`, 2 arms (2-variant true, 1-variant false) |

All five were a `matches!` over the true-side variants alone until each one's own
`Fix(..._classification_not_exhaustive)` converted it to the exhaustive shape
above — the true-side variant counts are unchanged, but a `matches!`'s implicit
`false` arm is now a written-out false arm with nowhere for a new variant to
fall through to.

**All five are `const`, `#[ must_use ]`, and take `self` by value.** The last is
available because all three types are `Copy`, and it matters: a value can be
classified after being moved into a log line or a `Result`.

**The measurement.** Separating production call sites from doctests requires
looking at what the grep hits actually are — every "source" hit for P1 and P2 is
inside a `///` block:

```sh
cd "$(git rev-parse --show-toplevel)"
for m in is_configuration is_transient is_non_blocking reports_failure drops_silently; do
  echo "== $m =="
  command grep -r "\.$m(" ring_*/src | command grep -v '^ring_types/'
done
```

Live output:

```
== is_configuration ==
ring_gating/src/lib.rs:    /// assert!(too_wide.is_configuration(), "never retry this one");
ring_gating/src/lib.rs:    /// assert!(!RingError::Full.is_configuration(), "but do retry this one");
== is_transient ==
ring_shutdown/src/lib.rs:    /// assert!(!RingError::Closed.is_transient());
== is_non_blocking ==
ring_config/src/lib.rs:        self.wait.is_non_blocking()
== reports_failure ==
== drops_silently ==
```

| Predicate | Production calls | Doctest calls | Test calls |
|-----------|-----------------:|--------------:|-----------:|
| P1 `is_configuration` | **0** | 2 | 11 |
| P2 `is_transient` | **0** | 1 | 6 |
| P3 `is_non_blocking` | **1** | 0 | 3 |
| P4 `reports_failure` | **0** | 0 | 1 |
| P5 `drops_silently` | **0** | 0 | 1 |

**One production call site across the family, and it does not escape its own
crate either.** That call sits inside `ring_config/src/lib.rs`'s `is_tick_safe`
body, and that method's own callers are `ring_config`'s tests and its own
doctest:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'is_tick_safe' . --include=*.rs
```

Live output:

```
ring_config/tests/config_test.rs:      cfg.with_wait( kind ).is_tick_safe(),
ring_config/tests/config_test.rs:  assert!( cfg.with_wait( WaitKind::None ).is_tick_safe() );
ring_config/tests/config_test.rs:  assert!( !cfg.with_wait( WaitKind::Spin ).is_tick_safe() );
ring_config/src/lib.rs:  /// assert!( !cfg.is_tick_safe() );
ring_config/src/lib.rs:  /// assert!( cfg.with_wait( WaitKind::None ).is_tick_safe() );
ring_config/src/lib.rs:  pub const fn is_tick_safe( &self ) -> bool
```

**So the family's entire classifier layer is reached, in running code, by
nothing.** The chain terminates one level up: `is_non_blocking` → `is_tick_safe`
→ tests.

**Why this is worth stating rather than fixing.** Two readings fit the data and
they call for opposite responses:

| Reading | Implies | Evidence for |
|---------|---------|--------------|
| Dead code | Delete P1–P5 | No caller, and 17 of 25 references are tests asserting the predicates against themselves |
| Written for the Contract consumer | Keep, and document the intended use | `ring_types` is on the five-crate export Contract; P1/P2 are exactly what an external caller needs to decide retry-versus-fix; the family's own crates know which error they just produced |

**The second reading is right, and the first is what a coverage tool reports.**
An internal crate almost never needs `is_configuration` — it constructed the
error and knows its class. The predicate exists for the caller on the other side
of the Contract, who receives a `RingError` with no context. Deleting it would
be a locally-justified change that removes the crate's answer to a question only
external consumers ask.

**P4 and P5 are the weakest of the five**, with zero production and zero doctest
references and one test each. `reports_failure` is `policy == Fail` and
`drops_silently` is `!reports_failure` — both derivable from the variant in one
comparison, unlike P1 and P2, whose membership sets are genuinely non-obvious.
**A predicate whose body a caller could inline correctly on sight earns its
place only by naming something**, and "drops silently" does name something the
enum's variant names do not.

**P5 has a second problem worth recording.** It returns `true` for `DropOldest`
— a policy `ring_core::Ring::new` refuses at construction
(`ring_core/src/lib.rs:165-168`). So P5 describes the behaviour of a
configuration that cannot currently be built through the family's own factory
(→ [`lifecycle/004`](../lifecycle/004_an_overflow_policy_from_declaration_to_refusal.md)).

### Error Handling

**None of the five can fail.** Each is a total function from a `Copy` value to a
`bool`, with no panic path, no allocation, and no `Result`. That is the point of
them: a caller holding an error should not risk a second failure while
classifying the first.

**The gap is not in error handling but in coverage of the domain.** P1 and P2
together leave three of `RingError`'s nine variants answering `false` to both —
`Closed`, `NameTaken`, `NameUnknown` — and `false` is a valid answer, so nothing
signals the omission (→ [`algorithm/002`](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md)).
A caller writing `if is_configuration() { fix } else { retry }` retries a closed
ring forever.

### Compatibility Guarantees

| # | Guarantee | Basis | State |
|---|-----------|-------|-------|
| Q1 | All five stay `const` and `Copy`-taking | Bodies are exhaustive `match` over `Copy` types | ✅ Held |
| Q2 | Membership sets change only with a variant's meaning | Convention | ⚠️ Unenforced — a set could be edited silently |
| Q3 | P1 and P2 remain non-complementary | Three variants are in neither | ✅ Held, and undocumented on the type |
| Q4 | A new `RingError` variant is classified deliberately | Both predicates are wildcard-free exhaustive `match`es inside the defining crate, where `#[ non_exhaustive ]` does not apply | ✅ Held — a new variant fails to compile at both predicates until classified |
| Q5 | P4/P5 stay consistent with what backends accept | **Nothing** | ❌ Already drifted — P5 describes `DropOldest`, which `ring_core` refuses |

**Q4 is now compiler-enforced.** Before `Fix(ring_error_classification_not_exhaustive)`,
adding a variant to `RingError` required no edit to either predicate, compiled
cleanly, and silently enlarged the unclassified bucket. Both predicates are now
wildcard-free exhaustive `match`es living inside the defining crate — the one
place `#[ non_exhaustive ]` does not restrict — so a new variant fails to compile
at `is_configuration` and `is_transient` until each is given an explicit arm.
What the compiler forces is that *some* answer is given, not that it is the
*correct* one, and nothing yet asserts the unclassified bucket stays at exactly
three (the gap § Error Handling describes below is unchanged by this fix).

**Q5 is already broken and its breakage is invisible.** P5's contract is about
what a policy does when the ring is full; `ring_core` decides one of the three
policies never gets that far. Neither crate is wrong, and nothing connects them.

### APIs

| File | Relationship |
|------|--------------|
| [001_the_vocabulary_surface.md](001_the_vocabulary_surface.md) | The full surface — these five among fourteen operations |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_classifying_an_error_into_configuration_or_traffic.md](../algorithm/002_classifying_an_error_into_configuration_or_traffic.md) | P1 and P2's membership sets, and the three-variant gap Q4 would guard |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) | The `Copy` property Q1 rests on |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md) | The Contract that makes the "absent external consumer" reading the right one |

### Items

| File | Relationship |
|------|--------------|
| [../item/associated_function/004_ring_error_is_configuration.md](../item/associated_function/004_ring_error_is_configuration.md) | P1, with its caller tree |
| [../item/associated_function/005_ring_error_is_transient.md](../item/associated_function/005_ring_error_is_transient.md) | P2 |
| [../item/associated_function/011_wait_kind_is_non_blocking.md](../item/associated_function/011_wait_kind_is_non_blocking.md) | P3 — the one with a production caller |
| [../item/associated_function/012_overflow_policy_reports_failure.md](../item/associated_function/012_overflow_policy_reports_failure.md) | P4 |
| [../item/associated_function/013_overflow_policy_drops_silently.md](../item/associated_function/013_overflow_policy_drops_silently.md) | P5 |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_an_overflow_policy_from_declaration_to_refusal.md](../lifecycle/004_an_overflow_policy_from_declaration_to_refusal.md) | Q5 — the policy P5 describes and `ring_core` refuses |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_ring_error.md](../type/002_ring_error.md) | The enum P1 and P2 partition |

### Sources

| File | Relationship |
|------|--------------|
| [`src/error.rs`](../../src/error.rs) | Lines 111–125 and 146–159 — P1 and P2 |
| [`src/policy.rs`](../../src/policy.rs) | Lines 81–88, 153–160, 178–185 — P3, P4, P5 |
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | Line 240 — the family's only production call of any of the five |
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | Lines 154–157 — the refusal that makes Q5 stale |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ All five are asserted exhaustively over their own type's variants — `errors_split_configuration_from_traffic`, `only_full_and_empty_are_transient`, `exactly_one_wait_kind_is_non_blocking`, `overflow_policies_partition_by_reporting`. ⚠️ **Every one of those tests asserts a predicate against the variant set it was written from**, so all five pass by construction and none would catch Q4's silent default. The test that would is the one counting the unclassified bucket, and it does not exist |

### TY24 — All Five Predicates Are `const fn` and `#[ must_use ]`, and Four Are Never Called

`#[ must_use ]` prevents a caller discarding a result; `const fn` lets one
evaluate it at compile time. Four of the five predicates have no caller to do
either.

That is not an argument for removing the attributes — they cost nothing and they
are correct. It is a note on what the attribute census in
[`001`](001_the_vocabulary_surface.md) measures: *thirteen `#[ must_use ]`
methods* describes the source, not the family, and the two figures differ by
more than the attributes suggest.

### TY25 — `is_non_blocking` Is the Only Predicate With a Caller, and It Has One

One call, in the crate that turns a `WaitKind` into a validated configuration.
It is also the crate where the predicate matters most — a `WaitKind` chosen for
a tick-path ring has to be `None`, and `ring_config` is where that is checked
rather than assumed.

So the finding is not that the predicate is unused. It is that the *export* is
justified by one consumer, and the other four predicates have not found theirs.
