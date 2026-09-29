# docs

Control fixture for the corpus gates. Not a crate — no `src/`, no manifest, and
nothing here describes real behaviour. It exists so that G14-G17 have something
they are known to be able to fail against.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | Two branches that do nothing |
| `definition/` | Module Index — every definition and instance in this fixture |
| `invariant/` | Two properties, only one of which is recorded as a finding |

Scope of this fixture: one seeded defect — `invariant/002_second.md` exists as
a genuine instance (its recipe reproduces, its test is cited) but carries no
`### <ID> — ` finding heading, so `finding_owners()` counts only 3 findings
against the declared `min_findings` floor of 4. Both definitions are present,
both clear the per-definition instance floor of two, and every finding that
does exist has a matching definition-readme row and Module Index row — so the
only problem G14 may report here is the total finding count, isolated from
every other assertion this gate makes.
