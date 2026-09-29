# invariant

Measurable constraints the validation machinery must hold, each with a number,
a unit, and the method that measures it.

| File | Responsibility |
|------|-----------------|
| [001_gate_non_vacuity.md](001_gate_non_vacuity.md) | Every gate must fail on an unimplemented family, not pass vacuously |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
files=$( ls bench_harness/docs/invariant/ | grep -v '^readme.md$' )
echo '  -- invariant/ holds exactly the one instance this table names --'
printf '%s\n' "$files"
```

Live output:

```
  -- invariant/ holds exactly the one instance this table names --
001_gate_non_vacuity.md
```
