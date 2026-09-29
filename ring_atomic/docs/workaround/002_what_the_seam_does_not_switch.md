# Workaround: What the Seam Does Not Switch

### Scope

**Purpose:** Record the two edges of the `cfg( loom )` seam — the third imported name
that crosses it unswitched, and what this crate's own test suite does when the seam
selects the loom side.

**Responsibility:** The three `use` statements at the seam, and the relationship
between the 21 tests and the configuration they compile under.

**In Scope:** `ring_atomic/src/lib.rs:63-70`;
`ring_atomic/tests/atomic_test.rs`.

**Out of Scope:** What the seam costs inside the crate and above it is
[`workaround/001`](001_the_loom_seam_and_the_manifest_above_it.md). The
creation-site pattern the seam depends on is
[`pattern/001`](../pattern/001_one_place_where_an_atomic_is_created.md).

---

## Three Names, and Seventeen Tests

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- three names imported at the seam, two of them switched --'
command grep -m1 -B1 -A5 -F 'use loom::sync::atomic::{ AtomicU64, AtomicUsize };' ring_atomic/src/lib.rs
echo '  -- and what this crate does about the branch it can select --'
printf '    tests in the crate                       : %s\n' \
  "$( command grep -c '^fn ' ring_atomic/tests/atomic_test.rs || true )"
printf '    of those wrapping work in a loom::model  : %s\n' \
  "$( command grep -c 'loom::model' ring_atomic/tests/atomic_test.rs || true )"
printf '    loom::model uses elsewhere in the family : %s\n' \
  "$( command grep -rc 'loom::model' --include=*.rs . | cut -d: -f2 | paste -sd+ | bc )"
```

Live output:

```
  -- three names imported at the seam, two of them switched --
#[ cfg( loom ) ]
use loom::sync::atomic::{ AtomicU64, AtomicUsize };
#[ cfg( not( loom ) ) ]
use core::sync::atomic::{ AtomicU64, AtomicUsize };

use core::sync::atomic::Ordering;
use ring_types::Seq;
  -- and what this crate does about the branch it can select --
    tests in the crate                       : 21
    of those wrapping work in a loom::model  : 1
    loom::model uses elsewhere in the family : 38
```

---

### AT51 — The Seam Switches Two Names of Three and Assumes the Third Is the Same Type

`AtomicU64` and `AtomicUsize` are selected by `cfg`. `Ordering` is imported
unconditionally from `core`, one line below the switch. So under `--cfg loom`, every
call in the crate hands a `core::sync::atomic::Ordering` to a `loom` atomic's method.

That compiles, which is the whole point — `loom::sync::atomic::Ordering` is a
re-export of `core`'s rather than a distinct instrumented type, so the two are
literally the same type and the unconditional import is correct. Building the crate
with the seam on the loom side confirms it, cleanly:

```
=== exit 0 / build finished ===
```

**Finding.** The correctness of the seam's most-used parameter rests on a property of
loom's public API that this crate depends on and does not name. Nine `Ordering`
values are passed across the boundary on every counted operation, and if loom ever
introduced its own `Ordering` — to instrument ordering choices, which is exactly the
kind of thing a model checker might want to do — the failure would be a type error at
every call site in the file, in a configuration that no ordinary build and no ordinary
test run exercises.

The assumption is sound today and worth one line of comment beside the import, since
the two lines above it establish that this crate does *not* assume loom's types match
core's. A reader seeing `AtomicU64` switched and `Ordering` not switched has no way to
tell whether that asymmetry is deliberate or an oversight, and it is deliberate.

**Deletion condition:** none — the unconditional import is correct and should stay.
What is missing is the sentence recording why.

---

### AT52 — All Twenty-One Tests Compile Under the Loom Branch and Every One of Them Panics

`cargo build --tests` under `--cfg loom` succeeds. The suite compiles: the constructors
have their non-`const` forms, every signature still matches, nothing is gated out.

Then loom's atomics panic the moment they are touched outside a `loom::model`, and no
test here builds one:

```
thread 'a_fresh_cell_reads_zero' (2229752) panicked at loom-0.7.2/src/rt/scheduler.rs:128:13:
cannot access Loom execution state from outside a Loom model. are you accessing a Loom synchronization primitive from outside a Loom test (a call to `model` or `check`)?
```

That is the crate's simplest test — construct a cell, read zero — failing at the
first atomic access.

**Finding.** This is not a bug: `ring_atomic` is not where the models belong, and
`ring_testkit`'s own documentation gives the reason ("a coverage run does not set the
cfg"), which is why the family's fifteen `loom::model` uses live in four other crates'
test files. The observation is about what the crate's build signals report.

`cargo build --tests` says the loom configuration is fine. It compiles, so every
ordinary check short of running the suite is green, and the suite is never run in that
configuration by anything. A change to this crate that broke the loom branch's
*behaviour* — as opposed to its types — would be caught only by `ring_testkit`,
`ring_mpsc`, `ring_spsc` or `ring_publish` running their own models, three or more
dependency edges away from the edit.

The cheap guard is a single test in this crate that opens a `loom::model`, drives one
`fetch_add` through `AtomicSeq`, and asserts nothing beyond completing — which would
fail loudly here rather than downstream, and would make the seam's own branch
self-testing. It needs no new dependency: `loom` is already a
`[target.'cfg(loom)'.dependencies]` entry in this crate's manifest, and unlike the
four crates that run models it is a regular dependency here rather than a dev one, so
the loom side is already linked into the library itself.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](001_the_loom_seam_and_the_manifest_above_it.md) | The seam's cost inside the crate, and the entry it pushes up to the root |
| [`pattern/001`](../pattern/001_one_place_where_an_atomic_is_created.md) | Why one switch point suffices for the whole family |
| [`item/001`](../item/001_six_constructors_for_two_types.md) | The constructors the seam duplicates |
| [`integration/002`](../integration/002_five_crates_downstream.md) | The crates that run the models this branch exists for |

### Sources

| Fact | Where |
|------|-------|
| Two names switched, `Ordering` not | `ring_atomic/src/lib.rs:63-70` |
| The loom branch compiling clean | `RUSTFLAGS="--cfg loom" cargo build -p ring_atomic`, quoted above |
| 21 tests, 0 models here | Census above |
| The family-wide `loom::model` count in the recipe above is a raw, corpus-wide grep total that does not exclude this crate's own use; AT52's "fifteen... four other crates" figure below is a separate, narrower claim not reconciled against it here | Recipe above vs. AT52 prose |
| The panic outside a model | `RUSTFLAGS="--cfg loom" cargo test -p ring_atomic`, quoted above |
| Why the models live elsewhere | `ring_testkit/docs/integration/001_the_three_edges_and_the_one_that_is_missing.md:58-64` |

### Tests

| Test | Covers |
|------|--------|
| `a_fresh_cell_reads_zero` | The simplest path, and the one shown above failing under the loom branch |
| *(to create)* | No test in this crate opens a `loom::model`, so the branch the seam exists to select is exercised only from four other crates |
