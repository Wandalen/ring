# Workaround: A Compile-Failure Probe Standing In for a Harness

### Scope

- **Purpose**: Record that this crate's two strongest claims — the ones only the compiler can refute — are verified by a throwaway file written, run and deleted by hand, and that the harness which would automate them already ships twice in this family.
- **Responsibility**: The constraint as it was understood, the hand-run probe that stands in for a test, what the substitution costs, and why the deletion condition was already met before the workaround was needed.
- **In Scope**: `tests/manual/readme.md` D1; the throwaway `tests/d1_probe.rs`; ```compile_fail``` doctests and `trybuild` as the two available harnesses.
- **Out of Scope**: What the token guarantees and how narrowly (→ [`../type/001`](../type/001_stopped_proof_token.md), SD46); whether the token should be unique (→ [`../decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md)); the coverage-shaped loop (→ [`001`](001_the_coverage_shaped_while_this_crate_measured.md)).

### Constraint

A test suite demonstrates that code *runs*. It cannot demonstrate that code
*does not compile*: a non-compiling case in `tests/` fails the build, and a case
removed to make the build pass proves nothing. So the two properties
[`../type/001`](../type/001_stopped_proof_token.md) rests on —

1. `drain_all` is not reachable on an open ring through this crate's surface
2. a token consumed by `reopen` cannot be drained through afterwards

— have no place in `tests/shutdown_test.rs`, whose 20 test functions all assert
runtime behaviour.

### Replacement

`tests/manual/readme.md` D1: write `ring_shutdown/tests/d1_probe.rs`
containing both cases, run `cargo build --tests -p ring_shutdown`, read the
errors, delete the file. Recorded once, with the two error codes named — `E0599`
for *the method is not there* and `E0382` for *the proof was consumed* — because
two distinct mechanisms is what makes them two properties rather than one stated
twice.

### Cost

**The verdict is a date, and nothing re-runs it.**

Ten documents in this crate assert that something does not compile. All of them
trace to D1, and D1 says *"Result (2026-08-28)"*. Between that date and any
future reader, the suite is the only thing running, and the suite is green for
both of these changes:

| Change | Suite | Property lost |
|---|---|---|
| Add an inherent `drain_all` to `Shutdown` | green | 1 — the drain becomes reachable without a close |
| `#[ derive( Clone ) ]` on `Stopped` | green | 2 — a copy survives the `reopen` that consumed the original |

Neither is a strange thing to do. The first is what a caller asks for the second
time they write `let s = shutdown.close(); s.drain_all( … )`. The second is what
a maintainer reaches for when a token has to cross a struct boundary — and
[`../decisions/readme.md`](../decisions/readme.md) SD16 already records that
`Stopped::shutdown` reconstitutes a second token from a shared borrow, so the
derive would only make explicit a duplication the type already permits.

The cost is therefore not the manual labour. It is that the crate's two
compile-time guarantees degrade to *nobody has broken them since a Tuesday in
August*, in a family whose gate demands 100% line coverage of the code that
merely runs.

### Deletion condition

**When a compile-failure harness exists that `verb/test` already runs.** It does.

`verb/test` line 14 is `RUSTDOCFLAGS="-D warnings" cargo test --doc
--all-features`, which collects and compiles every ```compile_fail``` block in
every library target in the workspace. Two crates in this family already use
them — ten blocks between `ring_mpsc` and `ring_spsc` — and a third,
`ring_handle`, carries a full `trybuild` UI harness with seven recorded
`.stderr` files. `ring_mpsc`'s module header even writes down the reasoning that
leads here: *"These are `compile_fail` doc tests rather than integration tests
because rustdoc collects doc tests from the library target only; the same blocks
in `tests/mpsc_test.rs` would never be compiled."*

D1's Case A and Case B are two such blocks. The infrastructure cost is zero
dependencies, zero new files and zero new commands, and the properties move from
dated to continuous.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'the probe standing in here:    %s\n' "$( command grep -o '^## D1 — .*' ring_shutdown/tests/manual/readme.md )"
printf 'the file it writes by hand:    %s\n' "$( command grep -o 'ring_shutdown/tests/d1_probe\.rs' ring_shutdown/tests/manual/readme.md | head -1 )"
printf 'does that file exist:          %s\n' "$( ls ring_shutdown/tests/d1_probe.rs 2>/dev/null | wc -l )"
printf 'the date on its verdict:       %s\n' "$( awk '/^## D1/{f=1} f&&/^## D2/{exit} f' ring_shutdown/tests/manual/readme.md | command grep -o 'Result ([0-9-]*)' )"
printf 'error codes it recorded:       %s\n' "$( command grep -ohE 'E0[0-9]{3}' ring_shutdown/tests/manual/readme.md | sort -u | tr '\n' ' ' )"
printf 'test fns in the suite:         %s\n' "$( command grep -c '^#\[ test \]' ring_shutdown/tests/shutdown_test.rs )"
printf 'of those, compile-fail ones:   %s\n' "$( command grep -c '```compile_fail' ring_shutdown/tests/shutdown_test.rs || true )"
printf 'compile_fail blocks in our src: %s\n' "$( command grep -c '```compile_fail' ring_shutdown/src/lib.rs || true )"
printf 'docs resting on that claim:    %s\n' "$( cd ring_shutdown; command grep -rlE 'does not compile|no longer compiles|compile error|E0[0-9]{3}' docs --include='*.md' | command grep -vE '^docs/(workaround|definition)/' | wc -l )"
printf 'family crates with such blocks: %s\n' "$( command grep -rl '```compile_fail' ring_*/src --include='*.rs' | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'blocks across those:           %s\n' "$( command grep -rhc '```compile_fail' ring_*/src --include='*.rs' | awk '{ n += $1 } END{ print n+0 }' )"
printf 'why ring_mpsc chose them:      %s\n' "$( awk '/^### Regenerate/{ exit } { print }' ring_mpsc/src/lib.rs | tr '\n' ' ' | sed 's/  */ /g' | command grep -o 'rustdoc collects doc tests from the library target only' )"
printf 'family crates with trybuild:   %s\n' "$( command grep -rl 'trybuild' ring_*/Cargo.toml | cut -d/ -f1 | tr '\n' ' ' )"
printf 'expected-error files it keeps: %s\n' "$( ls ring_handle/tests/ui/*.stderr 2>/dev/null | wc -l )"
printf 'what verb/test already runs:   %s\n' "$( command grep -o 'cargo test --doc --all-features' verb/test | head -1 )"
printf 'under which flags:             %s\n' "$( command grep -o 'RUSTDOCFLAGS="-D warnings" cargo test --doc' verb/test | head -1 )"
```

Live output:

```
the probe standing in here:    ## D1 — The two `Stopped` properties are compile errors, not runtime ones
the file it writes by hand:    ring_shutdown/tests/d1_probe.rs
does that file exist:          0
the date on its verdict:       Result (2026-08-28)
error codes it recorded:       E0382 E0599 
test fns in the suite:         20
of those, compile-fail ones:   0
compile_fail blocks in our src: 0
docs resting on that claim:    10
family crates with such blocks: ring_mpsc ring_spsc 
blocks across those:           10
why ring_mpsc chose them:      rustdoc collects doc tests from the library target only
family crates with trybuild:   ring_handle 
expected-error files it keeps: 7
what verb/test already runs:   cargo test --doc --all-features
under which flags:             RUSTDOCFLAGS="-D warnings" cargo test --doc
```

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_stopped_proof_token.md`](../type/001_stopped_proof_token.md) | The two properties D1 verifies, and SD46 — the qualifier D1 supplies and the document omits |

### Decisions

| File | Relationship |
|------|--------------|
| [`../decisions/002_should_a_stopped_token_be_unique.md`](../decisions/002_should_a_stopped_token_be_unique.md) | The uniqueness question; `derive( Clone )` is one of the changes the missing harness would not catch |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_drain_terminates_because_close_preceded_it.md`](../invariant/002_drain_terminates_because_close_preceded_it.md) | The termination argument that rests on property 1 holding at compile time |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `Shutdown`, `Stopped`, and the absence of any ```compile_fail``` block |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` | D1 — the throwaway-probe procedure, its two error codes, and its date |

### SD51 — Ten Documents Rest on a Property Nothing Re-Runs

Ten documents in this crate assert that some expression does not compile. Every
one of them traces to a single source of truth: `tests/manual/readme.md` D1,
whose verdict reads *"Result (2026-08-28): both hold"*. `tests/d1_probe.rs` was
written, built, read and deleted; it does not exist (measured: 0), and the 20
test functions that do exist assert runtime behaviour exclusively.

So the crate's two compile-time guarantees are the only claims it makes that no
automated check defends, and they are the two it leans on hardest —
[`../invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)'s
termination argument is *"the drain cannot start on an open ring"*, and
[`../pattern/001`](../pattern/001_proof_token_orders_two_operations.md)'s rule 3
is *"anything that undoes A consumes the token"*. Both are compile-time or
nothing.

Two one-line changes delete them and leave the suite green: an inherent
`drain_all` on `Shutdown`, and `#[ derive( Clone ) ]` on `Stopped`. Neither is
exotic; the second is the natural response to a token needing to cross a struct
boundary, and SD16 already records that `Stopped::shutdown` hands out a second
token from a shared borrow anyway, so the derive would formalise something the
type already allows.

The shape generalises past this crate. **A property verified by a dated manual
probe is a property with a maintainer, not a test** — it holds exactly as long
as everyone who edits the file has read the file that records it, and 33 crates
in this family carry a `tests/manual/readme.md` full of exactly this kind of
verdict.

### SD52 — The Deletion Condition Was Met Before the Workaround Was Written

A workaround is entitled to exist while its constraint does. This one's
constraint — *there is no harness for a compile-failure claim* — was already
false in this family when D1 was written, twice over and in two different ways.

`compile_fail` doctests are collected by rustdoc from the library target and
compiled by `cargo test --doc`, which `verb/test` runs on line 14 under
`RUSTDOCFLAGS="-D warnings"`. `ring_mpsc` and `ring_spsc` between them carry
ten such blocks. Separately, `ring_handle` depends on `trybuild` and keeps
seven `.stderr` files under `tests/ui/`, so the *exact text* of its expected
compiler errors is version-controlled and diffed on every run — which is
strictly more than D1 does, since D1 records `E0599` and `E0382` in prose and
nothing compares them to anything.

`ring_mpsc`'s module header goes further and writes down the reasoning that
would have led here, including the trap that makes the manual route look
necessary: *"rustdoc collects doc tests from the library target only; the same
blocks in `tests/mpsc_test.rs` would never be compiled."* A reader who tries
the obvious thing — put the failing cases in `tests/` — discovers they are
silently not compiled, concludes no harness exists, and writes a manual probe.
That is very likely what happened, and the correction was one crate away.

The cost of the miss is not the probe. It is that this crate spent its
compile-failure evidence on a form that expires, while the two forms that do not
were already in the same workspace, already wired into the same command, and
already documented — and the definition that would have surfaced the mismatch is
the one that recorded zero instances (→ SD49).
