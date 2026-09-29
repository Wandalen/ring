# Pattern: Discriminants Here, Handlers Elsewhere

### Scope

- **Purpose**: Record the split this crate rules — this crate owns `WaitKind` and `OverflowPolicy` as names, and other crates own every behaviour that dispatches on them — and measure how completely the family actually observes it.
- **Responsibility**: State the problem, solution, applicability, and consequences.
- **In Scope**: The two policy enums; the crate's one behaviourally-dispatching `match` (the renderer) among its six, and why it is not a violation; the six dispatch sites elsewhere; the third dispatcher the ruling does not name.
- **Out of Scope**: What the strategies do (owned by `ring_wait` and `ring_overflow`); the closure requirement over the same enums (→ [`non_functional_requirement/002`](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md)).

### Problem

**Two features each name an enum and its handlers in one sentence, and two
crates each claim to own that feature.**

Wait-kind strategies are one such area; overflow-policy handling is another.
Read literally, each maps to a single owner — but the family has
`ring_types` declaring both enums and `ring_wait`/`ring_overflow` implementing
what they select. Left unruled, the overlap resolves itself the wrong way in
whichever direction is locally convenient:

| If the enum's crate also dispatches | If the handler's crate also declares |
|-------------------------------------|--------------------------------------|
| `ring_types` gains a `spin()` and a `park()`, so tier 0 grows a `std` dependency and stops compiling in isolation | Two crates declare a `WaitKind`, and `ring_config` must pick one — the family's vocabulary forks |

The first failure is the one this crate is structurally exposed to.
`WaitKind::Park` obviously *wants* a `thread::park` beside it. Writing that
method here is a one-line edit that costs the family its acyclic tier 0
(→ [`invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md)) and is
not visibly wrong at the call site.

### Solution

**Split every such feature on the line between naming a case and acting on it.**
This crate's own design states it directly:

> **Ruled:** `ring_types` owns the discriminants (`WaitKind`, `OverflowPolicy`);
> `ring_wait` and `ring_overflow` own the strategy implementations that dispatch
> on them. Each feature maps to two crates, split on the same line.

The ruling cites the crate's own manifest description as already deciding it —
*"Shared ids, errors, and policy enums for the ring family — **no ring
logic**"*. The rule was written down before the conflict arose; the ruling
above only formalizes it.

**The line is drawn at behaviour, not at syntax — and syntax no longer tracks
it.** Every classification predicate in this crate started as a `matches!`
and has since been converted to an exhaustive `match`, one at a time, so that
a variant the predicate does not name fails to compile here instead of
silently reading `false`
(`Fix(ring_error_classification_not_exhaustive)`,
`Fix(wait_kind_is_non_blocking_classification_not_exhaustive)`,
`Fix(overflow_policy_classification_not_exhaustive)`). The crate now contains
six `match self` expressions and zero `matches!`:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r '\bmatch\b' ring_types/src/ | command grep -v ': *//'
```

Live output:

```
ring_types/src/error.rs:    match self
ring_types/src/error.rs:    match self
ring_types/src/error.rs:    match self
ring_types/src/policy.rs:    match self
ring_types/src/policy.rs:    match self
ring_types/src/policy.rs:    match self
```

*(The second `grep` excludes comment lines — each of the three
`Fix(..._classification_not_exhaustive)` comments quotes the pre-fix
`matches!` form in prose, which a bare keyword search can't tell apart from
the keyword itself. Same filter as
[`item/enum/001`](../item/enum/001_wait_kind.md)'s identical check, kept
consistent rather than reinvented here.)*

Five of the six are classifiers — `is_configuration`, `is_transient`
(`error.rs`) and `is_non_blocking`, `reports_failure`, `drops_silently`
(`policy.rs`) — each still a total function from `self` to `bool`, unchanged
in what it decides despite the keyword change. Only `Display::fmt`
(`error.rs`) selects among actions, one `write!` per variant:

| Site | Function | Returns |
|------|----------|---------|
| `error.rs` | `is_configuration`, `is_transient` | `bool` |
| `error.rs` | `Display::fmt` | A message string, via `write!` |
| `policy.rs` | `is_non_blocking`, `reports_failure`, `drops_silently` | `bool` |

**The distinction is behavioural, not syntactic — and now provably so.** A
classifier answers a question *about* a value and returns a `bool` the caller
acts on; `Display::fmt` selects among actions and moves the decision into
this crate. That distinction used to be visible in the keyword too —
`matches!` for the five classifiers, `match` for the renderer — but each
classifier's conversion to an exhaustive `match`, done for
exhaustiveness-safety rather than for this pattern, erased the syntactic tell
without touching the behavioural one. A `grep` for `match` no longer
separates the two; only the return type does. Rendering remains the one
behavioural exception the rule tolerates, because a message is a property of
the name and nothing downstream can supply it without re-deriving the whole
enum.

**Where the dispatch actually lives:**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn -A3 'match \(kind\|policy\)' ring_*/src \
  | command grep -E '(WaitKind|OverflowPolicy)::' | sed 's/\.rs[-:].*/.rs/' | sort -u
```

Live output:

```
ring_overflow/src/lib.rs
ring_stats/src/lib.rs
ring_wait/src/lib.rs
```

*(`sed` rather than `cut -d:` — `grep -A` separates context lines with `-`, not
`:`, so cutting on a colon leaves the line number and text attached.)*

| Crate | Function | Maps | Sites |
|-------|----------|------|------:|
| `ring_wait` | `escalation_hint` | `WaitKind → Option< WaitKind >` | 1 |
| `ring_wait` | `pause` | `WaitKind →` a real spin/yield/park | 1 |
| `ring_overflow` | `resolve` | `OverflowPolicy →` `Resolution` or `Err( Full )` | 1 |
| `ring_overflow` | `would_resolve` | `OverflowPolicy → Resolution`, pure | 1 |
| `ring_stats` | `record_drop` | `OverflowPolicy →` which counter | 1 |
| `ring_stats` | `dropped` | `OverflowPolicy →` which counter | 1 |

**`ring_wait::pause` is where the whole point lands.** Its doc comment says so
outright: *"The whole behavioural difference between the four variants is in
this one"* function. Four variants declared in a crate with no dependencies;
one function elsewhere that turns them into `std::thread` calls.

### Applicability

**Apply this split when a type's variants name *choices* rather than *data*,
and at least one choice's implementation would pull in a dependency the
declaring crate must not have.**

| Signal | Present here |
|--------|--------------|
| The enum is a policy or strategy selector, not a payload | ✅ Both |
| Some variant's handler needs a capability the declaring crate lacks | ✅ `Park` needs `std::thread` |
| More than one crate must name the enum to talk about it | ✅ 10 crates name `OverflowPolicy`, 7 name `WaitKind` |
| The handlers differ enough to warrant separate crates | ✅ Waiting and overflow share nothing |

**It does not apply to `RingError`, and the contrast is instructive.**
`RingError`'s variants are data, not choices — nothing *dispatches* on an error
to select a behaviour; callers classify it with a predicate and decide for
themselves (→ [`api/002`](../api/002_the_five_classifier_predicates.md)). So
`RingError` keeps its rendering here and needs no handler crate, and the
family's error surface is one type rather than a type plus a handler.

**It does not apply to `Capacity` or `Seq` either.** Those are values with
operations, and their operations are arithmetic — no capability is needed beyond
what tier 0 already has, so splitting them would buy nothing and cost a crate
boundary.

### Consequences

**Benefit — the handlers enforce the closure the declaring crate cannot.** All
six dispatch sites are wildcard-free. Adding a fourth `OverflowPolicy` variant
fails to compile in four places across two crates, and adding a fifth
`WaitKind` fails in two more. The declaring crate cannot achieve that on its
own: an `ALL` array's length constrains nothing about the enum
(→ [`non_functional_requirement/002`](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md)),
and the crate has no dispatch of its own to break. **Moving the handlers out is
what makes the enums closed in practice.**

**Benefit — tier 0 stays dependency-free.** `WaitKind::Park` is a name here and
a `thread::park` in `ring_wait`. That is the single largest reason this crate
compiles against nothing.

**Cost — a third dispatcher exists that the ruling does not name.**
`ring_stats` matches on `OverflowPolicy` twice, mapping a policy to one of three
atomic counters. It is not a "strategy implementation" in the ruling's sense, and it
is not a discriminant either; it is a third category the ruling did not
anticipate. Nothing is wrong with the code — a per-policy counter has to select
a counter somehow — but the ruling's two-crate framing under-describes the
family by one crate, and a future reader checking compliance by grepping for
`match` will find a site the ruling does not account for.

**Cost — the split is unenforced.** Nothing prevents a `pause()` from being
added to `WaitKind` tomorrow. The manifest description says "no ring logic", the
decision ruled it, and no gate checks it:

| Guard | Exists |
|-------|:------:|
| Manifest description states the rule | ✅ |
| This pattern instance rules it | ✅ |
| A gate that fails on a `std` dependency in `ring_types` | ❌ |
| A gate that fails on a behaviour-selecting `match` here | ❌ |

The dependency half is the checkable one — `ring_types`'s `[dependencies]`
being empty is a one-line assertion, and it would catch the `Park` handler on
the day it arrived, since the handler is what forces the dependency
(→ [`invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md), whose V4
is the same gap seen from the dependency side).

**Cost — a caller must import two crates to use one concept.** Configuring a
wait strategy means naming `ring_types::WaitKind` and calling
`ring_wait::pause`. For the family that is free — both are path dependencies.
For an external consumer it is not: the export Contract lists `ring_types` but
not `ring_wait`, so a Contract-bound consumer can *name* a `WaitKind` and cannot
*execute* one. That is the intended shape — strategy execution happens inside
the ring, and the consumer only declares a preference through `RingConfig` — but
it is the same asymmetry that leaves `RegistryError` unreachable
(→ [`integration/002`](../integration/002_the_registry_that_declined_the_shared_error.md)),
arrived at from the opposite direction and, here, deliberately.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | G7 — the guarantee this pattern states |
| [../api/002_the_five_classifier_predicates.md](../api/002_the_five_classifier_predicates.md) | The classifier side of the line, and why a `bool` is not dispatch |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md) | Which crates name each enum, and the Contract the last cost turns on |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_tier_zero_depends_on_nothing.md](../invariant/002_tier_zero_depends_on_nothing.md) | The property this pattern protects, and the missing gate seen from the dependency side |

### Items

| File | Relationship |
|------|--------------|
| [../item/enum/001_wait_kind.md](../item/enum/001_wait_kind.md) | The four names, and the crate that acts on them |
| [../item/enum/003_overflow_policy.md](../item/enum/003_overflow_policy.md) | The three names, and the two crates that act on them |
| [../item/implementation/003_impl_display_for_ring_error.md](../item/implementation/003_impl_display_for_ring_error.md) | The one `match` among six that dispatches to action rather than classifying |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md) | The closure the six wildcard-free dispatch sites enforce for free |

### Patterns

| File | Relationship |
|------|--------------|
| [002_a_newtype_that_makes_a_check_unnecessary.md](002_a_newtype_that_makes_a_check_unnecessary.md) | The crate's other structural pattern — moving a check inward rather than a behaviour outward |

### State Machines

| File | Relationship |
|------|--------------|
| [004_an_overflow_policy_from_declaration_to_refusal.md](../lifecycle/004_an_overflow_policy_from_declaration_to_refusal.md) | The full arc of one discriminant from its declaration here to its handler |

### Sources

| File | Relationship |
|------|--------------|
| [`src/policy.rs`](../../src/policy.rs) | Both enums, and the three classification `match`es that are not dispatch |
| [`ring_wait/src/lib.rs`](../../../ring_wait/src/lib.rs) | Lines 83–92 and 112 — `WaitKind`'s handlers |
| [`ring_overflow/src/lib.rs`](../../../ring_overflow/src/lib.rs) | Lines 106–115 and 131–139 — `OverflowPolicy`'s handlers, impure and pure |
| [`ring_stats/src/lib.rs`](../../../ring_stats/src/lib.rs) | Lines 287–292 and 343–348 — the third dispatcher the ruling does not name |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ Asserts what the enums *are* — variant counts, defaults, partitions — which is exactly the scope the pattern leaves here. ❌ **Nothing asserts the pattern itself**, and nothing here could: a test compiled against this crate cannot observe the absence of a behaviour, only the presence of one. The checkable half is the empty `[dependencies]`, which belongs to a gate rather than a test |
| [`ring_wait/tests/wait_test.rs`](../../../ring_wait/tests/wait_test.rs) | Where `WaitKind`'s behaviour is asserted — the split's test-side consequence, four variants exercised in the crate that implements them |

### TY47 — The Split Holds in Both Directions

The pattern's risk is that two enums declared fifty lines apart get handled
together somewhere downstream. They are not:

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'OverflowPolicy in ring_wait:  '; grep -c 'OverflowPolicy' ring_wait/src/lib.rs
printf 'WaitKind in ring_overflow:    '; grep -c 'WaitKind' ring_overflow/src/lib.rs
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
OverflowPolicy in ring_wait:  0
WaitKind in ring_overflow:    0
```

Zero both ways. The two policies share a declaration file and share nothing
else — which is the pattern working, and is worth asserting because the
co-location in `policy.rs` is the one thing that could have undone it.
