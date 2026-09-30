---
name: architecture-review
description: Review a proposal that changes ring's crate boundaries, the claim/publish/commit protocol, the export contract, the unsafe allowlist or the gate harness — before implementation.
disable-model-invocation: true
argument-hint: "[proposal, issue, or crates involved]"
---

# Architecture review

Reviews a proposal; does not implement it. Not for local changes behind an unchanged contract.

## Pressure points specific to this repo

Ground every finding in these, not in a generic checklist:

- **One mechanism per crate.** A proposal that merges two mechanisms, or splits one across
  crates, must say what each crate's tests and docs can then no longer verify alone.
- **Tier order.** Workspace `members` is dependency order; a new edge may not point upward.
  Dev-dependencies count for the build order too.
- **Export contract.** Five crates (`export_surface.txt`, G5). Widening it widens what can never
  change freely again; `ring_flush::Flusher::new` taking a `ring_core::Producer` is the known
  leak.
- **`unsafe` allowlist.** Two crates actually use it; "a fourth needs a decision". A proposal
  that needs a new one must show why `UnsafeCell` per slot in an existing ring does not do.
- **Orderings.** Any change to who stores or loads a cursor or stamp rewrites the happens-before
  pairs; name them, and name the loom model that will prove them.
- **Measured before adopted.** Replacing a mechanism needs the benchmark verdict first
  (`ring_mpsc/docs/non_functional_requirement/001_measured_before_adopted.md`).
- **Gates.** The change must keep every declared gate meaningful, not only green. A gate
  narrowed to pass is a regression.

## Workflow

1. **Scope**: the decision, requirements, non-goals, done criteria. Facts vs. assumptions.
2. **Current state** from code and the crates' `docs/` — not from module prose, which drifted
   in the extraction.
3. **Proposal**: changed boundaries, contracts, orderings, dependencies, failure modes.
4. **Evaluate**: simplicity (is there a smaller change?), cohesion, ownership of each atomic,
   contract compatibility, failure and overflow behaviour, verification (tests, loom,
   trybuild, gates), migration of the doc corpus.
5. **Alternatives**: the current design and at least one simpler option.
6. **Verdict**: approve / approve with conditions / revise / reject.

## Output

Verdict and rationale; current vs. proposed; findings by impact with evidence and required
action; the simpler alternative if one exists; verification gates; open decisions (record
them with the `decision` skill).
