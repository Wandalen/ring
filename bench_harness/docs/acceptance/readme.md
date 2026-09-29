# acceptance

The binary reached-test each of the 22 graded features is measured against — what
must be observably true for the feature to count as delivered, and which test claims it.

Filed here rather than beside each feature's own definition, because those definitions
share one section schema across hundreds of instances, and adding an acceptance section
there would re-schema all of them to serve a handful. Gate `g3_features.sh` is what reads
a feature citation out of a `ring_*` test and decides whether the feature is claimed, so
the criteria live beside it.

| File | Responsibility |
|------|-----------------|
| `001_feature_reached_tests.md` | The 22 features, their owning crates, and each one's binary reached-test |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
files=$( ls bench_harness/docs/acceptance/ | grep -v '^readme.md$' )
echo '  -- acceptance/ holds exactly the one instance this table names --'
printf '%s\n' "$files"
```

Live output:

```
  -- acceptance/ holds exactly the one instance this table names --
001_feature_reached_tests.md
```
