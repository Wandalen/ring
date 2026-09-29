# algorithm

Three instances — one more than its sibling `invariant/` carries in this
fixture, which is the seeded defect — six lines of prose, and a regenerate
block that counts them.

Nothing here is a finding about anything. The rows below exist so the tables
that restate them have something to disagree with.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_first.md) | One | The first half |
| [002](002_second.md) | Two | The second half |
| [003](003_third.md) | Three | The compensating third |

## What This Holds

Three instances, three findings, and one regenerate block that prints the
count of each so a reader can check the sentence above against the tree.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/bench_harness/gate/declared/ring/corpus_control/g14_thin_instances
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
| CT5 | `fixture` | n/a — observation | The compensating third instance says what it says, and the tree agrees |
