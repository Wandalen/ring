# Pitfall Doc Definition

### Scope

- **Purpose**: Record the two traps this crate's position sets — an acceptance criterion that grades a value the caller may never have written, and a config field naming a crate that is not in the closure.
- **Responsibility**: For each, state the scope, the trap, the failure, and the mitigation.
- **In Scope**: Silent clamping upstream; `WaitKind`'s unreachable implementation.
- **Out of Scope**: Traps inside a constructed ring, which belong to the backend crates.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Criterion Grades the Clamped Value](001_the_criterion_grades_the_clamped_value.md) | `with_batch( 999 )` on a 16-slot ring is 16 before `build` runs, so "behaviour matches every field" is satisfied by construction | 🔄 |
| 002 | [A Wait Strategy It Can Read and Cannot Honour](002_a_wait_strategy_it_can_read_and_cannot_honour.md) | `cfg.wait()` compiles because the enum is in `ring_types`; `ring_wait` is not in the closure | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/pitfall
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FC[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FC[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC41 | `ring_factory` | n/a — observation | The two clamped fields are exactly the two nothing reads, so the pitfall is dormant rather than absent — it arms itself the moment `wait` or `batch` acquires a consumer |
| FC42 | `ring_factory` | n/a — unadopted | There is a legality check on `RingConfig` — `is_tick_safe` — that no build path calls; every one of its callers is a test of itself |
| FC43 | `ring_factory` | n/a — observation | The implementing crate is not in the closure, measured rather than asserted: `ring_wait` is absent from all nineteen crates this crate reaches |
| FC44 | `ring_factory` | **latent hazard** | The unhonourable field is on the export surface via `ring_types` and the crate that would honour it is not — five public `ring_wait` functions take a `WaitKind` no consumer of the Contract can reach them with |
