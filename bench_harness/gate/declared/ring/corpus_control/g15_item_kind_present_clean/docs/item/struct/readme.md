# Struct Items

One placeholder instance, so G15's `item/<kind>/` readme-check has a correct
case to accept, not only a missing one to refuse.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_first.md) | Placeholder | The fixture's one instance |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/bench_harness/gate/declared/ring/corpus_control/g15_item_kind_present_clean
printf '    instances: %s\n' "$( ls docs/item/struct/0*.md 2>/dev/null | wc -l )"
```

Live output:

```
    instances: 1
```
