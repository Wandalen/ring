# verb

Crate-scoped test/lint/build for `ring_cursor` — thin wrappers over the workspace's
`verb/_crate_dispatch` (see [../../verb/readme.md](../../verb/readme.md) for the full
parameter convention and rationale). Every parameter is `key::val`, never `--flag val`.

| File | Responsibility |
|------|-----------------|
| `test` | Full check for this crate — nextest, doctests, clippy |
| `test_only` | Filtered nextest run for this crate — `filter::<substring>` |
| `lint` | Clippy for this crate |
| `build` | Compile this crate |
