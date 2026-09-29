# invariant

Two instances, four lines of prose, and a regenerate block that counts them.
The shape is the family's, at a fifth of the size.

Nothing here is a finding about anything. The rows below exist so the tables
that restate them have something to disagree with.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_first.md) | Three | The first half |
| [002](002_second.md) | Four | The second half |

## What This Holds

Two instances, two findings, and one regenerate block that prints the count of
each so a reader can check the sentence above against the tree.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/bench_harness/gate/declared/ring/corpus_control/g16_tier_disagreement
printf '    instances: %s   findings: %s
' \
  "$( ls docs/invariant/0*.md 2>/dev/null | wc -l )" \
  "$( command grep -rh '^### CT[0-9]* — ' docs/invariant/ 2>/dev/null | wc -l )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CT3 | `fixture` | n/a — observation | The first instance says what it says, and the tree agrees |
| CT4 | `fixture` | **latent hazard** | The second instance says what it says, and the tree agrees |
