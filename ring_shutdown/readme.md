# ring_shutdown

Publisher stop, drain, and waiter join.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

This crate holds the family's **only liveness flag**. Every other crate that
could plausibly carry an `is_closed` deliberately does not, because a handle
carrying its own copy of liveness is the failure this crate exists to prevent.
`ring_handle` records its side of that in
[its ADR](../ring_handle/docs/decisions/002_handles_have_no_is_closed.md).

**None of close, drain-all and reset is hard alone. The hard part is that
drain-all only terminates because close came first.** A drain loop against an
open ring with a live producer never ends. So the crate enforces that ordering
with types. `close()` returns a `Stopped` token, `drain_all` is a method on the
token, and `reopen` consumes it. A caller cannot write a drain that did not
follow a close.

**One rule is still convention, and it is the thing to know before using this
crate.** Nothing forces a producer to consult the flag. `Shutdown::guard` wraps
a producer so that it must, because a `Guarded` producer's only push checks
first. A caller holding a raw `ring_core::Producer`, though, publishes into a
closed ring without complaint. The gap is the price of the flag living here
rather than in `ring_core`.

## Decisions

- [`Shutdown::close` keeps minting any number of `Stopped` tokens, and `Guarded::into_inner` stays, until a real caller decides both](docs/decisions/001_stopped_tokens_and_into_inner_wait_for_a_real_caller.md)

## Known limitations

- The cost of the close check in `Guarded::try_push`, an `Acquire` load of the
  flag before every guarded push, has never been measured. `ring_bench` never
  drives a `Guarded` producer, and the family has no `benches/` directory or
  `criterion` dependency.

## Run it

```sh
cargo nextest run -p ring_shutdown --all-features
cargo test --doc -p ring_shutdown --all-features
```

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | The flag, the `Stopped` token, the guarded producer, and the close-aware waits |
| `tests/shutdown_test.rs` | An end-to-end close, drain and reset run, and tests across the public API |
| `tests/manual/readme.md` | The readings and measurements automation cannot make |
