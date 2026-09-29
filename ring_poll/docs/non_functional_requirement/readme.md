# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: What a tick-path helper is allowed to cost, and what evidence exists that it costs it.
- **Responsibility**: The per-attempt cost of each helper, the one allocation on the path, the single automated timing assertion, and the family's benchmarking arrangement.
- **In Scope**: Cost per attempt, the batch inner loop, `Vec` growth in `drain_up_to`, the 500 ms bound, and `ring_bench`'s reach.
- **Out of Scope**: The parking prohibition itself (→ [`../invariant/001`](../invariant/001_no_parking_operation_on_the_tick_path.md)); what a caller should conclude about latency (→ [`../pitfall/001`](../pitfall/001_non_parking_is_not_bounded_latency.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [What A Tick May Cost](001_what_a_tick_may_cost.md) | The cost model: what one attempt buys per helper, and the one call that allocates | 🔄 |
| 002 | [The Cost Nobody Has Measured](002_the_cost_nobody_has_measured.md) | The single measured figure, its 62× margin, and the benchmark crate that skips this one | 🔄 |

**The claim, then the evidence for it.** `001` is what the crate promises about
cost and what the source actually does; `002` is what has ever been put on a
clock. They separate because they go stale on different events — `001` on a
change to a loop or a signature, `002` on a change to the suite, the manual plan,
or `ring_bench`'s manifest — and because a cost model with no measurement behind
it is a different problem from a cost model that is wrong.

The four findings run in a chain. The budget's unit is not constant across the
three helpers that take one (PL33); the one operation that can allocate is on the path built to
bound cost (PL34); the only automated check has sixty-two times more margin than
a regression would need to hide in (PL35); and the crate that could measure any
of it does not depend on this one (PL36). Each is small. Together they say the
cost claim is reasoned rather than observed.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/non_functional_requirement
printf 'instances:                  %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:    %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:    %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'helpers taking a Budget:    %s\n' "$( command grep -c 'while attempt < budget.attempts()' ../../src/lib.rs || true )"
printf 'helpers bounded otherwise:  %s\n' "$( command grep -c 'while taken < max' ../../src/lib.rs || true )"
printf 'parking calls on the path:  %s\n' "$( command grep -cE 'thread::sleep|yield_now\(|::park\(' ../../src/lib.rs || true )"
printf 'Box/String the crate owns:  %s\n' "$( command grep -cE 'Box::|String::' ../../src/lib.rs || true )"
printf 'Vec::new in the doctest:    %s\n' "$( command grep -c 'Vec::new()' ../../src/lib.rs || true )"
printf 'timing assertions in suite: %s\n' "$( command grep -c 'Duration::from' ../../tests/poll_test.rs || true )"
printf 'measured figures on record: %s\n' "$( command grep -coE '\*\*Result \(2026' ../../tests/manual/readme.md )"
printf 'benches dirs in the family: %s\n' "$( cd ../../.. && for d in ring_*/benches; do [ -d "$d" ] && echo x; done | wc -l )"
printf 'ring_bench reaching here:   %s\n' "$( command grep -c 'ring_poll' ../../../ring_bench/Cargo.toml || true )"
```

Live output:

```
instances:                  2
finding headings inside:    4
rows in the table below:    4
each instance has a recipe: 2
helpers taking a Budget:    3
helpers bounded otherwise:  1
parking calls on the path:  0
Box/String the crate owns:  0
Vec::new in the doctest:    1
timing assertions in suite: 1
measured figures on record: 4
benches dirs in the family: 0
ring_bench reaching here:   0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL33 | the budget's unit is not the same size across the helpers it bounds | **latent hazard** | All three budget-taking helpers loop on `while attempt < budget.attempts()`, but one attempt is a single `try_push`/`try_recv` for `push_within` and `recv_within` and a whole `try_push_batch` for `push_batch_within` — whose inner `for record in records.by_ref()` takes no limit from the budget and ends only when the ring refuses — so `Budget::once()`, the documented conservative default, buys one ring operation from one helper and up to the ring's free space from another; `Tick::drain`'s doc makes precisely this rounds-versus-work argument for the drain limit and nothing makes it for the batch. |
| PL34 | the only unbounded allocation sits on the path built to bound cost | n/a — observation | `drain_up_to` writes into a caller-owned `Vec` via `out.push( record )`, which reallocates and copies at each growth point — the one operation in the crate that is neither a ring operation nor bounded by any parameter — while the crate allocates nothing of its own, `with_capacity` appears nowhere in `src` or `tests`, and the published doctest opens `let mut out = Vec::new();`, so the worked example a caller copies starts at capacity zero. |
| PL35 | the only automated cost check cannot detect a cost regression | n/a — coverage | `a_large_budget_spins_rather_than_sleeping` bounds 20 000 attempts at 500 ms and the manual probe P3 measured the real cost at ≤ 8 ms — 62× under the bound, 125× under the parking cost it exists to exclude — a margin correct for separating spin from park and wide enough that a tick path five times slower passes it, along with every behavioural test in the suite, leaving *does not park* structurally enforced and *is fast* asserted nowhere. |
| PL36 | the family has a benchmarking crate and no benchmarks | n/a — coverage | `ring_bench` exports `Workload`, `Candidate`, `Outcome`, `Comparison` and `run` and ships the family's only runnable performance artifact, `examples/comparison.rs`, against zero `benches/` directories, zero `[[bench]]` targets and zero criterion dependencies across all thirty-three crates — and its nine dependencies are all backend crates, so the harness that could price `Budget`'s attempts is one manifest line from `ring_poll` and has never been pointed at any tick-path helper. |
