# workaround

External constraints `bench_harness` absorbs on behalf of its consumers, each with the
cost it imposes and the condition under which it can be deleted.

| File | Responsibility |
|------|-----------------|

No workarounds recorded. The gates shell out to `cargo` and `cargo tarpaulin`
rather than linking them, which is a deliberate design choice rather than an
absorbed constraint: the machinery must run against a workspace that does not
yet compile, so it cannot depend on the workspace building.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
n=$( ls bench_harness/docs/workaround/ 2>/dev/null | grep -vc '^readme.md$' )
echo '  -- workaround/ instance count, matching "No workarounds recorded" above --'
printf '%s\n' "$n"
```

Live output:

```
  -- workaround/ instance count, matching "No workarounds recorded" above --
0
```
