# Decision: The Model Lives In Tests

### Scope

- **Purpose**: Promote [`readme.md`](readme.md) Closed 2 to an instance, because the coverage argument that settled it has since been measured and the measurement does not cover the thing the decision moved.
- **Responsibility**: The decision, the coverage argument, what M5 actually measured, and the cost the decision transfers to the doc examples.
- **In Scope**: `tests/exhaustive_test.rs`'s location and its `#![ cfg( loom ) ]` gate; the coverage and clippy stages that check the crate.
- **Out of Scope**: What the model asserts (→ [`integration/002`](../integration/002_the_edge_that_only_exists_under_a_cfg.md)); the leak helpers it needs (→ [`workaround/001`](../workaround/001_two_allocations_loom_cannot_avoid.md)).

### The decision

The loom model lives in `tests/exhaustive_test.rs`, not in `src/`.

**Status: closed.** The register entry is [`readme.md`](readme.md) Closed 2. This
document does not reopen it — the decision is right, and what follows is what
measuring it revealed about the shape of the crate's checks.

### The argument as recorded

Under `--cfg loom`, `ring_atomic` swaps in loom's instrumented atomics, so a
model can only exist behind that cfg. An ordinary coverage run does not set the
cfg, and `cfg`-removed lines in `src/` are counted as **uncovered** by
`cargo tarpaulin`. A model in `src/` would therefore put a permanent hole in the
crate's coverage that no test could close.

Keeping `src/` free of `#[ cfg ]` attributes is what lets the scripted fixture
report a clean number on an ordinary run. That reasoning is sound and the
consequence held: M5 measured `79/83` first and `83/83` after two real gaps were
closed.

### What the measurement covers

M5's command is:

```text
cargo tarpaulin -p ring_testkit --all-features --out Stdout 2>&1 \
  | grep 'ring_testkit/src'
```

The filter is `src`. The 83 lines are `src/lib.rs`'s. `tests/exhaustive_test.rs`
is 163 lines behind `#![ cfg( loom ) ]`, so on that run it compiles to nothing
and would not appear in the output even without the filter.

| What | Lines | Covered by a number |
|---|---|---|
| `src/lib.rs` | 83 | yes — M5, `83/83` |
| `tests/testkit_test.rs` | 519 | not measured; it is the measurer |
| `tests/exhaustive_test.rs` | 163 | no |

The decision moved the model out of the measured region *on purpose*, and that
is exactly what makes the number clean. What is worth writing down is that no
second measurement was added on the other side of the cfg — M6 runs clippy under
`--cfg loom` and nothing runs coverage there.

### The cost the decision transfers

A `tests/*.rs` file can gate itself: twenty-four test files across the family
open with `#![ cfg( not( loom ) ) ]`, and this crate's own
`tests/testkit_test.rs` is one of them. A doc example in `src/` cannot — it is
prose in a comment, compiled by `rustdoc`, and there is no attribute on it to
gate.

Three of this crate's four doc examples construct a `ring_core::Ring`. Under
`--cfg loom`, `tests/exhaustive_test.rs`'s own header states that loom's atomics
*"panic the moment they are touched outside a `loom::model` closure"* — which is
what a doc example does.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'cfg attributes in src/:          %s\n' "$( command grep -cE '^ *#!?\[ cfg' src/lib.rs || true )"
printf 'doc examples in src/:            %s\n' "$( awk '/^(\/\/!|\/\/\/) ```/{ if( inb ){ if( bare ) n++; inb = 0 } else { inb = 1; bare = ( $0 ~ /```$/ ) } ; next } END{ print n+0 }' src/lib.rs )"
printf 'of those, constructing a Ring:   %s\n' "$( awk '/^(\/\/!|\/\/\/) ```/{ if( inb ){ if( bare && hit ) n++; inb = 0 } else { inb = 1; hit = 0; bare = ( $0 ~ /```$/ ) } ; next } inb && /Ring::new/{ hit = 1 } END{ print n+0 }' src/lib.rs )"
printf 'lines in the loom model:         %s\n' "$( wc -l < tests/exhaustive_test.rs )"
printf 'the model gate:                  %s\n' "$( command grep -m1 '^#!\[ cfg' tests/exhaustive_test.rs )"
printf 'the scripted suite gate:         %s\n' "$( command grep -m1 '^#!\[ cfg' tests/testkit_test.rs )"
printf 'family test files gated not(loom): %s\n' "$( command grep -rl 'cfg( not( loom ) )' ../ring_*/tests/*.rs | wc -l )"
printf 'manual stages running under loom: %s\n' "$( command grep -c 'cfg loom' tests/manual/readme.md || true )"
printf 'crate-doc lines on the doctest gap: %s\n' "$( awk '/^\/\/! The doc examples in this crate/,/^\/\/! clippy does not run doctests\.$/' src/lib.rs | wc -l )"
```

Live output:

```
cfg attributes in src/:          0
doc examples in src/:            5
of those, constructing a Ring:   3
lines in the loom model:         163
the model gate:                  #![ cfg( loom ) ]
the scripted suite gate:         #![ cfg( not( loom ) ) ]
family test files gated not(loom): 28
manual stages running under loom: 3
crate-doc lines on the doctest gap: 7
```

### Decisions

| File | Relationship |
|------|--------------|
| [readme.md](readme.md) | Closed 2, the register entry this promotes |
| [001_the_record_type_is_a_u32.md](001_the_record_type_is_a_u32.md) | Pending 1, the other entry that grew a measurement |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_edge_that_only_exists_under_a_cfg.md](../integration/002_the_edge_that_only_exists_under_a_cfg.md) | The `loom` edge this decision positions |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_two_runs_compare_equal.md](../non_functional_requirement/001_two_runs_compare_equal.md) | The scripted half's determinism claim, which is measured |

### Sources

| File | Relationship |
|------|--------------|
| [`tests/exhaustive_test.rs`](../../tests/exhaustive_test.rs) | The model and its gate |
| [`tests/manual/readme.md`](../../tests/manual/readme.md) | M5 and M6, the two stages that check across the cfg |

### Tests

| File | Relationship |
|------|--------------|
| `tests/exhaustive_test.rs` | The file this decision places |

### TK15 — the half that was moved is the half with no coverage number

M5 reports `83/83` and the crate's readme, its decision register and its Run
Record all carry that figure. It is `src/lib.rs`'s coverage: the tarpaulin
invocation filters its output to `ring_testkit/src`, and the file the
decision is *about* is 163 lines behind `#![ cfg( loom ) ]`, compiled to nothing
on any run that does not set the cfg.

That is not a flaw in the decision — moving the model out of the measured region
is the point, and it is why the number can be clean at all. It is a gap in the
crate's checks: nothing measures the other side. M6 is the one stage that sets
`--cfg loom`, and it runs `cargo clippy`, which reports lints rather than
execution. So the crate has a coverage figure for the half whose coverage was
never in doubt and none for the half the decision exists to protect.

M2 partly compensates — a negative control proving the model explores rather than
trivially passing — but a control is not a measurement of how much of the model
runs.

### TK16 — the doc examples cannot take the gate the test files take

Twenty-four test files across the family open with `#![ cfg( not( loom ) ) ]`,
including this crate's own scripted suite, because loom's atomics panic when
touched outside a `loom::model`. That gate is available to a `tests/*.rs` file
and is not available to a doc example: rustdoc compiles the example as its own
crate from a comment, and there is no inner attribute the author can place on it.

Three of this crate's five doc examples call `Ring::new`. Under `--cfg loom`
those construct loom atomics outside a model, which is the exact condition
`tests/exhaustive_test.rs`'s header describes as panicking. The other two —
`audit_received`'s and `audit_received_unordered`'s — take slices and are the
only ones that would survive.

Nothing ran into it. M6 sets the cfg and runs `cargo clippy`, which does not
execute doc tests, and no routine check in the crate runs `cargo test --doc`
under the cfg. The exposure is a family-wide loom run of the form the recurring
comment in those nineteen files anticipates — the comment warns that an ungated
test file would *"die here instead of reaching the models"*, and the doc examples
are the case that comment does not cover because they cannot be gated the same
way.

**Disposition:** applied — as a crate-doc section, because the only real fixes
are worse than the hazard. The three examples could drop `Ring::new` and lose the
thing they demonstrate; they could be marked `no_run`, which stops them being
checked at all and so trades a hypothetical loom failure for a certain loss of
coverage; or the crate could grow its own `cfg`, which is the fifth copy of a
switch `ring_atomic` explicitly says no other crate should need. So the crate doc
now carries a `# Under --cfg loom` section stating the whole chain: that the seam
arrives four crates down through `ring_core` → `ring_mpsc`/`ring_spsc` →
`ring_atomic` with no `cfg` written here, that `Script::run` therefore panics
under the cfg outside a model, that three of five doc examples are the visible
edge, that a doc example cannot carry `#![ cfg( not( loom ) ) ]` the way the 24
gated test files do, and that the one stage setting the cfg runs clippy, which
does not run doctests. What this does not buy: nothing executes under the cfg, so
the claim that these three would panic is still an argument from loom's
documented behaviour and not a measurement — the first real family-wide loom run
is what will settle it. Now prints:
`crate-doc lines on the doctest gap: 7`
