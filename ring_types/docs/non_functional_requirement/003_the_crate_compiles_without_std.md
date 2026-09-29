# Non-Functional Requirement: The Crate Compiles Without std

### Scope

- **Purpose**: Record that `ring_types` is `no_std`-compatible in fact and now says so, and that the declaration itself — not a separate gate — is what would catch the day it stops being true.
- **Responsibility**: State the quality attribute, the requirement, the measurement method, and the acceptance threshold.
- **In Scope**: The crate's `core`-only import surface; the probe that proves compilation under `#![ no_std ]`; the two realistic ways the property is lost.
- **Out of Scope**: Whether the types allocate, which is a related but distinct property held by a different mechanism (→ [`001_errors_and_positions_do_not_allocate.md`](001_errors_and_positions_do_not_allocate.md)); the empty dependency list, which is an absolute invariant rather than a graded requirement (→ [`../invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md)).

### Quality Attribute

**Portability.** Specifically: whether this crate constrains where the family can
run, or declines to.

Tier 0 sits under all thirty-two other `ring_*` crates
(→ [`../integration/001`](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md)),
so its platform requirements are the family's floor. A `ring_types` that needs
`std` makes every crate above it need `std`, whatever those crates themselves
import. That is a decision this crate makes on everyone's behalf, and it now
states that decision outright — for itself and the two dependents
(`ring_stats`, `ring_overflow`) that declared `#![ no_std ]` alongside it, not
yet for the other thirty crates in the family.

### Statement

**`ring_types` must compile as a `#![ no_std ]` crate, using only `core`.**

It does, and now declares it (`src/lib.rs:27`). The requirement used to split
into two halves of very different strength — substance held, declaration was
absent — and both halves hold now:

- **The substance holds and is verifiable today.** No `std::` path appears
  anywhere in `src/`; the only import from outside the crate is `use core::fmt;`
  (→ [`../item/use_declaration/006_use_core_fmt.md`](../item/use_declaration/006_use_core_fmt.md)),
  and the one foreign trait implemented is `core::error::Error`, stable in `core`
  since Rust 1.81 (→ [`../item/implementation/004_impl_error_for_ring_error.md`](../item/implementation/004_impl_error_for_ring_error.md)).
- **The declaration is present.** `src/lib.rs:27` reads `#![ no_std ]`, so the
  compiler no longer links `std` implicitly and every `std`-only facility is out
  of scope. The next edit that reaches for one fails to compile instead of
  silently succeeding.

**The requirement stays written as a requirement rather than an invariant, but
for a narrower reason than before.** It is no longer unguarded — the attribute
turns a regression into a compile error, the strongest mechanism this corpus
has. What keeps it a requirement rather than an invariant is scope, not
strength: the property is enforced for `ring_types` and the two dependents that
declared it alongside it, not for the family the Quality Attribute above is
about. An invariant here would be family-wide; this is a three-crate island's
threshold, met and now enforced by the compiler for exactly those three crates.

### Measurement Method

**Two checks, one cheap and one conclusive.**

**(1) The import census — cheap, and now redundant with the compiler's own enforcement.**

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types
command grep -rn '\bstd::' src/ | command grep -vE ':[0-9]+: *(//|///|//!)' | command sed -E 's/^([^:]+):[0-9]+:/\1:/'
command grep -rh '^use \|^pub use ' src/
```

Live output:

```
use core::fmt;
use crate::RingError;
pub use capacity::Capacity;
pub use error::RingError;
pub use id::{ Seq, SlotIndex };
pub use policy::{ OverflowPolicy, WaitKind };
```

The first exits 1 once doc-comment mentions of `std::` are filtered out. The second returns six lines: four crate-local `pub use`
re-exports in `lib.rs`, one `use crate::RingError;` in `capacity.rs`, and one
`use core::fmt;` in `error.rs` — so exactly one of the six leaves the crate at
all (→ [`../item/use_declaration/`](../item/use_declaration/)).

**(2) The compile probe — conclusive, and the only check that catches an
implicit `std` dependency a grep would miss** (a `String` in a signature, a
`HashMap`, a `Box`, none of which need to be written with a `std::` prefix):

```sh
S=$( mktemp -d )/probe && mkdir -p "$S/src" "$S/out"
cd "$(git rev-parse --show-toplevel)"/ring_types
cp src/*.rs "$S/src/"
rustc --edition 2021 --crate-name probe --crate-type lib --out-dir "$S/out" "$S/src/lib.rs"
echo "exit=$?"; ls "$S/out"
```

Live output:

```
exit=0
libprobe.rlib
```

**Two details of that recipe are load-bearing** and each cost a failed attempt
to discover. `--crate-name` is required because a temp directory's path may not
be a valid crate identifier. `--out-dir` rather than `-o /dev/null`, because
`/dev` is read-only in this environment.

**What the probe deliberately does not cover: the test suite.**
`tests/types_test.rs` calls `to_string()`, which is `alloc`'s `ToString`, not
`core`'s `Display`. The library is `core`-only; its tests are not, and no
reformulation of this requirement makes them so without giving up the assertions
that hold [`../invariant/003`](../invariant/003_every_error_renders_distinctly.md).

### Acceptance Threshold

| # | Threshold | Method | Status |
|---|-----------|--------|--------|
| T1 | Zero `std::` paths in `src/` | grep, check (1) | ✅ 0 |
| T2 | Every external import is `core::` | grep, check (1) | ✅ 1 of 1 (`core::fmt`) |
| T3 | `[dependencies]` empty | read `Cargo.toml` | ✅ — and it is one of only two crates of the 33 for which this is true (`ring_align` is the other) |
| T4 | Compiles under `#![ no_std ]` | probe, check (2) | ✅ exit 0 |
| T5 | The crate *declares* `#![ no_std ]` | read `src/lib.rs` | ✅ line 27 |
| T6 | A gate asserts T1–T4 | read `bench_harness/gate/` | ✅ the declaration is the gate — no named script needed |

**Six of six met now, and the last two closed together.** Declaring
`#![ no_std ]` (`src/lib.rs:27`) collapsed T5 and T6 into the same one-line
fact: the declaration *is* the gate, permanently and for free, which is the
strongest form of enforcement available in this corpus and cost exactly the
one line it was always going to cost.

**Two ways the property could be lost — one still silent, one no longer.**

The first is the house convention. `error_tools` is the workspace's standard
error facility, and adopting it here — which
[`../invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md) already
names as the violation most likely to actually happen — would add the first
entry to a `[dependencies]` table that is empty today. That edit still violates
[`../invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md)'s
empty-dependencies invariant silently, on its own terms, whatever `error_tools`
itself requires — `#![ no_std ]` here does not stop a dependency from existing,
it only stops this crate's own code from naming a `std`-only item without
qualification. Whether that specific dependency would also break T4 is a
separate, untested question this instance does not answer.

The second needs no dependency at all, and is no longer silent. A future
variant of `RingError` carrying a `String` instead of a `usize` used to compile,
pass every test, and break T4 invisibly — nobody writes `std::string::String`
for check (1)'s grep to catch. It no longer does, because check (2)'s probe now
compiles the real `#![ no_std ]` source rather than a std-linked stand-in.
Verified directly: injecting a `BadName( String )` variant into a copy of
`error.rs` and re-running the compile probe fails, as expected — but only
once the injection matches this crate's own brace-on-its-own-line style.
`pub enum RingError` and its `{` are two separate lines here
(`src/error.rs:44–45`), so a `sed` pattern written against
`pub enum RingError {` as a single line matches nothing, copies `error.rs`
through unmodified, and the probe reports a false `exit=0` — no error, no
warning, silently the wrong answer. Caught by re-running the recipe below
verbatim before recording it here:

```sh
S=$( mktemp -d )/probe && mkdir -p "$S/src" "$S/out"
cd "$(git rev-parse --show-toplevel)"/ring_types
cp src/*.rs "$S/src/"
sed -i '/pub enum RingError/{n;a\
  BadName( String ),
}' "$S/src/error.rs"
rustc --edition 2021 --crate-name probe --crate-type lib --out-dir "$S/out" "$S/src/lib.rs" 2>&1 | sed "s|$S|/tmp/probe|g"
echo "exit=${PIPESTATUS[0]}"
```

Live output:

```
error[E0425]: cannot find type `String` in this scope
  --> /tmp/probe/src/error.rs:46:12
   |
46 |   BadName( String ),
   |            ^^^^^^ not found in this scope

error[E0425]: cannot find type `String` in this scope
  --> /tmp/probe/src/error.rs:46:12
   |
46 |   BadName( String ),
   |            ^^^^^^ not found in this scope
   |
help: you might be missing a type parameter
   |
44 | pub enum RingError<String>
   |                   ++++++++

error[E0425]: cannot find type `String` in this scope
  --> /tmp/probe/src/error.rs:46:12
   |
46 |   BadName( String ),
   |            ^^^^^^ not found in this scope
   |
help: you might be missing a type parameter
   |
44 | pub enum RingError<String>
   |                   ++++++++

error[E0004]: non-exhaustive patterns: `RingError::BadName(_)` not covered
   --> /tmp/probe/src/error.rs:114:11
    |
114 |     match self
    |           ^^^^ pattern `RingError::BadName(_)` not covered
    |
note: `RingError` defined here
   --> /tmp/probe/src/error.rs:44:10
    |
 44 | pub enum RingError
    |          ^^^^^^^^^
 45 | {
 46 |   BadName( String ),
    |   ------- not covered
    = note: the matched value is of type `RingError`
help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern or an explicit pattern as shown
    |
124 ~       | Self::NameUnknown => false,
125 ~       RingError::BadName(_) => todo!(),
    |

error[E0004]: non-exhaustive patterns: `RingError::BadName(_)` not covered
   --> /tmp/probe/src/error.rs:149:11
    |
149 |     match self
    |           ^^^^ pattern `RingError::BadName(_)` not covered
    |
note: `RingError` defined here
   --> /tmp/probe/src/error.rs:44:10
    |
 44 | pub enum RingError
    |          ^^^^^^^^^
 45 | {
 46 |   BadName( String ),
    |   ------- not covered
    = note: the matched value is of type `RingError`
help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern or an explicit pattern as shown
    |
158 ~       | Self::PolicyUnsupported => false,
159 ~       RingError::BadName(_) => todo!(),
    |

error[E0004]: non-exhaustive patterns: `&RingError::BadName(_)` not covered
   --> /tmp/probe/src/error.rs:167:11
    |
167 |     match self
    |           ^^^^ pattern `&RingError::BadName(_)` not covered
    |
note: `RingError` defined here
   --> /tmp/probe/src/error.rs:44:10
    |
 44 | pub enum RingError
    |          ^^^^^^^^^
 45 | {
 46 |   BadName( String ),
    |   ------- not covered
    = note: the matched value is of type `&RingError`
help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern or an explicit pattern as shown
    |
180 ~       Self::PolicyUnsupported => write!( f, "this backend cannot honour the configured overflow policy" ),
181 ~       &RingError::BadName(_) => todo!(),
    |

error: aborting due to 6 previous errors

Some errors have detailed explanations: E0004, E0425.
For more information about an error, try `rustc --explain E0004`.
exit=1
```

Six errors, not one — the same `E0425` repeats three times against the same
span, plus three `E0004`s, because `is_configuration`, `is_transient`
(→ [`../item/implementation/002_impl_ring_error.md`](../item/implementation/002_impl_ring_error.md),
both exhaustive `match` since the `matches!`→`match` exhaustiveness fix) and
the crate's own hand-written `Display` match
([`../item/implementation/003_impl_display_for_ring_error.md`](../item/implementation/003_impl_display_for_ring_error.md))
now all three fail to cover the new variant, where before that fix only
`Display`'s match would have. The count is incidental to this check:
`String` is `alloc`, not `core`, and this crate declares neither `alloc` nor
an `extern crate alloc`, so the `E0425` is what T4 was always meant to
guarantee. What used to be a silent hole in check (1) is now exactly that
compile error, in three places instead of one.

**Whether the rest of the family wants `no_std` is still not settled here.**
`ring_types`, `ring_stats` and `ring_overflow` — a dependency-closed island of
three, chosen because each depends on nothing outside that island — now declare
it together. The other thirty crates do not, and nothing downstream asks them
to; this instance records a capability three crates committed to, not a
family-wide decision this crate could ever make alone (→ TY46, below).

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md) | Why tier 0's platform floor is the family's platform floor |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_tier_zero_depends_on_nothing.md](../invariant/002_tier_zero_depends_on_nothing.md) | The empty `[dependencies]` this requirement leans on, and the `error_tools` edit that would end both |
| [../invariant/003_every_error_renders_distinctly.md](../invariant/003_every_error_renders_distinctly.md) | The assertions that keep the test suite on `alloc` |

### Items

| File | Relationship |
|------|--------------|
| [../item/use_declaration/006_use_core_fmt.md](../item/use_declaration/006_use_core_fmt.md) | The crate's single external import |
| [../item/implementation/004_impl_error_for_ring_error.md](../item/implementation/004_impl_error_for_ring_error.md) | `core::error::Error`, the trait that used to require `std` and no longer does |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_errors_and_positions_do_not_allocate.md](001_errors_and_positions_do_not_allocate.md) | The neighbouring property — held by a derive, so violating it fails to compile; the contrast with T5/T6 is the point |
| [002_the_enum_sets_are_closed_and_asserted.md](002_the_enum_sets_are_closed_and_asserted.md) | The other requirement whose weakest link is an absent assertion |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Line 27 — the crate root, where `#![ no_std ]` is declared |
| [`Cargo.toml`](../../Cargo.toml) | The empty `[dependencies]` table T3 measures |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ **No test names this requirement, and none needs to** — building the lib target is a prerequisite of running any test in this crate at all, so every `cargo test -p ring_types` already recompiles `src/lib.rs` under its own `#![ no_std ]` before a single test body runs. A regression that broke T4 would fail the build, not a test — which is a stronger guarantee than a passing assertion would give |

### TY46 — The Requirement Is Now Met by the Compiler, for a Three-Crate Island

The attribute was added. Nothing broke: the crate imports `core::fmt` and
`core::error` and nothing else
(→ [`../invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md), TY37),
exactly as this finding predicted.

The reason recorded here was that this crate does not own the decision about
whether the *family* is `no_std`, and a tier-0 crate declaring itself `no_std`
while its thirty-one dependents do not would be a claim about the family that
this crate cannot make alone. That reasoning was honored rather than
overridden: the attribute was not added to `ring_types` alone. `ring_stats`
(depends only on `ring_types`) and `ring_overflow` (depends on `ring_types` and
`ring_stats`) declared it in the same change, so the claim actually being made
is scoped to a dependency-closed island of three crates, not to the
thirty-three-crate family. The other thirty crates remain undecided, and
nothing here decides for them.
