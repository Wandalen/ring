# algorithm

Two instances, four lines of prose, and a regenerate block that counts them.
The shape is the family's, at a fifth of the size.

Nothing here is a finding about anything. The rows below exist so the tables
that restate them have something to disagree with.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_first.md) | One | The first half |
| [002](002_second.md) | Two | The second half |

## What This Holds

Two instances, two findings, and one regenerate block that prints the count of
each so a reader can check the sentence above against the tree.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/bench_harness/gate/declared/ring/corpus_control/g14_definition_present_clean
printf '    instances: %s   findings: %s
' \
  "$( ls docs/algorithm/0*.md 2>/dev/null | wc -l )" \
  "$( command grep -rh '^### CT[0-9]* — ' docs/algorithm/ 2>/dev/null | wc -l )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CT1 | `fixture` | n/a — observation | The first instance says what it says, and the tree agrees |
| CT2 | `fixture` | **latent hazard** | The second instance says what it says, and the tree agrees |
