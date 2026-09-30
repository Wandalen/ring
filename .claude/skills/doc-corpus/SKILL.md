---
name: doc-corpus
description: Add or change an instance, finding, recipe or cross-reference in a ring crate's docs/ corpus so G14–G17, G20 and G21 stay green. Use whenever a crate's behaviour changes or its docs/ is edited.
argument-hint: "[crate] [definition, e.g. pitfall]"
---

# Doc corpus

The standard is the gates (`bench_harness/gate/corpus/*.py`),
`bench_harness/gate/declared/ring/corpus_standard.txt`, and the minimal fixtures in
`bench_harness/gate/declared/ring/corpus_control/*_clean/`. The rulebooks the docs cite are not
in this repository. Copy the shape of a neighbouring instance in the same crate.

## Layout (G14)

- `docs/` holds exactly the 13 definitions plus `definition/`. Never add a directory.
- Floors per crate: 26 instances, 2 per definition, 52 findings.
- Instance: `docs/<def>/NNN_snake_case.md`, next free number, `# <Type>: <Title>`, then
  `### Scope` (Purpose, Responsibility, In Scope, Out of Scope).

## A finding — three places, identical text

```markdown
### XX7 — The title

**Finding.** What is true, and why it matters.

**Disposition:** applied — what changed. Now prints: `literal from a Live output below`
```

1. The heading in the instance: crate prefix (`command grep -rhoE '^### [A-Z]{2}[0-9]+' <crate>/docs | sort -u`),
   next number, em dash, exactly `###`.
2. A row in `docs/<def>/readme.md` under `### Findings Recorded Here`:
   `| XX7 | Subject | Tier | Finding |`.
3. A row in `docs/definition/readme.md` under `## Findings`:
   `| XX7 | Finding | Subject | Tier | [def/NNN](../def/NNN_x.md) |`. Subject and Tier equal
   row 2 character for character; the link's directory is the definition of row 2.

Tier is one of `corpus_standard.txt`'s 13 strings, bold included. A **bold** tier needs exactly
one `**Disposition:** applied — … Now prints: \`…\`` (the literal must appear in a `Live output`
block in the same file) or `declined — …` (≥ 40 characters naming a `` `token` `` or a path).
Also add the instance to the Module Index `## Master Doc Instances Table` and the definition
readme's overview table, and update any counts line that is not computed by a recipe.

## Recipes (G15, G21)

````markdown
```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F 'pub fn try_push' ring_spsc/src/lib.rs
```

Live output:

```
  pub fn try_push(&mut self, record: T) -> Result<(), T> {
```
````

- Fence exactly ```` ```sh ````; `bash`-tagged blocks are never run. `command grep`, not
  `grep`. The output fence is bare; one blank line after `Live output:`.
- Paste real output from running the recipe, byte for byte. Never type it.
- Address content, not line numbers: `sed -n '/^pub fn x/,/^}/p'`, `grep -m1 -A3 -F '…'`.
  `sed -n 'N,Mp'` and `awk 'NR==…'` fail G21.
- Every definition readme needs at least one `sh` block that exits 0 (`### Regenerate`).

## Links and tests (G16)

Relative links must resolve (`../../../ring_x/docs/...` across crates). A `### Tests` table
names test functions that exist in the crate's `tests/`.

## Check

`./verb/gate gate::g14 gate::g16 gate::g17 gate::g20 gate::g21` is fast; `gate::g15` runs
every recipe and is slower. The corpus was already red on `master` (2026-09-30: G15 quotes
pre-rustfmt source) — compare the touched crate's problems against `master`, and refresh the
`Live output` of recipes in any instance you edit.
