# Integration: The Wrapper That Had to Be Rewritten

### Scope

- **Purpose**: Read the two `ring_shutdown` functions that are this crate's only multi-call consumer, and say what each one had to supply that `ring_wait` could not.
- **Responsibility**: Show both bodies, name what each preserves and discards, and derive the general shape from the two concrete cases.
- **In Scope**: `wait_for_close` and `for_space_or_close`, as consumers.
- **Out of Scope**: The wrappers they bypassed — see [`algorithm/002`](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md).

### The Thin One

```rust
// ring_shutdown/src/lib.rs:575-579
pub fn wait_for_close( shutdown : &Shutdown, kind : WaitKind, spins : usize )
-> Result< usize, RingError >
{
  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
}
```

This is what a consumer of `wait_until` looks like when nothing goes wrong. It
forwards `kind` and `spins` unchanged, supplies a one-condition predicate,
returns the `Result` untouched — including the attempt count — and documents
the error as *"`ring_wait`'s own budget-exhausted error, unchanged"* at `:563`.

There is no `for_close` in this crate to have used instead, and there should not
be: `Shutdown` is `ring_shutdown`'s type, and a named wrapper for it here would
invert the dependency. `wait_until` is doing exactly the job it exists for.

### The One That Could Not Be a Wrapper

```rust
// ring_shutdown/src/lib.rs:611-631
{
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

  match outcome
  {
    // The close is reported even when room also appeared: a producer told to
    // stop must stop, and a `Ready` here would send it back to publish.
    Ok( _ ) if closed => Ok( Wake::Closed ),
    Ok( _ ) => Ok( Wake::Ready ),
    Err( _ ) => Err( RingError::Full ),
  }
}
```

Its own documentation says what it is: *"This is `ring_wait::for_space` with a
second exit"* (`:434`). Line for line, that is true — `pair.may_claim()` is
`for_space`'s predicate and `Err( _ ) => Err( RingError::Full )` is
`for_space`'s `map_err`. Both are re-typed rather than reused.

Three things it needed that `for_space` cannot give:

| Needed | Why `for_space` cannot | How it is supplied |
|--------|-----------------------|--------------------|
| A second exit condition | its predicate is fixed by the body | a two-branch closure |
| To know *which* exit fired | its return type is `Result< usize, _ >` | a captured `mut bool` |
| A three-valued success | same | `Wake::Ready` / `Wake::Closed` |

The second row is the interesting one. `wait_until` returns `Ok` for *ready*
without saying why, and both branches of this closure return `true`. So the
distinction has to leave the closure by mutation, which is why `FnMut` rather
than `Fn` is load-bearing at the boundary
([`api/002`](../api/002_the_predicate_is_the_parameter.md)).

`Wake` — a two-variant enum declared in `ring_shutdown` — is what the caller
actually wanted from the wait, and there is no shape of `Result< usize,
RingError >` that carries it. A generic loop that returns "ready or not" cannot
express "ready, for one of two reasons", and the crate correctly does not try.

### What Both Consumers Do With the Attempt Count

| Consumer | Returns `Ok( n )`? | What it does with `n` |
|----------|:------------------:|-----------------------|
| `wait_for_close` | yes | forwards it |
| `for_space_or_close` | no | `Ok( _ )` twice — discarded |
| `ring_barrier::Barrier::wait_for` | no | `wait_until( … )?;` — discarded by the `;` |

Two of the three production call sites throw the count away
([`item/002`](../item/002_the_loop_the_wrapper_and_the_two_questions.md) § WT4),
and both do so because they have something more useful to return in its place —
a `Wake` in one case, and in `ring_barrier`'s, the frontier the barrier went on
to read. The count survives only where the consumer had nothing else to say.

### The General Shape

Reading the two together gives the rule that
[`algorithm/002`](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md)
states as WT3, from the consumer's side rather than the library's:

> A consumer that wants exactly the question `ring_wait` names, and exactly the
> answer it returns, can use `wait_until` in one line. A consumer that wants
> either of those changed must write the loop's third argument itself — and once
> it is writing that, the named wrapper for the *first* condition saves it
> nothing.

`for_space` is not too small to be useful; it is too *finished*. Every parameter
it fixed is one a real consumer wanted back.

### What This Predicts

If a third dependent appears, the same reading says where it will land:

| A consumer that wants… | Will use |
|------------------------|----------|
| one ring question, default budget | `wait` — still unused |
| one ring question, own budget | `for_space` / `for_data` — still unused |
| one non-ring question | `wait_until`, like `wait_for_close` |
| a compound condition | `wait_until`, like `for_space_or_close` |
| to distinguish two successes | `wait_until` plus its own enum |

Four of the five rows go to `wait_until`, which is what
[`api/001`](../api/001_seven_items_and_the_one_with_a_caller.md) § WT1 already
measures at three of three.

### Integrations

| File | Relationship |
|------|--------------|
| [001_two_dependencies_two_dependents_and_a_roster.md](001_two_dependencies_two_dependents_and_a_roster.md) | The graph these two functions are one edge of |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seven_items_and_the_one_with_a_caller.md](../api/001_seven_items_and_the_one_with_a_caller.md) | WT1, which this instance is the mechanism behind |
| [../api/002_the_predicate_is_the_parameter.md](../api/002_the_predicate_is_the_parameter.md) | Why `FnMut`, demonstrated by the closure above |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_two_wrappers_over_a_predicate_they_fix.md](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md) | WT3, stated from the library's side |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_loop_the_wrapper_and_the_two_questions.md](../item/002_the_loop_the_wrapper_and_the_two_questions.md) | WT4 — who keeps the attempt count |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_reading_empty_as_nothing_to_do.md](../pitfall/001_reading_empty_as_nothing_to_do.md) | The `Full` this consumer reproduces by hand |

### Sources

| File | Relationship |
|------|--------------|
| `ring_shutdown/src/lib.rs:555-579` | `wait_for_close`, the thin consumer |
| `ring_shutdown/src/lib.rs:581-631` | `for_space_or_close`, the one that could not wrap |
| `ring_barrier/src/lib.rs:285` | The third call site, which discards the count |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:255-268` | The `Full`/`Empty` split `for_space_or_close` reproduces |
| `tests/wait_test.rs:304-324` | A blocking wait that succeeds when another thread publishes |
