# Doc Definitions

Module Index for `bench_harness`. It lists every doc definition this crate declares and
every instance under each, in one place.

## Master Doc Definitions Table

| Type | Purpose | Master File | Instances |
|------|---------|-------------|----------:|
| `acceptance/` | The binary reached-test each graded feature is measured against | [acceptance/readme.md](../acceptance/readme.md) | 1 |
| `guide/` | How to reproduce the family's verdicts, what they establish, and what they miss | [guide/readme.md](../guide/readme.md) | 3 |
| `invariant/` | Measurable constraints the validation machinery must hold | [invariant/readme.md](../invariant/readme.md) | 1 |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions | [workaround/readme.md](../workaround/readme.md) | 0 |

## Master Doc Instances Table

| Type | Instance | Summary |
|------|----------|---------|
| `acceptance/` | [001_feature_reached_tests.md](../acceptance/001_feature_reached_tests.md) | The 22 features, their owning crates, and each one's binary reached-test |
| `guide/` | [001_running_the_verdicts_yourself.md](../guide/001_running_the_verdicts_yourself.md) | Every command from the gates to the comparison demo, with expected output |
| `guide/` | [002_the_four_verdicts.md](../guide/002_the_four_verdicts.md) | What the comparison established: three deterministic, one coarse-only |
| `guide/` | [003_what_the_gates_do_not_prove.md](../guide/003_what_the_gates_do_not_prove.md) | 100% coverage with zero defence against the crate's headline defect |
| `invariant/` | [001_gate_non_vacuity.md](../invariant/001_gate_non_vacuity.md) | Every gate reports 0/6 while the family is unimplemented |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- instance count per definition, against the Master Doc Definitions Table above --'
for d in acceptance guide invariant workaround; do
  n=$( ls bench_harness/docs/$d/ 2>/dev/null | grep -vc '^readme.md$' )
  printf '%-11s %s\n' "$d/" "$n"
done
echo '  -- rows in the Master Doc Instances Table below --'
awk '/Master Doc Instances Table/,0' bench_harness/docs/definition/readme.md | command grep -c '^| `'
```

Live output:

```
  -- instance count per definition, against the Master Doc Definitions Table above --
acceptance/ 1
guide/      3
invariant/  1
workaround/ 0
  -- rows in the Master Doc Instances Table below --
5
```
