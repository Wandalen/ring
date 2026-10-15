# docs

Control fixture for the corpus gates. Not a crate — no `src/`, no manifest, and
nothing here describes real behaviour. It exists so that G14-G17 have something
they are known to be able to fail against.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | Three branches that do nothing |
| `definition/` | Module Index — every definition and instance in this fixture |
| `invariant/` | One property that holds trivially |

Scope of this fixture: one seeded defect — `invariant/` carries only one
instance against the per-definition floor of two, while `algorithm/` carries a
compensating third so the *aggregate* corpus totals (2 definitions, 4
instances, 4 findings) are unchanged from `g14_definition_present_clean` and
clear every floor on their own. That is deliberate: `shape.py`'s own comment
names the exact hazard this proves G14 catches — "a crate can clear the family
floor of 26 with thirteen instances in one directory and none in the other
twelve, which is the same count and not the same corpus." Nothing else in the
tree is broken — every finding still has a matching definition-readme row and
Module Index row — so the only problem G14 may report here is the
per-definition instance floor.
