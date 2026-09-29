# docs

Control fixture for G15. Not a crate — no `src/`, no manifest, and nothing
here describes real behaviour. The `_clean` counterpart to
`g15_crate_readme_recipe_fails/`: the crate-level readme's own recipe is
present and exits 0 — proving G15 does not also flag the case it must accept.

Scope of this fixture: zero defects — the must-pass counterpart.

```sh
echo ok
```
