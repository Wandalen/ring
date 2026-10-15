# docs

Control fixture for the corpus gates. Not a crate — no `src/`, no manifest, and
nothing here describes real behaviour. It exists so that G14-G17 have something
they are known to be able to fail against.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | Two branches that do nothing, plus a third that names an orphan finding |
| `definition/` | Module Index — every definition and instance in this fixture |
| `invariant/` | Two properties that hold trivially |

Scope of this fixture: one seeded defect — `algorithm/003_orphan.md` carries a
`### CT5 — ` heading that `finding_owners()` counts, but neither
`algorithm/readme.md`'s "Findings Recorded Here" table nor `definition/readme.md`'s
"Findings" table names CT5 anywhere. Every other count in the tree (definitions,
instances, the four CT1-CT4 findings) is untouched and clean, so this fixture
proves G14's cross-count-agreement assertion specifically — "the third assertion
... the one that does the work" per `g14_corpus_shape.sh`'s own header comment —
rather than the definition-presence check the other two G14 fixtures already
cover.
