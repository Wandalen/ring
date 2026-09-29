# Invariant Doc Definition

### Scope

- **Purpose**: State the two restrictions this crate must hold: that a configuration fully determines the ring, and that construction is the only way to get one.
- **Responsibility**: For each, state the invariant, the enforcement mechanism, and the consequences of violation.
- **In Scope**: Determinism of construction; the single construction path.
- **Out of Scope**: Invariants of the constructed ring, which belong to the backend crates.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Configuration Fully Determines the Ring](001_configuration_fully_determines_the_ring.md) | Two builds from equal configs must be indistinguishable — the property the benchmark sweep rests on | 🔄 |
| 002 | [Construction Is the Only Path](002_construction_is_the_only_path.md) | The restriction this crate exists to impose, and the four ways it leaks | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/invariant
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FC[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FC[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC21 | `ring_factory` | **misleading doc** | The enforcement mechanism is structural — `Factory` has no fields, so a second input cannot exist — and the mechanism this instance names is a grep that would pass on a stateful factory |
| FC22 | `ring_factory` | **latent hazard** | The determinism test passes for a reason that would survive the invariant being false: it compares observable behaviour, and a built ring retains only two of the five fields, so three could differ undetected |
| FC23 | `ring_factory` | n/a — inconsistency | Nine public ways to obtain a ring exist family-wide and three of them are this crate's; the pattern documenting the restriction still says five |
| FC24 | `ring_factory` | n/a — observation | The export surface confines crates, not types, so every leak it permits is a type living inside a confined crate — `Registry`'s eight methods being the worked example |
