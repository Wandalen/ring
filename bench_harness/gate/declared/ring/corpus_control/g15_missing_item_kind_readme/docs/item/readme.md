# Item Kinds

The crate's one item kind, for this fixture's purposes.

| Kind | Responsibility |
|------|-----------------|
| `struct/` | One placeholder instance |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/bench_harness/gate/declared/ring/corpus_control/g15_missing_item_kind_readme
printf '    kinds: %s\n' "$( ls -d docs/item/*/ 2>/dev/null | wc -l )"
```

Live output:

```
    kinds: 1
```
