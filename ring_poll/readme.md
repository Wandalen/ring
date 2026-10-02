# ring_poll

Non-blocking progress helpers.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

**This crate exists to keep a claim about a crate it deliberately does not
depend on.** The claim is that parking operations must not be *reachable* from
inside a system's step. Half of that was already true, because
`ring_core::Producer` has no parking operation on it. The parking lives in
`ring_wait`, one crate over. So the work here is the other half:

- **`ring_wait` is not a dependency, at any depth.** Not even to wait with a
  tick-safe `WaitKind`, because taking the dependency would put the parking
  kinds one autocomplete away.
- **The roster of crates permitted to park is public.** `PARKING_CRATES` names
  them, and the suite asserts it against the manifests on disk. Adding
  `ring_wait` to `ring_handle` fails *here*, which matters because it would not
  fail in `ring_handle`.
- **The helpers exist so the rule stays cheap to obey.** A restriction survives
  exactly as long as following it is easier than working around it.

**Know this limitation before using it.** Non-parking is not the same as
prompt. `Budget::new( 1_000_000 )` never deadlocks and will still drop the
frame. `Budget::once()` is the tick-path default for that reason.

## Decisions

- [`PARKING_CRATES` stays a hand-written list that a test diffs against the manifests, not a generated one](docs/decisions/001_parking_roster_is_written_by_hand.md)

## Run it

```sh
cargo nextest run -p ring_poll --all-features
cargo test --doc -p ring_poll --all-features
cargo tree -p ring_poll | grep -c ring_wait   # prints 0, the point of the crate
```

| File | Responsibility |
|------|-----------------|
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | The budget, the progress verdict, the bounded helpers, and the tick that accumulates them |
| `tests/poll_test.rs` | The check that no tick-path crate reaches `ring_wait`, and tests across the public API |
| `tests/manual/readme.md` | The readings and measurements automation cannot make |
