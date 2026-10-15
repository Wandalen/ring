# invariant

One instance — the seeded defect: the per-definition floor this corpus holds
every declared definition to is two, and this directory carries only one.
`algorithm/` carries a compensating third instance in this fixture so the
corpus' own aggregate totals stay unchanged.

Nothing here is a finding about anything. The rows below exist so the tables
that restate them have something to disagree with.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_first.md) | Three | The first half |

## What This Holds

One instance, one finding, and one regenerate block that prints the count of
each so a reader can check the sentence above against the tree.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/bench_harness/gate/declared/ring/corpus_control/g14_thin_instances
printf '    instances: %s   findings: %s
' \
  "$( ls docs/invariant/0*.md 2>/dev/null | wc -l )" \
  "$( command grep -rh '^### CT[0-9]* — ' docs/invariant/ 2>/dev/null | wc -l )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CT3 | `fixture` | n/a — observation | The first instance says what it says, and the tree agrees |
