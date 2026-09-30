---
name: decision
description: Record an open trade-off or a ruling as a decision instance in a ring crate's docs/decisions/, in the corpus format the gates check.
disable-model-invocation: true
argument-hint: "<crate> <question being decided>"
---

# Decision

A decision instance is for a genuine trade-off with a real switching cost. A settled fact goes
where it takes effect (`algorithm/`, `invariant/`, `workaround/`), not here.

## Instance

`<crate>/docs/decisions/NNN_snake_case_title.md`, next free number. Shape, after
`ring_debug/docs/decisions/001_four_edges_not_two.md`:

```markdown
# Decision: <Title>

**Status:** open | accepted, YYYY-MM-DD | superseded by [NNN](NNN_x.md). <One sentence: what it rules.>

### Scope

- **Purpose**: …
- **Responsibility**: …
- **In Scope**: …
- **Out of Scope**: … (→ [`algorithm/001`](../algorithm/001_x.md))

### Context
The forces, measured where possible — a recipe with `Live output:` beats an argument.

### Options
| Option | Consequence |
|---|---|

### Decision
What is chosen, and why each rejected option lost.

### Consequences
What gets easier, harder, and what would reverse it.

### Sources
| File | Relationship |
|---|---|

### Tests
| Test | Relationship |
|---|---|
```

An open decision says what would settle it instead of `### Decision`.

## Register it

1. `docs/decisions/readme.md`: a row in its index table; `### Open` / `### Closed` entries if
   the crate keeps that register.
2. `docs/definition/readme.md`: a row in `## Master Doc Instances Table`.
3. Findings raised by the decision: follow the `doc-corpus` skill (three places, contiguous id,
   disposition for bold tiers).
4. Where the code enforces it, a one-line comment pointing at `docs/decisions/NNN`.

Superseding: the old instance changes only its `**Status:**` line.

## Check

`./verb/gate gate::g14 gate::g16 gate::g17 gate::g20 gate::g21`, then `gate::g15` for its recipes.
