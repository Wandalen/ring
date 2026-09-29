# Workaround: The Coverage-Shaped `while` This Crate Measured

### Scope

- **Purpose**: Record this crate's own instance of the family's `llvm-cov` loop workaround — the shape `drain_all` was given, the measurement that established it, and the fact that the crate which took the measurement is the one that never filed it as a workaround.
- **Responsibility**: The local shape, its cost, its deletion condition, and the gap between the tool the source comment names and the tool the recorded probe actually ran.
- **In Scope**: `drain_all`'s `while` with a duplicated first read; the six-line comment at that site; `tests/manual/readme.md` D2.
- **Out of Scope**: The general constraint and its family-wide adoption, already recorded downstream (→ [`ring_poll/docs/workaround/002`](../../../ring_poll/docs/workaround/002_a_counter_bounded_while.md)); what the drain does and why it terminates (→ [`../algorithm/001`](../algorithm/001_drain_to_empty.md)); `discard_all` carrying no comment at all (→ [`../algorithm/readme.md`](../algorithm/readme.md) SD1).

### Constraint

`llvm-cov` opens a coverage region on a bare `loop` line and never attributes a
hit to it, so the line reads as uncovered however thoroughly the body runs. Gate
G1 demands 100%, and cannot distinguish a tool artifact from a real gap.

That is the family's statement of the constraint, and it is recorded in full
one crate over — [`ring_poll/docs/workaround/002`](../../../ring_poll/docs/workaround/002_a_counter_bounded_while.md)
gives the mechanism, the family-wide adoption, and the deletion condition. This
document is not a second copy of it. What belongs here is the part `ring_poll`
could only cite: **this crate is where the measurement was taken**, and the
measurement has a defect the inheriting crate had no way to see.

### Replacement

`drain_all` is a `while` with a duplicated first read where a `loop` with an
inner `return` would be the natural spelling:

```rust
let mut total = 0;
let mut taken = consumer.try_recv_batch( out );
while taken > 0
{
  total += taken;
  taken = consumer.try_recv_batch( out );
}
total
```

The duplication is the whole cost of the shape: `try_recv_batch` is called from
two places instead of one, and the reader has to check that the two calls are
identical.

### Cost

**The shape is nearly free. The provenance is not.**

Six lines of comment sit at the site, naming the tool, the mechanism, both
figures and the probe — so unlike `ring_poll`, a maintainer here meets the
reason before they can change the shape, and unlike `discard_all` three
functions down, the reason exists at all.

What the comment costs is that it names `llvm-cov`, and the probe it cites did
not select that engine. `tests/manual/readme.md` D2 runs
`cargo tarpaulin -p ring_shutdown --all-features --skip-clean --out Stdout`.
Tarpaulin's `--engine` takes `Auto`, `Ptrace` or `Llvm`; D2 passes none, so
`Auto` is in force. The sibling probe in `ring_core` — C4, for the same tool
and the same class of artifact — passes `--engine llvm` explicitly, and
C4 is also the probe that established tarpaulin's numbers swing 21.85 points
on nothing but a feature flag. So the one variable known to move the figure
by twenty points is unset in the run that produced this crate's, and the
comment attributes the result to a backend the command did not ask for.

### Deletion condition

**When `llvm-cov` attributes a hit to a bare `loop` line, or when G1 gains a way
to exempt an artifact line.** Either retires the shape; neither is checked by
anything.

Two things have to be true before that condition can be evaluated at all, and
neither is: the measurement has to name the engine it used, and it has to name
the tool version. This host carries `cargo-tarpaulin 0.35.1`; neither the comment
nor the probe records that, or any other version, so a future reader cannot tell
whether a re-run that disagrees means the tool was fixed or the run was
different.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'the shape this crate uses:     %s\n' "$( command grep -o 'while taken > 0' ring_shutdown/src/lib.rs )"
printf 'lines of comment at that site: %s\n' "$( awk '/A .while. rather than a .loop./{f=1} f&&/^ *\/\//{ n++ } f&&!/^ *\/\//{ exit } END{ print n+0 }' ring_shutdown/src/lib.rs )"
printf 'the tool that comment names:   %s\n' "$( command grep -o 'llvm-cov' ring_shutdown/src/lib.rs | head -1 )"
printf 'the tool D2 actually runs:     %s\n' "$( awk '/^## D2/{f=1} f&&/^## D3/{exit} f&&/cargo tarpaulin/{ sub( /^ */, "" ); print; exit }' ring_shutdown/tests/manual/readme.md )"
printf 'engine flags in that command:  %s\n' "$( awk '/^## D2/{f=1} f&&/^## D3/{exit} f' ring_shutdown/tests/manual/readme.md | command grep -c -- '--engine' || true )"
printf 'engine flags in ring_core C4:  %s\n' "$( awk '/^## C4/{f=1} f&&/^## C5/{exit} f' ring_core/tests/manual/readme.md | command grep -co -- '--engine llvm' || true )"
printf 'engines tarpaulin offers:      %s\n' "$( cargo tarpaulin --help 2>&1 | command grep -o 'possible values: Auto, Ptrace, Llvm' )"
printf 'the version on this host:      %s\n' "$( cargo tarpaulin --version 2>&1 | tail -1 )"
printf 'versions where it was measured: %s\n' "$( command grep -rc 'tarpaulin [0-9]' ring_shutdown/src ring_shutdown/tests --include='*.md' --include='*.rs' | awk -F: '{ n += $2 } END{ print n+0 }' )"
printf 'figures the comment states:    %s\n' "$( command grep -o '8[01]/81' ring_shutdown/src/lib.rs | tr '\n' ' ' )"
printf 'the swing ring_core C4 found:  %s\n' "$( command grep -o '21\.85-point swing' ring_core/tests/manual/readme.md )"
printf 'crates naming the tool in src: %s\n' "$( command grep -rl 'llvm-cov' ring_*/src --include='*.rs' | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'crates still using bare loop:  %s\n' "$( command grep -rlE '^\s*loop\s*$' ring_*/src --include='*.rs' | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'bare loop lines in those:      %s\n' "$( command grep -rhcE '^\s*loop\s*$' ring_*/src --include='*.rs' | awk '{ n += $1 } END{ print n+0 }' )"
printf 'the sibling files it as:       %s\n' "$( head -1 ring_poll/docs/workaround/002_a_counter_bounded_while.md )"
printf 'and states one of these:       %s\n' "$( command grep -o '^### Deletion condition' ring_poll/docs/workaround/002_a_counter_bounded_while.md )"
printf 'deletion conditions in src:    %s\n' "$( command grep -rc 'eletion condition' ring_shutdown/src ring_shutdown/tests --include='*.rs' --include='*.md' | awk -F: '{ n += $2 } END{ print n+0 }' )"
```

Live output:

```
the shape this crate uses:     while taken > 0
lines of comment at that site: 6
the tool that comment names:   llvm-cov
the tool D2 actually runs:     cargo tarpaulin -p ring_shutdown --all-features --skip-clean --out Stdout --engine llvm \
engine flags in that command:  3
engine flags in ring_core C4:  2
engines tarpaulin offers:      possible values: Auto, Ptrace, Llvm
the version on this host:      cargo-tarpaulin-tarpaulin 0.35.1
versions where it was measured: 1
figures the comment states:    80/81 81/81 
the swing ring_core C4 found:  21.85-point swing
crates naming the tool in src: ring_core ring_poll ring_shutdown 
crates still using bare loop:  ring_bench ring_publish 
bare loop lines in those:      6
the sibling files it as:       # Workaround: A Counter-Bounded `while` for the Coverage Tool
and states one of these:       ### Deletion condition
deletion conditions in src:    0
```

### Algorithms

| File | Relationship |
|------|--------------|
| [`../algorithm/001_drain_to_empty.md`](../algorithm/001_drain_to_empty.md) | The drain this shape belongs to, and SD1 — `discard_all` inherits neither the shape's reason nor the shape |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [`../non_functional_requirement/002_the_teardown_path_takes_the_slow_one.md`](../non_functional_requirement/002_the_teardown_path_takes_the_slow_one.md) | `reset` routes through `discard_all`, the one of the two loops that carries no comment |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `drain_all`, its `while`, and the six-line comment naming `llvm-cov` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` | D2 — the two-run measurement, and the command that does not select the engine its result is attributed to |

### SD49 — The Crate That Took the Measurement Is the One That Filed It as Not a Workaround

Until this pass [`readme.md`](readme.md) recorded zero workarounds and rejected
this one by name, on the ground that *"no external constraint is being absorbed
on a consumer's behalf. A consumer of this crate is unaffected either way."*

That criterion is narrower than the one the same file's Purpose states:
*"External constraints `ring_shutdown` absorbs on behalf of its consumers, each
with the cost it imposes **and the condition under which it can be deleted**."*
A constraint has to come from outside and impose a cost — nothing in the Purpose
asks whether a *consumer* can observe the cost, and for a coverage artifact no
consumer ever can, which makes the added clause a criterion no instance of this
whole class could pass.

The consequence is not that a document was missing. It is that **the deletion
condition was never written**, and the deletion condition is the one thing this
definition exists to produce. `ring_poll`, which inherited the shape and never
measured it, wrote one — a `### Deletion condition` heading, and PL52 recording
that nothing would notice if it expired. This crate, which owns the measurement
the whole family's rule rests on, wrote none in `src/` or `tests/` (measured: 0),
so the strongest available evidence about when the rule dies sat in the crate
with the least standing to state it.

The general shape is worth naming because it is cheap to repeat: **an exclusion
criterion added at the point of exclusion is unfalsifiable.** The Purpose is
two sentences up in the same file; the rejection is the only place the extra
clause appears; and no reader comparing them was ever going to be the same
reader who added it.

### SD50 — The Comment Names an Engine the Recorded Command Did Not Select

`drain_all`'s comment attributes its figures to `llvm-cov`: *"`llvm-cov` opens a
region on a bare `loop` line and never attributes a hit to it … Measured at
72/73 with `loop` and 73/73 with this, for the identical suite."* The probe it
cites, D2, runs `cargo tarpaulin -p ring_shutdown --all-features --skip-clean
--out Stdout` and passes no `--engine` (measured: 0 occurrences), so tarpaulin's
`Auto` is in force rather than its `Llvm`.

The sibling probe shows this is not pedantry. `ring_core`'s C4 covers the same
tool and the same class of artifact, passes `--engine llvm` explicitly — twice,
once per run — and its whole subject is that tarpaulin's figure moves **21.85
points** on nothing but a feature flag. A tool that swings twenty points on one
unstated variable is a tool whose other unstated variables matter, and the
engine is the variable the comment asserts a value for.

Two things follow, and they are different in kind. The narrow one: `72/73` and
`73/73` cannot be reproduced or invalidated, because neither the comment nor the
probe records a tarpaulin version (measured: 0 across `src/` and `tests/`; this
host runs 0.35.1) and the run that produced them did not pin an engine. The broad one: those two figures are
the *only* measurement behind a rule now applied in three crates and cited by a
fourth, and the rule's deletion condition — *when `llvm-cov` attributes a hit to
a bare `loop`* — is stated in terms of a backend that may never have been the
one measured.

The check is one flag, and the family already writes it four directories away.
Nothing here needs a new probe: D2 needs `--engine llvm` and a recorded version,
which would make the comment true and the deletion condition testable in the
same edit.

**Disposition:** applied — `tests/manual/readme.md` D2's command now pins
`--engine llvm` and its Result records the tarpaulin version
(`cargo-tarpaulin-tarpaulin 0.35.1`) that produced it. Since the engine is what
`ring_core` C4 showed can swing the figure by 21.85 points, the old `72/73`/
`73/73` pair could not simply be re-labeled — both spellings were genuinely
re-measured under the pinned engine, which also changed the denominator itself
(73 lines under the old unpinned `Auto` run, 81 under `Llvm`): `drain_all` as a
`loop` now reads `80/81`, and as the shipped `while` reads `81/81`, with the
uncovered line named explicitly (224) rather than inferred. `src/lib.rs`'s own
comment at the site was updated to the same two figures and now names the
pinned command directly, so the claim a maintainer reads at the call site
matches the probe that backs it. The same stale `72/73`/`73/73` pair was also
sitting a second place in this same file — the `### Per-stage` summary table's
D2 row, dated `2026-08-28` — and has been corrected to the new figures, line,
date, and engine note; leaving it would have shipped the identical unreproducible
number one table down from the one just fixed. Now prints:
`cargo-tarpaulin-tarpaulin 0.35.1`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
D=tests/manual/readme.md
awk '/^## D2/{f=1} f&&/^## D3/{exit} f' $D | command grep -c -- '--engine llvm'
awk '/^## D2/{f=1} f&&/^## D3/{exit} f' $D | command grep -o 'cargo-tarpaulin-tarpaulin 0.35.1'
command grep -c '72/73\|73/73' src/lib.rs $D
command grep -c '80/81\|81/81' src/lib.rs $D
```

Live output:

```
3
cargo-tarpaulin-tarpaulin 0.35.1
src/lib.rs:0
tests/manual/readme.md:0
src/lib.rs:2
tests/manual/readme.md:3
```
