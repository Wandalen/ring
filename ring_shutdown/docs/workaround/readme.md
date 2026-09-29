# Workaround Doc Definition

### Scope

- **Purpose**: External constraints `ring_shutdown` absorbs on behalf of its consumers, each with the cost it imposes and the condition under which it can be deleted.
- **Responsibility**: Two — a loop shape this crate measured and gave to the family, and a compile-failure probe standing in for a harness that already exists.
- **In Scope**: The constraint, the shape it forces, what it costs, and the condition that retires it.
- **Out of Scope**: `close` not revoking a raw producer, which is a chosen invariant rather than an absorbed constraint (→ [`../invariant/001`](../invariant/001_exactly_one_liveness_flag.md), [`../pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)); the general statement of the coverage constraint, recorded downstream (→ [`ring_poll/docs/workaround/002`](../../../ring_poll/docs/workaround/002_a_counter_bounded_while.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Coverage-Shaped `while` This Crate Measured](001_the_coverage_shaped_while_this_crate_measured.md) | The measurement the family's loop rule rests on, and the engine its own command did not select | 🔄 |
| 002 | [A Compile-Failure Probe With No Harness](002_a_compile_failure_probe_with_no_harness.md) | Two compile-time guarantees verified by hand on a date, in a workspace that already runs `compile_fail` | 🔄 |

**This directory recorded zero instances, and the argument it used to get there
was not the ordinary one.** The usual reason a crate has no workarounds is that
nothing outside constrains it, and that reason genuinely applies in part here:
the dependency closure is entirely in-house and the whole implementation is one
`AtomicBool`. But the readme did not stop at that. It named both candidates and
rejected them, and the ground it rejected the first on — *"no external
constraint is being absorbed on a consumer's behalf. A consumer of this crate is
unaffected either way"* — is a clause the Purpose two lines above does not
contain, and one no coverage artifact could ever satisfy (→ SD49).

The two instances are separate because they fail in opposite directions.
**`001`'s constraint is real and its evidence is thin**: `llvm-cov` genuinely
cannot attribute a hit to a bare `loop`, this crate genuinely measured it, and
the family adopted the result — but the source comment names an engine the
recorded command left unset, in a tool whose figure is known to swing 21.85
points on one unstated flag (→ SD50). **`002`'s evidence is sound and its
constraint is gone**: D1's two compile errors are real and precisely recorded,
and the harness that would run them continuously already ships in three sibling
crates under a command `verb/test` already executes (→ SD52).

That split is what makes the pair worth reading together. One is a workaround
whose deletion condition cannot yet be evaluated because the measurement behind
it is under-specified; the other is a workaround whose deletion condition was
already satisfied and nobody checked. Both failures are invisible from inside
the crate, and both are one command away from being visible — which is the case
for writing the condition down, and the reason this definition exists.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/workaround
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each names its constraint:    %s\n' "$( command grep -lc '^### Constraint' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'each names when it dies:      %s\n' "$( command grep -lc '^### Deletion condition' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'the criterion the purpose set: %s\n' "$( awk '/^### Overview Table/{ exit } { print }' readme.md | command grep -o 'the condition under which it can be deleted' )"
printf 'deletion conditions in src:   %s\n' "$( cd ../..; command grep -rc 'eletion condition' src tests --include='*.rs' --include='*.md' | awk -F: '{ n += $2 } END{ print n+0 }' )"
printf 'crates filing the same shape: %s\n' "$( cd ../../..; command grep -rl 'llvm-cov' ring_*/docs/workaround --include='*.md' | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'crates still using bare loop: %s\n' "$( cd ../../..; command grep -rlE '^\s*loop\s*$' ring_*/src --include='*.rs' | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'compile-fail harnesses nearby: %s\n' "$( cd ../../..; { command grep -rl '```compile_fail' ring_*/src --include='*.rs'; command grep -rl 'trybuild' ring_*/Cargo.toml; } | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'and what already runs them:   %s\n' "$( cd ../../../..; command grep -o 'cargo test --doc --all-features' /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/verb/test | head -1 )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
each names its constraint:    2
each names when it dies:      2
the criterion the purpose set: the condition under which it can be deleted
deletion conditions in src:   0
crates filing the same shape: ring_poll ring_shutdown 
crates still using bare loop: ring_bench ring_publish 
compile-fail harnesses nearby: ring_handle ring_mpsc ring_spsc 
and what already runs them:   cargo test --doc --all-features
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD49 | the crate that took the measurement is the one that filed it as not a workaround | n/a — doc gap | This readme recorded zero instances and rejected the coverage-shaped loop by name, on the ground that *"no external constraint is being absorbed on a consumer's behalf. A consumer of this crate is unaffected either way"* — a clause its own Purpose does not contain (*"…each with the cost it imposes and the condition under which it can be deleted"*), and one no coverage artifact could pass, since no consumer of any crate can observe a coverage figure; the consequence is not a missing document but a missing **deletion condition**, the one artefact this definition exists to produce, and the asymmetry is sharp — `ring_poll`, which inherited the shape and never measured it, wrote a `### Deletion condition` heading and filed PL52 against its unenforceability, while this crate, which owns the 72/73→73/73 measurement the family's whole rule rests on, wrote none anywhere in `src/` or `tests/` (measured: 0); the general form is that **an exclusion criterion introduced at the point of exclusion is unfalsifiable** — the Purpose was two lines up, the extra clause appears nowhere else, and no reader comparing the two was ever going to be the one who added it. |
| SD50 | the source comment names an engine the recorded command did not select | **misleading doc** | `drain_all`'s six-line comment attributes its figures to `llvm-cov` — *"`llvm-cov` opens a region on a bare `loop` line and never attributes a hit to it … Measured at 72/73 with `loop` and 73/73 with this"* — while `tests/manual/readme.md` D2, the probe it cites, runs `cargo tarpaulin -p ring_shutdown --all-features --skip-clean --out Stdout` and passes no `--engine` (measured: 0), leaving tarpaulin's `Auto` in force rather than its `Llvm`; the sibling probe shows this is not pedantry, since `ring_core`'s C4 covers the same tool and the same artifact class, passes `--engine llvm` explicitly twice, and exists precisely to record that tarpaulin's figure swings **21.85 points** on one unstated flag — so the two figures cannot be reproduced or invalidated (no document in `src/` or `tests/` records a tarpaulin version; this host runs 0.35.1), and they are the *only* measurement behind a rule now applied in three crates and cited by a fourth, whose stated deletion condition is phrased in terms of a backend that may never have been measured. |
| SD51 | ten documents rest on a compile-time property nothing re-runs | n/a — coverage | Ten documents in this crate assert that some expression does not compile, and every one traces to `tests/manual/readme.md` D1, whose verdict is *"Result (2026-08-28): both hold"* — produced by writing `tests/d1_probe.rs`, building, reading `E0599` and `E0382`, and deleting the file, which does not exist (measured: 0) and never did outside that run; the 20 test functions that do exist assert runtime behaviour exclusively, so the crate's two compile-time guarantees are the only claims it makes that no automated check defends, and they are the two it leans on hardest ([`invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)'s termination argument and [`pattern/001`](../pattern/001_proof_token_orders_two_operations.md)'s rule 3 are compile-time or nothing); two one-line changes delete them with the suite still green — an inherent `drain_all` on `Shutdown`, and `#[ derive( Clone ) ]` on `Stopped`, the latter merely formalising the duplication SD16 already records `Stopped::shutdown` permitting — and the shape generalises, because **a property verified by a dated manual probe has a maintainer, not a test**, across 33 crates that each carry a `tests/manual/readme.md` full of exactly this kind of verdict. |
| SD52 | the deletion condition was already satisfied when the workaround was written | n/a — unadopted | The constraint behind D1 — *there is no harness for a compile-failure claim* — was already false in this family, twice and by two mechanisms: ```` ```compile_fail ```` blocks are collected by rustdoc from the library target and compiled by `cargo test --doc`, which `verb/test` runs on line 14 under `RUSTDOCFLAGS="-D warnings"`, and `ring_mpsc` and `ring_spsc` carry nine such blocks between them, while `ring_handle` additionally depends on `trybuild` and version-controls seven `.stderr` files under `tests/ui/` — strictly more than D1 achieves, which records its two error codes in prose and compares them to nothing; `ring_mpsc`'s module header even writes down the trap that makes the manual route look necessary (*"rustdoc collects doc tests from the library target only; the same blocks in `tests/mpsc_test.rs` would never be compiled"*), which is very likely what happened here, and the correction was one crate away — so the cost is not the probe but that this crate spent its compile-failure evidence on a form that expires while both non-expiring forms were already in the workspace, already wired into the same command, and already documented, with the definition that would have surfaced the mismatch being the one that recorded zero instances (→ SD49). |
