# `WaitKind::Park` sleeps 50 µs per attempt instead of parking the thread

Status: Accepted

## Context

`ring_types::WaitKind` holds four discriminants, and `ring_wait::pause` holds their handlers. Callers pick `Park` to
stay idle between looks instead of burning a core.

A real `std::thread::park` is half of a rendezvous. The waiter parks, and the publisher calls `unpark` on the waiter's
`Thread` handle. So the publisher must hold that handle, which means some crate must register waiters against a ring,
decide which ones to wake on a publish, and clean up when a thread exits. In this family `ring_handle` owns the
knowledge of who holds a handle to which ring.

`ring_wait` cannot reach that knowledge. It depends only on `ring_types` and `ring_cursor`, takes a `&CursorPair` and a
closure, and declares no type of its own. `ring_barrier` and `ring_shutdown` depend on it. `ring_poll`, the tick-path
helper crate, asserts that its own manifest names none of `ring_poll::PARKING_CRATES`, and `ring_wait` is one of them.

## Decision

The `Park` arm of `ring_wait::pause` calls `std::thread::sleep(Duration::from_micros(50))` and returns `true`. That
gives the cost profile callers choose `Park` for, idle rather than spinning, without a registration relationship. No
publisher wakes the waiter. It wakes on the clock and looks again. The `//` comment on the arm records why it does not
call `thread::park`, and the `WaitKind::Park` doc in `ring_types` states only the cost profile ("Idle between reads; no
publisher wakes it").

The sleep also keeps a layering mistake cheap. A tick-path crate that gains a stray edge to `ring_wait` today pays one
sleep per attempt, which blows a frame. With a real park the same mistake is a thread nobody unparks, which hangs, and
the `PARKING_CRATES` check would be all that stands between a stray edge and a hang.

## Alternatives considered

- **A waiter registry inside `ring_wait`.** It gives a leaf crate with no type of its own a type, a lifetime and a
  teardown, beside the knowledge `ring_handle` already owns.
- **A registry in `ring_handle`, depended on from `ring_wait`.** `ring_handle` depends on `ring_core`, which depends on
  `ring_spsc`, `ring_mpsc`, `ring_slot` and more. The wait strategy would sit downstream of the rings it waits on, and
  `ring_barrier` and `ring_shutdown` would inherit that whole closure.
- **Rename the variant to `Sleep`.** `WaitKind` belongs to `ring_types`, travels through `RingConfig` as a config value,
  and `ring_wait`'s tests assert its discriminant order as part of what a config means. A rename breaks every crate and
  config that names `Park`.
- **Delete the variant.** It breaks every match over the four variants.

## Consequences

- Wall-clock cost depends on the `WaitKind` argument alone. A release-build probe that ran `wait_until` against an
  always-false predicate (three runs, one development machine) measured `Spin` at 55 to 77 ns per attempt, `Yield` at
  about 520 ns and `Park` at 112 to 119 µs. A failing `wait(WaitKind::Park, ..)` at `DEFAULT_SPINS` (1024) therefore
  takes about 0.12 s, while the signature names only an attempt count.
- `Park` asks for 50 µs and gets about 115 µs, because of scheduler granularity. Nothing asserts the 50 µs constant.
  `ring_handle`'s test `an_empty_ring_answers_promptly` bounds 10,000 empty drains at 500 ms, which is exactly
  10,000 x 50 µs, so it tells a parking `try_recv` from a prompt one only because of the overshoot. Lowering the sleep
  would silently defeat that test. `ring_poll`'s test `a_large_budget_spins_rather_than_sleeping` keeps a 2x margin on
  the same arithmetic.
- The 50 µs is a literal inside the match arm. No parameter reaches it, so a caller cannot tune it.
- Revisit when `ring_handle` gains an API to register a waiter against a ring and signal it on publish. `Park` then
  waits on that signal and the 50 µs sleep goes. The change should add a latency test that publishes from one thread
  while another waits under `Park` and bounds the wait well below 50 µs.
- The same trigger reopens the split that keeps the discriminants in `ring_types`. A real park needs per-waiter state
  that a one-byte `Copy` `WaitKind` cannot carry, and `ring_config::RingConfig::with_wait` is a `const fn` only because
  `WaitKind` is a plain discriminant.
