# ring_wait

Wait strategies for space and data availability.

Part of the `ring` family; [../readme.md](../readme.md) describes the whole.

The crate delivers the four wait strategies over one loop, `wait_until`.
`ring_types::WaitKind` holds the four discriminants and this crate holds the
four handlers, so a `WaitKind` can travel through a config into a struct field
without dragging a thread-parking implementation behind it.
[`tests/wait_test.rs`](tests/wait_test.rs) asserts the behaviour, and it is read
by hand against [`tests/manual/readme.md`](tests/manual/readme.md). Every line
is covered.

`None` is the variant the rest exist around. It evaluates the predicate once and
returns, because a tick has a deadline and a strategy that might block has
already missed it. Every other wait is bounded by an explicit budget rather than
a deadline, so the same call takes the same number of samples on every machine.

The crate declares no type of its own. What it holds is policy: when to stop
asking, and what to do between asks.

## Decisions

- [`WaitKind::Park` sleeps 50 µs per attempt instead of parking the thread](docs/decisions/001_park_sleeps_on_a_timer_instead_of_parking.md)

## Known limitations

- `ring_wait::wait_until` pauses after every failed look, including the last
  one, so a failing call pays one pause after its answer is already known. For
  `ring_wait::for_space(pair, WaitKind::Park, 1)` that is a sleep of about
  115 µs for nothing. A budget of `spins = 0` still makes one look and one
  pause, although the summary says "at most `spins` attempts", so under
  `WaitKind::Park` a budget meant as "do not wait" sleeps before returning
  `RingError::Empty`. `ring_poll` pauses only while
  `attempt < budget.attempts()`.

## Run it

```sh
cargo nextest run -p ring_wait --all-features
cargo test --doc -p ring_wait --all-features
```

| File | Responsibility |
|------|-----------------|
| `docs/decisions/` | Architecture decision records |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `wait_test.rs` and the manual plan under `manual/` |
