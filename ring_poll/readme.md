# ring_poll

Non-blocking progress helpers.

Depends on [`ring_core`](../ring_core/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../README.md`](../README.md).

**This crate exists to keep a claim about a crate it deliberately does not
depend on.** The claim is that parking operations must not be *reachable* from
inside a system's step — and half of that was already true, because
`ring_core::Producer` has no parking operation on it. The parking lives in
`ring_wait`, one crate over. So the work here is the other half:

- **`ring_wait` is not a dependency, at any depth** — not even to use the two of
  its four `WaitKind` variants that are tick-safe, because taking the dependency
  would put the other two one autocomplete away.
- **The roster of crates permitted to park is public** — `PARKING_CRATES`, three
  names — and the suite asserts it against the manifests on disk. Adding
  `ring_wait` to `ring_handle` fails *here*, which matters because it would not
  fail in `ring_handle`.
- **The helpers exist so the rule stays cheap to obey.** A restriction survives
  exactly as long as following it is easier than working around it.

**The limitation to know before using it:** non-parking is not the same as
prompt. `Budget::new( 1_000_000 )` never deadlocks and will still drop the
frame. `Budget::once()` is the default for that reason, and
[`docs/pitfall/001`](docs/pitfall/001_non_parking_is_not_bounded_latency.md)
states the trap rather than implying it away.

```sh
cargo nextest run -p ring_poll        # 22 tests
cargo test --doc -p ring_poll         # 8 doc tests
cargo tree -p ring_poll | grep -c ring_wait   # 0 — the point of the crate
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build — see [verb/readme.md](verb/readme.md) |
| `docs/` | 11 doc instances across 9 definitions — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | The budget, the progress verdict, four bounded helpers, and the tick that accumulates them |
| `tests/poll_test.rs` | This crate's reached-test and 21 tests across the surface |
| `tests/manual/readme.md` | The four readings and measurements automation cannot make |
