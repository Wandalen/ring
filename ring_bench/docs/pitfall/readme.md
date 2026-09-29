# Pitfall Doc Definition

### Scope

- **Purpose**: Record the three traps a measurement crate sets that a data-structure crate cannot — a ceiling imposed by the door rather than the structure, a counter that changes what it counts, and a success return that is not evidence of success.
- **Responsibility**: For each, state the scope, the trap, the failure, and the mitigation.
- **In Scope**: Traps whose failure mode is a *wrong verdict* rather than a wrong record.
- **Out of Scope**: Traps inside the candidates themselves, which belong to the backend crates; whether `DropNewest` is the right default, which is `ring_types`' decision and already made.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Door Caps What the Structure Does Not](001_the_door_caps_what_the_structure_does_not.md) | Two multi-producer structures are single-producer through the Contract, and neither cap is theirs | 🔄 |
| 002 | [A Counter Inside the Timed Region Measures Itself](002_a_counter_inside_the_timed_region_measures_itself.md) | `ring_stats`'s counters would instrument the loop this crate is timing | 🔄 |
| 003 | [`Ok` Is Not Kept, and the Verdict Inverts](003_ok_is_not_kept_and_the_verdict_inverts.md) | Counting API successes ranked the candidate that threw the workload away first | 🔄 |

**The three share one shape and it is worth naming once here.** Each is a place
where a number is available, correct on its own terms, and about something other
than what the report says it is about. 001's producer count is a real property
of a real door; 002's per-record counter would be a real count of real records;
003's `Ok` is a truthful return value from a correctly-behaving ring. **None of
the three is a bug in anything being measured** — which is exactly why a
measurement crate has to document them, since no test in any other crate can go
red for them.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/pitfall
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### BN[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| BN[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN41 | C5, Mitigation 3, and `every_candidate_declares_a_name_and_a_ceiling` | **latent hazard** | Three sections make three non-agreeing statements about C5, and the test named as its guard pins the very value C5 would falsify |
| BN42 | `the_counters_are_the_runs_own_totals`, and Mitigation 3 | n/a — coverage | Mitigation 3 presents two omissions as deliberate because both are asserted; one of the two is `wait_nanos() == 0`, which nothing in the crate can make fail |
| BN43 | `run_mutex_queue` and `run_direct_mpsc`'s `reported` | **misleading doc** | The Mitigation section's second rejected attempt is per-producer counters summed afterwards, and that is exactly what `reported` is on both threaded runners |
| BN44 | `a_failing_policy_closes_the_gap_for_every_candidate` | n/a — coverage | The pair of tests is justified on the grounds that either alone would be consistent with the wrong behaviour, and "for every candidate" has one load-bearing iteration in the build the default `cargo test` compiles |
