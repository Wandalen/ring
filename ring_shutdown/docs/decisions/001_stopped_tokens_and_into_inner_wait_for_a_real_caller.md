# `Shutdown::close` keeps minting any number of `Stopped` tokens, and `Guarded::into_inner` stays, until a real caller decides both

Status: Deferred

## Context

`ring_shutdown::Shutdown` owns the family's one close flag. `Shutdown::close(&self)` sets it and returns a
`Stopped<'_>`, the token that `drain_all` and `discard_all` are methods on. `Stopped::reopen(self)` clears the flag
and consumes the token. `Shutdown::guard` wraps a `ring_core::Producer` in a `Guarded` that checks the flag before
every push.

Two questions ask what a caller can still reach after the types were meant to stop it.

The first is token supply. `close` can be called any number of times, and `Stopped::shutdown` returns the
`&Shutdown`, so `stopped.shutdown().close()` mints a second token from a shared borrow of the first. `reopen`
consumes only the token it is called on, so another token can outlive it on a reopened ring. The test
`close_is_idempotent_and_admit_reports_it` mints two tokens and reopens with one.

The second is the exit. `Guarded::into_inner` returns the raw producer, which publishes into a closed ring, as
`an_unguarded_producer_publishes_straight_through_a_close` asserts.

The two are decided together. Removing `into_inner` while `Guarded::shutdown` remains leaves the guard as escapable
as before. Making tokens unique while `into_inner` remains leaves the producer as unguarded as before.

`close` takes `&self` for reasons outside this question. `guard` stores `&'a Shutdown` in every `Guarded`.
`wait_for_close` and `for_space_or_close` take `&Shutdown` and are meant to run on a thread other than the closer.
`close` is idempotent because teardown is often reached from more than one path, such as the normal end of a run and
a panic unwinding through a guard. `into_inner` exists because a helper with a `Producer` parameter cannot take a
`Guarded`, which has no `Deref`, trait or conversion. Its use in the test above is circular and does not count.

## Decision

Keep both as they are until a caller outside this crate shows which way to change them. `close(&self)` stays
idempotent and returns a fresh token on every call. `Guarded::into_inner` stays. The `Stopped` doc says what does hold.
`reopen` consumes the token it is called on, and a token from an earlier `close` outlives it.

No crate outside `ring_shutdown` holds a `Guarded`. Outside this crate only `ring_testkit` binds a `Stopped`, and it
binds exactly one per scope.

## Alternatives considered

- **`close(&mut self)`.** The borrow checker would enforce one token. It breaks `guard`, `wait_for_close` and
  `for_space_or_close`, which all hold `&Shutdown`, and that shared flag is the crate's concurrency model.
- **`close(&self) -> Option<Stopped>`, `None` once closed.** It kills the idempotence `close` relies on. A second
  teardown path gets `None` and cannot drain, which is the case idempotence exists for.
- **A generation stamp.** `Shutdown` carries a counter, `reopen` bumps it, and drains check it. That turns a
  compile-time proof into a runtime check, adds a word of state and a load per drain, and needs a ruling on what a
  stale drain returns.
- **`drain_all` returning `Err` on an open ring.** It moves the failure from compile time to run time, and needs an
  error meaning "you did not close first" that nothing else would use.
- **A `Drop` on `Stopped` that reopens.** Dropping a token would reopen a ring the caller meant to leave closed, which
  is the common case.
- **Remove `into_inner`.** A helper that takes a `Producer` can then never be called from a guarded context. Every such
  helper, including ones outside the family, would have to take `&mut Guarded` instead.
- **Re-check on unwrap, `into_inner(self) -> Result<Producer, Self>`, refusing while closed.** A caller who unwraps
  before the close still holds a raw producer afterwards, and that is the path that fails.
- **A scoped `with_raw(&mut self, f)`.** The raw producer cannot escape the closure, so a helper that stores the
  producer cannot be written.

## Consequences

- A `Stopped` proves the ring was closed at some point, not that it is closed now. Closing twice, reopening once
  and draining on the surviving token compiles, and against a live producer that drain does not end.
- `into_inner` gives up the close guarantee in one call with no ceremony.
- Removing `into_inner` would not make `Guarded` one-way. `Guarded::shutdown` returns `&'a Shutdown` from `&self` in a
  `const fn`, so a guard holder can call `close` and reach `drain_all`, `discard_all` and `reopen` while still holding
  the guard. That accessor stays, because `wait_for_close(guarded.shutdown(), ..)` is the intended composition.
- The trigger first named for `into_inner`, `ring_factory` handing out guarded producers, passed without firing.
  `ring_factory` exists, does not depend on `ring_shutdown`, and hands out no `Guarded`.
- Revisit the token when two tokens are legitimately live at once. If every close is followed by its own reopen or drop
  in the same scope, `Option<Stopped>` costs nothing real. If two live tokens in one scope are both used, a generation
  stamp is the only option that keeps both callers working. If closes come from separate threads, neither borrow-based
  option is available, and the question becomes what a stale drain returns.
- Revisit `into_inner` once some crate hands out `Guarded`, by counting non-test `Guarded::into_inner` call sites.
  With none, remove it and build the test's raw producer from `split()` directly. If the sites use the producer and
  drop it, or re-wrap it, replace `into_inner` with `with_raw`. If they store it, keep it and document each site as a
  place the close guarantee does not hold.

See [`src/lib.rs`](../../src/lib.rs).
