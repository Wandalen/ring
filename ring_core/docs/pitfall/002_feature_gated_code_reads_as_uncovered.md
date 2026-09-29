# Pitfall: Feature-Gated Code Reads as Uncovered

### Scope

- **Purpose**: Record that this crate's cargo feature makes it two programs, that the family's measurement machinery assumed one, and what the resulting numbers looked like before that was found.
- **Responsibility**: The trap's shape, the measured numbers, why the wrong reading is the plausible-looking one, and what was changed.
- **In Scope**: Line-coverage and `cargo check` readings of a crate carrying `#[ cfg( feature = ... ) ]` and `#[ cfg( not( ... ) ) ]` code.
- **Out of Scope**: Whether the crossbeam backend should exist (→ [`workaround/001`](../workaround/001_crossbeam_queue_as_interim_backend.md)); coverage tooling choice, which is the harness's.

### Trap

This was the family's **only crate with a cargo feature** when this was
written — verified:

```sh
cd "$(git rev-parse --show-toplevel)"
for d in ring_*/; do
  awk '/^\[features\]/{inf=1;next} /^\[/{inf=0} inf && /^[a-z_-]+ *=/{printf "%s ", $1}' "$d/Cargo.toml"
done
```

Live output:

```
crossbeam default crossbeam crossbeam default crossbeam 
```

now returns entries for four crates, not `ring_core` alone — in glob order:
`ring_bench` (`crossbeam`), `ring_core` (`default`, `crossbeam`),
`ring_factory` (`crossbeam`), `ring_handle` (`default`, `crossbeam`). Of the
three new entries, only `ring_bench` and `ring_factory` carry their own
`#[ cfg( feature = "crossbeam" ) ]` code in `src/lib.rs`; `ring_handle`'s
feature forwards to `ring_core`'s and gates nothing locally. `ring_core` was
the first crate for which "run the tests and read the number" has to specify
*which build*, and the machinery that reads those numbers was written before
there was anything to specify. **Open, flagged for follow-up:**
`g1_coverage.sh` and `g2_docs.sh` still comment that `ring_core` is the only
crate with a feature — whether `ring_bench` and `ring_factory` need the same
dual-build gate treatment this pitfall documents is a question this instance
does not answer.

Two independent failures followed, in opposite directions.

**Coverage counted code the compiler had deleted.** `cargo tarpaulin` reports
`cfg`-eliminated lines as uncovered rather than omitting them from the
denominator. Measured, for the identical test suite:

| Build | Reading | Uncovered lines |
|---|---|---|
| default (`crossbeam` off) | **93/119 — 78.2%** | 26, every one a `Storage::Crossbeam` / `ProducerInner::Crossbeam` / `ConsumerInner::Crossbeam` arm |
| `--all-features` | **120/120 — 100%** | none |

A 21.85-point swing with nothing tested differently. Verified stable: repeated
runs agree, and dropping `--skip-clean` does not change it, so it is the tool's
behaviour rather than a stale-artifact effect. Gate G1 ran the first form.

**Compilation checked only the half the feature turns on.** `--all-features`
compiles the `#[ cfg( feature = ... ) ]` arms and skips every
`#[ cfg( not( ... ) ) ]` one. Gate G2 ran `--all-features` alone, so the default
build was compiled by **no gate at all** — it could have stopped compiling
entirely and all six gates would still have reported REACHED.

**Both wrong readings are the plausible-looking ones.** 78.2% reads as "someone
has not finished writing tests", which is a familiar and actionable-sounding
state; and one `cargo check` reads as thorough because `--all-features` sounds
like the maximal option. Neither looks like a measurement error, which is what
made them survivable.

### Failure

| Reading | What it says | What is true | Cost if believed |
|---|---|---|---|
| G1: `ring_core` at 78.2% | 26 lines lack tests | Those lines are not in the binary | Time spent writing tests for code the build does not contain; or, worse, the family's 100% threshold quietly relaxed to accommodate it |
| G2: default build unchecked | The crate compiles | The `--all-features` build compiles | A `cfg( not( ... ) )` arm can break with every gate green |
| Either, generalised | The gates measure the crate | The gates measure one of its two builds | **Every future feature-gated crate inherits both**, silently |

The last row is why this is filed as a pitfall rather than as a fixed bug. The
gates were amended, but the *shape* — machinery written for a
single-configuration family meeting its first multi-configuration member —
recurs the moment a second crate declares a feature.

### Mitigation

1. **Gate G1 now runs `--all-features`.** That is the honest reading: it is the
   build in which every counted line is also a line the binary contains, so the
   100% threshold means what it says. Safe family-wide because the four crates
   that now declare a `crossbeam` feature (`ring_bench`, `ring_core`,
   `ring_factory`, `ring_handle`) all forward to the same underlying
   capability rather than gating independent ones, and `loom` is a
   `RUSTFLAGS` cfg rather than a cargo feature — so `--all-features` still
   turns on exactly one thing.

2. **Gate G2 now also runs `--no-default-features --all-targets`.**
   `--all-targets` is not optional: without it the check compiles the library,
   finds no `cfg( not( ... ) )` code in it, and passes while never looking at
   the tests, which is where this crate's default-build-only code lives.

3. **A crate adding a cargo feature must be treated as a machinery change, not
   only a crate change.** The gates are the family's Validation Machinery, and
   a new configuration is a new thing for them to measure. `tests/manual/readme.md`
   C3 is the standing check that they cover both.

**What does not mitigate it: annotating the low number.** A note saying "the
78.2% is an artifact" leaves the gate red and the threshold meaningless, and the
next person to read it has to re-derive the argument. Building every line is
cheaper than explaining why some of them do not count.

### A second, smaller instance of the same shape

The crate briefly read **120/121** — one line short — and the line was `else`.
The bare keyword, on its own line under the family's brace style, where
`llvm-cov` opens a region nothing can execute. Both arms of that `if` were hit,
23 and 16 times respectively. It is now a `match`, which is what the other 32
crates happen to use:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -cE '^[[:space:]]*else[[:space:]]*$' ring_*/src/lib.rs | command grep -v ':0' || true
```

Live output:

```
ring_debug/src/lib.rs:1
```

**Expected: no output for a bare `if`/`else`.** This currently prints one hit,
`ring_debug`, flagged for follow-up rather than resolved here: that crate's
matching line is a `let`-`else` statement (a different construct from the
`if`/`else` branch this section is about), so whether it is a real instance of
the same coverage trap or a false positive of this grep's pattern is a
question about a different crate, out of scope for this instance. Any crate
this prints *for a genuine `if`/`else`* will read one line short of 100% for a
reason that has nothing to do with its tests. Filed here rather than
separately because it is the same trap in miniature — a measurement instrument
counting something that is not code — and because the two together are the
argument for the general rule: **when a coverage number moves and no test
changed, suspect the instrument before the tests.**

### A third instance — the gate sized to its widest arm

The two above are instrument problems: a number read low for a reason that was
never about the tests. This one runs the other way — a real assertion that
silently stopped happening, in the configuration nearly every consumer builds.

`every_backend_reports_the_capacity_it_was_configured_with` loops over all three
backends, and its own doc comment calls it "what holds the three backends to one
answer rather than a comment claiming they agree". It carried
`#[ cfg( feature = "crossbeam" ) ]` on the **function**, because one of its three
match arms calls `Ring::new_crossbeam`. But this crate declares `default = []` —
the crossbeam-*off* build is the default build — so the gate did not merely skip
the crossbeam arm. It deleted the whole assertion from the build most callers
get, leaving `Ring::capacity()` held to one answer for three backends in one
configuration and for no backend at all in the other. The number that would have
shown it — 19 tests against 22 — is a number no gate compares, because no gate
runs the default configuration's tests at all (Mitigation 2 above compiles them,
deliberately).

The rule the three instances share: **a `cfg` belongs on the code that cannot
compile without the feature, never on the general property that code is one case
of.** The `ring_on` helper in the same file had it right the whole time — the
gate sits on the `Backend::Crossbeam` match arm and the function is
unconditional. The fix was to make the test match its own neighbour: gate the arm,
drive the loop from `every_backend()`, which is already written twice under a
`cfg` and so reports exactly the backends the build offers.

Every remaining function-level gate must be one that genuinely cannot exist
without the feature, which is checkable by name:

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^#\[ cfg\( feature = "crossbeam" \) \]$/ { g = 1; next } g && /^#\[/ { next } g && /^fn / { print; g = 0; next } { g = 0 }' ring_core/tests/core_test.rs
```

Live output:

```
fn every_backend() -> Vec< Backend >
fn the_crossbeam_backend_ignores_the_producer_count()
fn crossbeam_honours_drop_oldest_by_evicting()
```

**Expected: three lines** — `every_backend` (which has a `not( ... )` twin, so
both builds get one), and the two whose names begin with the backend they are
about. A function whose name does not name crossbeam is the shape this section
describes. Exactly two of the three are `#[ test ]`, which is why the two
configurations differ by two: 22 tests by default, 24 with `--all-features`. A
gap wider than that list means a general property has gone missing from the
default build again.

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_no_atomic_of_its_own.md](../invariant/001_no_atomic_of_its_own.md) | The other property of this crate that only a command, not a test, can check |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_backend_swap_is_a_build_flag.md](../non_functional_requirement/001_backend_swap_is_a_build_flag.md) | The requirement that makes two builds necessary in the first place |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_crossbeam_queue_as_interim_backend.md](../workaround/001_crossbeam_queue_as_interim_backend.md) | Cost 2 of absorbing the dependency, of which this is the worked-out consequence |

### Sources

| File | Relationship |
|------|--------------|
| `bench_harness/gate/g1_coverage.sh` | Amended to `--all-features`; the comment there carries the measured figures |
| `bench_harness/gate/g2_docs.sh` | Amended to add the `--no-default-features --all-targets` check |
| `bench_harness/gate/readme.md` | The non-vacuity principle this extends — a gate that cannot tell "not started" from "finished" measures nothing, and one that cannot tell "not built" from "not tested" is the same failure |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` | C3 — both feature configurations are built by something. No test can check this; it is a property of which commands the gates run |
| `tests/manual/readme.md` | C4 — the two coverage readings, taken side by side, which is the only form in which the discrepancy is visible |

### CO49 — Sixteen Gates Mean Two Builds That Never Meet

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
printf 'cfg gates:   '; grep -c 'cfg( feature = "crossbeam" )' src/lib.rs
printf 'default:     '; grep -A1 '^\[features\]' Cargo.toml | tail -1
```

Live output:

```
cfg gates:   16
default:     default = []
```

`default = []`, so the ordinary `cargo test` compiles the crossbeam arms out
entirely. A coverage run reports them as uncovered lines; an `--all-features`
run covers them and changes the denominator, so the two percentages are not
comparable.

The instance already records the reading problem. What this adds is the count —
sixteen sites, all one spelling — and the consequence for the *other* direction:
`the_same_program_behaves_identically_on_every_backend` can only compare the
backends a given build contains (→ [`../invariant/002`](../invariant/002_uniform_delivery_across_backends.md), CO25),
so the default build's uniformity assertion covers two of three.
