# A gate's REACHED counts as evidence only after the gate has been seen to fail for the reason it names

Status: Accepted

## Context

The stage gates under `bench_harness/gate/` exit 0 (REACHED) or non-zero (NOT REACHED), and a stage verdict cites
their readings. A reading is worth citing only if the gate can tell the state it grades from the state it does not.
[gate/readme.md](../../gate/readme.md) records the first proof that this cannot be assumed. Against the untouched
`ring_*` skeletons, `cargo tarpaulin` reported `No coverable lines found` and `#![deny(missing_docs)]` passed in every
crate, so the coverage and documentation checks were green before the family had any code. Every gate now pairs its
assertion with a check that there is something to measure.

That pairing covers one way a gate goes wrong. Building the gates found three more, and found that the pairing check
is itself code that can fail:

- A gate that cannot pass. `g1_coverage.sh` once read tarpaulin's summary percentage. That figure is workspace-wide,
  because `-p` selects which packages' tests run, not which sources the report covers. Crates outside the family sat
  in the denominator, and the gate stayed short of 100% while the family itself was fully covered.
- A gate that measures the wrong subject. `g2_docs.sh` counted `missing_docs` diagnostics and public items. Neither
  reads `readme.md`, so implemented crates with full suites still opened with "Skeleton" and the gate passed.
- A declaration that decays. A `from` block under `declared/<family>/mutant/` that stops matching its target after a
  refactor shrinks the set of defects G12 replays, and the verdict line does not change. An allowlist entry for a
  crate that no longer uses its exemption has the same shape.
- A pairing check that is vacuous. `g2_docs.sh` first counted public items with `pub(`, which also matches the
  crate-private `pub(crate)` and `pub(super)`. A crate exporting nothing passed the check written to catch it.

## Decision

A gate's REACHED reading counts as evidence only after the gate has been seen to report NOT REACHED for the reason its
message names. Until then a stage verdict does not cite it. In practice:

- A gate written before the work it grades is run against the unimplemented family first, and must read NOT REACHED
  there. The ring family's gates were written ahead of the crates and read zero reached on the skeletons.
- Where a skeleton cannot provoke the failure, the failure is provoked on finished code before the gate is declared.
  On skeletons G12 has no code to mutate, so its `from` blocks match nothing and it fails for the wrong reason. A
  failure for the wrong reason proves nothing about the right one. G12's two failure modes were provoked on the
  finished family with throwaway probes: a `from` block edited to match nothing, and a mutation that edits only a doc
  comment, which no test can observe. Both read NOT REACHED.
- A gate reads its own subject, not a superset. G1 recomputes coverage from tarpaulin's per-file `Tested/Total Lines`
  breakdown, restricted to the crates in scope. Its `total > 0` check therefore counts the family's own lines, and an
  all-skeleton family reads zero coverable lines instead of borrowing lines from unrelated crates.
- Declarations are checked in both directions. G6 fails on an undeclared crate carrying `#![ allow( unsafe_code ) ]`
  and on a declared crate that no longer carries it. G12 fails on a `from` block that matches zero times or more than
  once, and on a mutation that leaves the suite green. None of these is skipped.
- A change to a pairing check is verified by feeding it the case it exists to reject. The G2 fix to `pub ` was checked
  by adding a `pub(crate) fn` to a skeleton crate and confirming the gate still reported that crate as exporting
  nothing.

## Alternatives considered

- **Trust a gate once it passes.** The skeleton baseline showed two gates green on an empty family. A pass from a gate
  never seen failing cannot tell "not started" from "finished".
- **Baseline every gate against skeletons.** Works for gates whose subject is absent on a skeleton. G12 fails there for
  a reason unrelated to the one it grades, so that baseline proves nothing about it.
- **Skip a stale declaration, or a mutation the suite cannot see.** Keeps the board green as the code moves. The
  mutant set then shrinks with each refactor while the gate prints the same verdict, so it decays toward a vacuous pass
  with no visible change.
- **Check allowlists in one direction.** G6 first asked only whether every crate carrying the opt-out was declared. A
  declared crate that stopped needing the exemption was never looked at, so the allowlist could only drift looser
  than the code.
- **Read the tool's summary figure.** Less code than recomputing per file. For G1 the summary was workspace-wide and
  made the gate impossible to pass.

## Consequences

- Each gate carries a second check and the code to scope its own reading, so the scripts are longer than their
  assertions.
- The proof that a family gate can fail is a reading taken once, when the gate or its pairing check changes. Nothing
  re-runs it. Only the corpus gates have seeded fixtures (`declared/ring/corpus_control/`, run by
  `run_all.sh --control`). The gates `declared/ring/gates.txt` lists have none, so an edit that makes a pairing check
  vacuous again gives no signal.
- Run output does not tell a gate that is vacuous for now, because its subject does not exist yet, from one that is
  vacuous by construction. Both read REACHED. Whoever cites a verdict has to know which gates have been seen failing.
- G6 reads NOT REACHED for the ring family while `ring_core` stays on `declared/ring/unsafe_allowlist.txt` without
  carrying the opt-out. The allowlist header puts `ring_core` there on architectural grounds, because it assembles
  slot storage with the cursors that bound it, and the soundness invariant is stated in terms of both. The two-way
  check is empirical. Settling it means re-ruling the allowlist, not editing the gate.
- Revisit when the family gates get seeded control fixtures like the corpus gates. The proof then becomes a repeatable
  run instead of a one-time reading.
- Revisit when a gate is proposed whose failure neither a skeleton nor a probe can provoke. This rule gives no way to
  admit it.
