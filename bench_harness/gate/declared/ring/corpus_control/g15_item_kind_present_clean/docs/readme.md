# docs

Control fixture for G15. Not a crate — no `src/`, no manifest, and nothing
here describes real behaviour. The `_clean` counterpart to
`g15_missing_item_kind_readme/`: the same `item/<kind>/` shape, but with its
readme.md present and correct — proving G15 does not also flag the case it
must accept.

| Directory | Responsibility |
|------|-----------------|
| `item/` | One kind (`struct/`), populated, readme.md present and correct |

Scope of this fixture: zero defects — the must-pass counterpart.
