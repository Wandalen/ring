# API: The Predicate Is the Parameter

### Scope

- **Purpose**: Explain why the reachable half of this crate's surface takes a closure, what `FnMut` permits that `Fn` would not, and what the three production call sites actually put in it.
- **Responsibility**: State the bound, show each real closure, and account for what the crate deliberately does not know about the thing being waited for.
- **In Scope**: `F : FnMut() -> bool` and the three closures that satisfy it in production.
- **Out of Scope**: The loop that evaluates it — see [`algorithm/001`](../algorithm/001_one_loop_and_the_two_ways_out.md).

### The Bound

```rust
// ring_wait/src/lib.rs:179-182
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
where
  F : FnMut() -> bool,
```

Three parameters, and the third carries everything the crate does not know:
which cursor, which threshold, whether this is a producer waiting for space or a
consumer waiting for data. The module documentation says so at `:20-22` — all of
that is the caller's, supplied as a closure.

`FnMut` rather than `Fn` is the load-bearing choice. `Fn` would be enough for
every predicate that only *reads* — `pair.may_claim()`, `shutdown.is_closed()`,
`self.admits( from, count )` are all `Fn` — and it would have rejected the one
production closure that matters:

```rust
// ring_shutdown/src/lib.rs:612-621
let mut closed = false;
let outcome = ring_wait::wait_until( kind, spins, ||
{
  if shutdown.is_closed()
  {
    closed = true;
    return true;
  }
  pair.may_claim()
} );
```

`closed = true` is a write to a captured local. Under `Fn` this does not
compile, and `for_space_or_close` — which must report *which* of its two exits
was taken — has no way to tell the caller. `Wake::Closed` and `Wake::Ready`
(`ring_shutdown/src/lib.rs:627-628`) are both `Ok` from `wait_until`'s
point of view, so the distinction can only leave the closure by mutation.

The crate's own test suite leans on the same permission throughout: every
`looks += 1` counter in `tests/wait_test.rs` — at `:138`, `:169`, `:185`, `:199`,
`:215`, `:231`, `:245` — is a captured `mut` that `Fn` would forbid. Seven of
the file's twenty-four tests count looks, and none of them could.

### The Three Real Predicates

| Site | Closure | Captures | Reads |
|------|---------|----------|-------|
| `ring_barrier:285` | `\|\| self.admits( from, count )` | `&self`, two `Copy` values | *n* cursors, one `Acquire` load each |
| `ring_shutdown:578` | `\|\| shutdown.is_closed()` | `&Shutdown` | one `AtomicBool` |
| `ring_shutdown:613` | two conditions, one mutation | `&Shutdown`, `&CursorPair`, `&mut bool` | one bool, then two cursors |

Note what varies and what does not. The *cost* of one evaluation differs by an
order of magnitude across the three — `is_closed` is one load, `admits` is one
load per dependency — and `wait_until` charges the same budget for each. A
budget is a count of looks, not of work, and the crate has no way to know what a
look costs ([`non_functional_requirement/001`](../non_functional_requirement/001_two_atomic_loads_for_every_look.md)).

### What the Closure Is Not Allowed to Be

| Not permitted | Why |
|---------------|-----|
| `FnOnce` | it is called up to `spins` times |
| Returning `Result` | a predicate that can fail would need a third exit, and the loop has two values |
| Taking the attempt index | `pause` gets it; `ready` does not, so a predicate cannot back off |
| Blocking | nothing prevents it, and nothing catches it — see below |

The last row is the real gap. `wait_until` is bounded in *attempts* and says so
(`:41-46`), but a closure that itself blocks makes the enclosing bound
meaningless: `wait_until( WaitKind::None, 1, || { thread::sleep( a_second );
false } )` looks exactly once, honours `None`, and takes a second. The
non-blocking guarantee `WaitKind::None` provides is over the crate's own pause,
not over the caller's predicate, and nothing in the type system says so.

That is why `ring_poll`'s guard is placed on the *dependency graph* rather than
on the wait: it cannot check what a predicate does, so it checks that the crate
which could park is not reachable at all
([`integration/001`](../integration/001_two_dependencies_two_dependents_and_a_roster.md)).

### The Cost of Generic Over Concrete

`wait_until` is generic in `F`, so it monomorphises once per distinct closure
type. Three production call sites means three instantiations, each inlining a
different predicate into the same thirteen-line loop. The alternative —
`&mut dyn FnMut() -> bool` — would give one instantiation and an indirect call
per look, on the hottest line in the crate.

`for_space` and `for_data` are the concrete half, and they are not generic:
their predicate is a closure the crate itself writes, so their signatures name
`&CursorPair` instead of a type parameter. That is what makes them one line and
also what makes them unextendable
([`algorithm/002`](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md) § WT3).

| | `wait_until` / `wait` | `for_space` / `for_data` |
|--|----------------------|--------------------------|
| Predicate | a parameter | fixed by the body |
| Generic | `F : FnMut() -> bool` | not generic |
| Knows about `ring_cursor` | no | yes — it is in the signature |
| Production callers | 3 | 0 |

The two generic functions do not name `ring_cursor` at all. Removing
`for_space` and `for_data` would make the whole `ring_cursor` dependency
unused — which is exactly what W6 checks, and what its own note records: before
those two existed the dependency was declared and unused.

### APIs

| File | Relationship |
|------|--------------|
| [001_seven_items_and_the_one_with_a_caller.md](001_seven_items_and_the_one_with_a_caller.md) | WT1 — which of these signatures is reached |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | Where `ready()` is called from |
| [../algorithm/002_two_wrappers_over_a_predicate_they_fix.md](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md) | The concrete half, and what fixing it cost |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | Why the ban is on the edge rather than on the predicate |
| [../integration/002_the_wrapper_that_had_to_be_rewritten.md](../integration/002_the_wrapper_that_had_to_be_rewritten.md) | The one closure that needs `FnMut` |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_none_looks_exactly_once.md](../invariant/002_none_looks_exactly_once.md) | The guarantee that stops at the closure boundary |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_two_atomic_loads_for_every_look.md](../non_functional_requirement/001_two_atomic_loads_for_every_look.md) | What one evaluation costs, per call site |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_predicate_the_pause_and_the_budget.md](../pattern/001_the_predicate_the_pause_and_the_budget.md) | The predicate as one of three separated concerns |

### Sources

| File | Relationship |
|------|--------------|
| `ring_shutdown/src/lib.rs:612-621,627-628` | The one production closure that mutates |
| `ring_barrier/src/lib.rs:285` | The predicate whose cost scales with dependency count |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:128-144` | A counting closure — `FnMut`, not `Fn` |
| `tests/wait_test.rs:179-191` | Another, asserting the returned attempt count |
| `tests/manual/readme.md` § W6 | Every declared dependency is used — `ring_cursor` only through the two concrete wrappers |
