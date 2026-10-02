# Line coverage means every line ran, and defence against defects is graded by replaying mutations

Status: Accepted

## Context

G1 (`g1_coverage.sh`) requires 100% line coverage over the family. Coverage records which lines executed, not
whether any test checked what they produced.

A manual `ring_bench` round reinstated the crate's headline defect, feeding the slot counters from `reported` instead
of `received`, and every test stayed green at full coverage. Both mutated lines ran under two tests, and neither test
looked at the counters on an input where the two values differ. `declared/ring/mutant/b5_counter_mapping.mutant`
records the details. `mutant_survey.sh` later found the same shape in crates nobody had reason to suspect, including
the workload generator in this crate.

Coverage still finds things assertions miss. Both `ring_bench` fixtures once offered a record count that the batch
size divided exactly, so the staged candidate never reached the `FlushOutcome::Flushed` arm after
`ring_flush::Flusher::drain_final`. Coverage reported the unreached line. No assertion did.

So the question is what a green board, and G1 in particular, claims, and what grades defence instead.

## Decision

- G1 stays. Its 100% means every line in scope ran, and nothing more. It catches code no test reaches. It does not
  show that a test checks what that code produces.
- Defence against known defects is graded by G12, which reinstates each defect recorded under
  `declared/<family>/mutant/` and requires the crate's suite to go red. `mutant_survey.sh` proposes new candidates, a
  person rules on each, and G13 fails when a crate's source or tests have moved since its last clean sweep.
  [gate/readme.md](../../gate/readme.md) describes how the survey and the gates divide that work.
- A green board is a claim about proxies. Each gate checks a cheap, mechanical property one judgement step away from
  the property it stands for. A check that needed the judgement could not exit 0 or 1. The step each gate leaves open:

| Gate | Proves | Does not prove |
|---|---|---|
| G1 coverage | Every line under `src/` ran at least once | That any value a line produced was checked |
| G2 docs | Every crate exports and documents public items, its doctests run, and no implemented crate calls itself a skeleton | That the prose is true |
| G3 features | Each declared feature is cited by some test | That the citing test exercises the feature |
| G4 manual | Every crate carries a dated run record | That the run went as written, or that its predictions held |
| G5 exports | No crate outside the family depends on an undeclared family crate | That the declared exports are enough for a consumer. [ring_bench/readme.md](../../../ring_bench/readme.md) records one case where they are not |
| G6 unsafe | The workspace `unsafe-code = "deny"` is in force, and opt-outs are confined to declared crates that justify them | That the `unsafe` code is sound. No gate reads it |
| G12 mutation | Each recorded defect, reinstated, turns its crate's suite red | That the suite catches a defect nobody has recorded |
| G13 freshness | Each crate was swept clean against the source and tests it has now | That the sweep found everything, or that its rulings were right. It compares digests |

- A test that relates two quantities runs where they can differ. That applies to the fixture: a candidate where
  `reported` equals `received` cannot catch a swap between them. It applies to the fixture's dimensions: a batch that
  divides the record count never runs the remainder path. And it applies to the assertion: a cross-check whose two
  sides both derive from the function under test moves with any mutation of it, as `ring_core::Producer::is_full`,
  defined as `free_capacity() == 0`, does against `ring_core::Producer::free_capacity`.
  [mutant/readme.md](../../gate/declared/ring/mutant/readme.md) states the one-polarity form of the same rule.

## Alternatives considered

- **Treat 100% line coverage as defence.** The `ring_bench` round reinstated a defect at full coverage and every test
  stayed green. Coverage read 100% both while the defect went undetected and after the fix, so it cannot tell the two
  apart.
- **Drop G1, since coverage proves so little.** Coverage found the unreached `Flushed` arm that no assertion found.
  Each instrument finds holes the other misses, and a project that runs only one has blind spots shaped like the
  other.
- **Gate the judgement steps.** Whether prose is true, or whether a citing test exercises its feature, needs a reader.
  A gate that guesses at it misfires, and a gate that misfires stops being read.
- **Make the survey a gate with a survivor threshold.** Many survivors are equivalent mutants that no input can tell
  apart from the original. A survivor count is not a defect count, so the survey proposes and stays off the board.

## Consequences

- G3 is satisfied by any one citation. A feature owned by several crates passes when one test file cites it, and a
  feature whose written criterion names a mechanism no test has still passes, because the gate never reads the
  criterion.
- G4 is satisfied by a dated record whose predictions were wrong, the same as by one whose predictions held.
- G13 adds no new proxy. It puts an expiry date on G12's list and on the survey's rulings. A board with G13 carries
  the same assurance as one without it, with one fewer way to go stale unnoticed.
- More REACHED gates read like more assurance. Each one says a proxy held, not that the family is correct.
- Revisit when a cell in the "does not prove" column gets a mechanical check, for example a G3 that reads each
  feature's criterion, or a coverage tool that reports which values an assertion read.
- Revisit if escaped defects keep arriving through the same row of the table. That row's proxy then needs a gate of
  its own.
