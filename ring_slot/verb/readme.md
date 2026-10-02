# verb

Crate-scoped test/lint/build for `ring_slot`. Each script is a thin wrapper over the
workspace's `verb/_crate_dispatch` (see [../../verb/readme.md](../../verb/readme.md) for
the full parameter convention and rationale). Every parameter is `key::val`, never
`--flag val`.

| File | Responsibility |
|------|-----------------|
| `test` | Full check for this crate: nextest, doctests, clippy |
| `test_only` | Nextest run for this crate, filtered by `filter::<substring>` |
| `lint` | Clippy for this crate |
| `build` | Compile this crate |
