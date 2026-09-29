# plan

Staged validation plans whose stages are graded by this crate's gates. A plan
belongs here rather than in the corpus' own `docs/plan/` when its stage table is
read by `gate/` scripts on every run — the declaration and the thing that reads
it stay in one place.

| File | Responsibility |
|------|-----------------|
| [001_ring_doc_corpus_staged.md](001_ring_doc_corpus_staged.md) | 15 stages bringing 33 `ring_*` crates to 13 definitions / 26 instances / 52 findings |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
files=$( ls bench_harness/docs/plan/ | grep -v '^readme.md$' )
echo '  -- plan/ holds one instance, and its Stage Table row count matches "15 stages" above --'
printf '%s\n' "$files"
awk '/^## Stage Table/,/^## Rulings/' bench_harness/docs/plan/001_ring_doc_corpus_staged.md \
  | command grep -cE '^\| (M[0-9]+|S[0-9]+) \|'
```

Live output:

```
  -- plan/ holds one instance, and its Stage Table row count matches "15 stages" above --
001_ring_doc_corpus_staged.md
15
```
