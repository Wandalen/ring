# docs

Control fixture for the corpus gates. Not a crate — no `src/`, no manifest, and
nothing here describes real behaviour. It exists so that G14-G17 have something
they are known to be able to fail against.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | Two branches that do nothing |
| `definition/` | Module Index — every definition and instance in this fixture |
| `invariant/` | Two properties that hold trivially |

Scope of this fixture: no seeded defect — every Tier agrees between each
definition readme and the Module Index, so G16 has a correct case to accept,
not only a disagreement to refuse.
