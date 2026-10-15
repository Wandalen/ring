# Doc Definitions

Every definition and every instance in this fixture, in one place. The tables
below restate what the definition readmes already say, in a different column
order, which is the arrangement G16 exists to hold consistent.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|-----------|
| `algorithm/` | Two branches that do nothing | [readme](../algorithm/readme.md) | 2 |
| `definition/` | This file | [readme](readme.md) | 0 |
| `invariant/` | Two properties that hold trivially | [readme](../invariant/readme.md) | 2 |

**2 definitions, 4 instances, 4 findings.** Regenerate all three counts:

```sh
cd "$(git rev-parse --show-toplevel)"/bench_harness/gate/declared/ring/corpus_control/g16_tier_agreement_clean
printf '    definitions: %s   instances: %s   findings: %s
' \
  "$( ls -d docs/*/ | command grep -vc '/definition/$' )" \
  "$( ls docs/*/0*.md 2>/dev/null | wc -l )" \
  "$( command grep -rh '^### CT[0-9]* — ' docs/ 2>/dev/null | wc -l )"
```

## Master Doc Instances Table

| Type | ID | Title | Carries |
|------|-----|-------|---------|
| `algorithm/` | 001 | [One](../algorithm/001_first.md) | The first half |
| `algorithm/` | 002 | [Two](../algorithm/002_second.md) | The second half |
| `invariant/` | 001 | [Three](../invariant/001_first.md) | The first half |
| `invariant/` | 002 | [Four](../invariant/002_second.md) | The second half |

## Findings

| ID | Finding | Subject | Reachable | Where |
|----|---------|---------|-----------|-------|
| CT1 | The recipe prints four and the sentence says four | `fixture` | n/a — observation | [algorithm/001](../algorithm/001_first.md) |
| CT2 | The recipe prints nine and the sentence says nine | `fixture` | **latent hazard** | [algorithm/002](../algorithm/002_second.md) |
| CT3 | The recipe prints four and the sentence says four | `fixture` | n/a — observation | [invariant/001](../invariant/001_first.md) |
| CT4 | The recipe prints nine and the sentence says nine | `fixture` | **latent hazard** | [invariant/002](../invariant/002_second.md) |
