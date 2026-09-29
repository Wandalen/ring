# docs

Control fixture for G15. Not a crate — no `src/`, no manifest, and nothing
here describes real behaviour. Seeds the one defect #793 taught G15 to see:
an `item/<kind>/` directory holding an instance but no readme.md catalog.

| Directory | Responsibility |
|------|-----------------|
| `item/` | One kind (`struct/`), populated, missing its own readme.md |

Scope of this fixture: one seeded defect, named by the directory it sits in.
