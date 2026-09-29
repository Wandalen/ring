# docs

Control fixture for G15. Not a crate — no `src/`, no manifest, and nothing
here describes real behaviour. Seeds the defect #910 taught G15 to see: a
crate-level `docs/readme.md` whose own recipe exits non-zero. Before #910,
`check()` never opened this file at all, so no exit code here was ever
checked — this is the `docs/readme.md` sibling of `g15_stale_quoted_output/`.

Scope of this fixture: one seeded defect, named by the file it sits in.

```sh
exit 3
```
