# Decision: `free_capacity` Keeps One Signature Over Two Contracts

**Status:** open, and weaker than it looks. The decision is defensible and the
evidence for its necessity is thin: four callers, one reading, no observed need
for the other. Reopening it would mean asking whether the SPSC guarantee is
worth a documented hazard nobody has hit.

### Scope

- **Purpose**: Record why `Producer::free_capacity` was left as one method meaning two different things, what the alternatives cost, and what the four calling crates actually do with it.
- **Responsibility**: The contract split, the three alternatives and their costs, and the measured caller behaviour.
- **In Scope**: `free_capacity`, `is_full`, and the four dependent crates that call them.
- **Out of Scope**: The hazard statement itself (→ [`../pitfall/001`](../pitfall/001_free_capacity_carries_two_contracts.md)); the discrimination surface that would let a caller tell (→ [`001`](001_the_backend_discrimination_surface_has_no_caller.md)).

### The Split

At SPSC a reported `n` is binding — nothing else can take the room. At MPSC and
crossbeam another producer may take it between the read and the push. One
signature, `fn free_capacity( &self ) -> usize`, and no way to tell from the type
which reading applies.

### The Alternatives, and What Each Costs

| Alternative | Cost |
|-------------|------|
| Two methods (`free_capacity_binding`, `free_capacity_advisory`) | The uniform surface breaks — a caller swapping backends now edits call sites, which is exactly the property `non_functional_requirement/001` exists to protect |
| Return a wrapper type carrying the reading | Every caller unwraps; the four production callers all want a `usize` for a comparison |
| Make it advisory everywhere by contract | Free, and loses a real SPSC guarantee that `ring_spsc` does provide and that a caller could exploit |
| Keep one signature, document the asymmetry | Chosen. The hazard is real and moved into prose |

### What the Four Callers Actually Do

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r '\.free_capacity()\|\.is_full()' ring_*/src/*.rs | grep -v '^ring_core/' \
  | grep -vE ': *(//|///|//!)' | sed 's/\.rs:[0-9]*:/.rs:/; s/:  */:/'
```

Live output:

```
ring_debug/src/lib.rs:let free = producer.free_capacity();
ring_flush/src/lib.rs:FlushPolicy::OnFull => if self.buffer.is_full() { Some( FlushCause::Full ) } else { None },
ring_flush/src/lib.rs:if self.producer.free_capacity() < staged
ring_handle/src/lib.rs:self.inner.free_capacity()
ring_handle/src/lib.rs:self.inner.is_full()
ring_shutdown/src/lib.rs:self.producer.free_capacity()
ring_shutdown/src/lib.rs:self.shutdown.is_closed() || self.producer.is_full()
ring_spsc/src/lib.rs:if self.is_full()
```

**Every one of them uses the advisory reading**, which the module documentation
says *"is always correct"*. Not one branches on backend first. So the chosen
option's cost has not yet been paid by anybody — and its remedy, the
discrimination surface, has no caller either
(→ [`001`](001_the_backend_discrimination_surface_has_no_caller.md)).

### CO16 — All Four Callers Take the Advisory Reading

The four production call sites — `ring_handle:160`, `ring_shutdown:462`,
`ring_flush:629`, `ring_debug:523` — all use the number as a hint and let
`try_push` be the authority, which the method's own rustdoc says "is always
correct".

That is the safe reading, and every one of them arrived at it without consulting
`backend()`. Either the hazard is well enough documented that callers avoid it,
or the advisory reading is simply the natural one to write. The measurement
cannot distinguish those, and the difference decides whether the SPSC guarantee
is worth keeping.

### CO17 — `is_full` Inherited the Asymmetry Silently, and Is Branched On

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
command grep -m1 -B9 -A2 -F '  pub fn is_full( &self ) -> bool' src/lib.rs
```

Live output:

```

  /// Whether the ring has no room, by the same reading as
  /// [`free_capacity`](Self::free_capacity).
  ///
  /// **Binding at SPSC, advisory at MPSC and crossbeam** — it inherits that
  /// asymmetry rather than resolving it, so at MPSC a reported `true` can be
  /// false by the time the caller branches on it. Let
  /// [`try_push`](Self::try_push) be the authority, which is always correct.
  #[ must_use ]
  pub fn is_full( &self ) -> bool
  {
    self.free_capacity() == 0
```

One expression, so every property of `free_capacity` transfers. Its callers use
it as a control-flow guard rather than as a hint:

- `ring_shutdown:478` — `self.shutdown.is_closed() || self.producer.is_full()`
- `ring_flush:604` — `FlushPolicy::OnFull if self.buffer.is_full()`

`ring_flush`'s is on its own buffer and safe. `ring_shutdown`'s is on a
`ring_core::Producer`, and at MPSC a `false` there can be stale by the time the
push it guards runs. The push still refuses correctly — the record comes back —
so the consequence is a misreported reason, not a lost record.

**What was found: the hazard was documented on `free_capacity` and not on
`is_full`**, and `is_full` is the one being branched on. "By the same reading as
`free_capacity`" was already there and was already true — it just delegated the
part a caller needs to a function the caller had no reason to open, which is the
weakest place to put a warning and the easiest place to think you have put one.

**Disposition:** applied — `is_full`'s rustdoc states the SPSC/MPSC asymmetry in its own words and names `try_push` as the authority, so the warning is at the surface being branched on rather than one hop away; the same sentence pair was added to `Consumer::len` and `Consumer::is_empty` under CO7, which leaves `backend` as the only occupancy-adjacent reading whose contract still has to be inferred. Now prints: `**Binding at SPSC, advisory at MPSC and crossbeam** — it inherits that`

### CO18 — The Decision Records No Date and No Revisit Trigger

A decision left open with no trigger is a decision that stays open. What would
settle this one is measurable: a dependent that branches on `backend()` before
reading `free_capacity` proves the discrimination need is real; `ring_mpsc`
reaching feature parity with `ring_spsc`'s binding guarantee removes the
asymmetry entirely and retires the question.

Neither is recorded as a trigger in either instance, so nothing prompts a
re-read. Recorded here rather than fixed silently, because choosing the trigger
is itself part of the decision.
