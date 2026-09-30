# Pitfall: Reading `Empty` as "Nothing to Do"

### Scope

- **Purpose**: Record the misreading this crate's one `map_err` exists to prevent, show that the defence covers only one of the three exits, and follow what each consumer actually propagates.
- **Responsibility**: State the confusion, name the single line that guards it, and find the places the guard does not reach.
- **In Scope**: `RingError::Empty` and `RingError::Full` as produced by this crate.
- **Out of Scope**: The two questions as API — see [`item/002`](../item/002_the_loop_the_wrapper_and_the_two_questions.md).

### The Misreading

`wait_until`'s give-up value is `RingError::Empty` for every caller, including a
producer waiting for room. Handed that verbatim, a producer reads:

> **Empty** — nothing to do. Stop.

when what happened was:

> **Full** — back-pressure. The consumer has not kept up. Retry, or apply an
> overflow policy.

Those lead to opposite actions. The first stops a producer that should have
retried; worse, on a ring that is genuinely full, "nothing to do" is precisely
the wrong conclusion — the ring is as far from empty as it can be.

### The Whole Defence Is One `map_err`

```rust
// ring_wait/src/lib.rs:241
wait_until( kind, spins, || pair.may_claim() ).map_err( | _ | RingError::Full )
```

W2 in the manual plan checks that it is the only error-shaped thing in the file:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E "RingError::[A-Za-z]+"
```

Live output:

```
    Err(RingError::Empty)
    wait_until(kind, spins, || pair.may_claim()).map_err(|_| RingError::Full)
```

**Exactly two hits** — `Empty` as `wait_until`'s give-up value, `Full` in
`for_space`'s `map_err`. The check's own note explains why `for_data` has no
symmetrical `map_err`:

> `for_data` appears in neither: it wants `wait_until`'s error unchanged, and
> adding a `map_err( |_| RingError::Empty )` there for symmetry would be a line
> that says nothing and one more place to get it wrong.

That is right, and it is also why the defence is narrow: the correction lives in
the wrapper, not in the loop, so it protects only callers who went through the
wrapper.

### Who Is Not Protected

| Caller | Error it gets on exhaustion | Correct for it? |
|--------|-----------------------------|-----------------|
| `for_space` | `Full` | yes |
| `for_data` | `Empty` | yes |
| `wait_until` directly, consumer-side | `Empty` | yes, by luck |
| `wait_until` directly, **producer-side** | `Empty` | **no** |
| `wait` directly, producer-side | `Empty` | **no** |

Three of the seven public items hand a producer the wrong word, and the only two
that get it right are the two with no production caller
([`api/001`](../api/001_seven_items_and_the_one_with_a_caller.md) § WT1).

`ring_shutdown::for_space_or_close` is a real case: it is a producer-side wait
built on `wait_until`, and it has to write the correction out again by hand
(`ring_shutdown/src/lib.rs:627-629`):

```rust
Err( _ ) => Err( RingError::Full ),
```

Same rule, second copy, because `for_space` had fixed a different parameter it
needed back ([`integration/002`](../integration/002_the_wrapper_that_had_to_be_rewritten.md)).

### The Consumer That Keeps `Empty` on Purpose

`ring_barrier::Barrier::wait_for` propagates `Empty` unchanged and says so
(`ring_barrier/src/lib.rs:256-259`):

> [`RingError::Empty`] when the `spins` budget runs out with fewer than `count`
> available, which for a consumer means exactly what it says.

Correct — a barrier is read by consumers. But the function has a second source
of the same error one line later:

```rust
// ring_barrier/src/lib.rs:285-286
ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
self.frontier().ok_or( RingError::Empty )
```

| Source | Means |
|--------|-------|
| the `?` on line 285 | the budget ran out before `count` items were available |
| the `ok_or` on line 286 | there are no cursors at all — `frontier()` returned `None` |

An `Err( Empty )` from `wait_for` does not say which, and the two want different
responses: the first is retryable back-pressure, the second is a configuration
state that will not change by waiting. Neither is wrong to call `Empty`; together
they are a value with two meanings, and this crate contributed one of them.

### The Deeper Cause

Both misreadings trace to the same design fact, recorded in `wait_until`'s own
documentation (`:157-159`):

> The caller decides what that means: a consumer treats it as "nothing yet", a
> producer as "no room yet", and neither is this crate's business.

That is a defensible position — a generic loop genuinely cannot know which side
called it — but it is stated as if the *value* were neutral, and `Empty` is not a
neutral word. A neutral give-up value (`RingError::Exhausted`, say, or a
dedicated `WaitTimedOut`) would have made every caller name its own meaning
instead of two callers correcting a default.

The family has room for it: `RingError` already carries nine variants, two of
which this crate produces
([`type/002`](../type/002_one_return_type_and_the_one_must_use.md) § WT15).

### Neither Error Is a Configuration Error

The one thing the two do agree on is asserted (`tests/wait_test.rs:270-278`):

```rust
assert!( !RingError::Full.is_configuration() );
assert!( !RingError::Empty.is_configuration() );
```

with a comment naming the failure it prevents:

> Both are back-pressure, so both must be retryable. `RingError` splits
> configuration errors from transient ones, and a retry loop that could not tell
> them apart would spin forever on a bad capacity.

So a caller that only asks *"is this retryable?"* is safe under either word. The
pitfall is specific to a caller that branches on `Full` versus `Empty` — and a
producer deciding whether to apply an overflow policy is exactly that caller.


### WT46 — The Two Production Callers Disagree About What a Failed Wait Is Called

Both real consumers call `wait_until`. One renames its failure, the other
propagates it, for the same underlying condition.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'ring_wait::wait_until' --include=*.rs . --exclude-dir=docs \
  | grep -v '^ring_wait/'
grep 'Err( _ ) => Err( RingError::Full )' ring_shutdown/src/lib.rs
grep -A2 'ring_wait::wait_until( kind, spins, || self.admits' ring_barrier/src/lib.rs
```

Live output:

```
ring_shutdown/src/lib.rs:    // `budget.max( 1 )` matches `ring_wait::wait_until`'s reading of its own
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
    Err( _ ) => Err( RingError::Full ),
    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
    self.frontier().ok_or( RingError::Empty )
  }
```

`ring_shutdown::for_space_or_close` ends `Err( _ ) => Err( RingError::Full )`.
`ring_barrier::wait_for` uses `?`, so the same `RingError::Empty` travels out
unchanged — and its next line is `.ok_or( RingError::Empty )`, so both of that
function's failure exits are `Empty` too.

Each is right locally. A producer told "no room" wants `Full`; a barrier that has
not admitted a sequence is closer to "nothing yet". But the condition underneath
is identical in both — a budget ran out — and it reaches the caller as `Full` in
one crate and `Empty` in the other, with no way to tell either apart from the
non-budget condition of the same name.

`RingError::is_transient` matches both variants (WT15), so a caller retrying on
transience behaves correctly under either spelling. A caller that branches on the
specific variant — which is the reason the two variants exist rather than one —
gets a different answer depending on which consumer it went through.


### WT47 — The Producer-Side Rename Lives in the Function Nobody Calls

`for_space` exists to turn a producer's failed wait into the producer's word for
it. It is the crate's only place that does, and it has no callers.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A3 'pub fn for_space' ring_wait/src/lib.rs | grep 'map_err\|pub fn'
# every map_err in the crate
grep -c 'map_err' ring_wait/src/lib.rs
# who calls it
grep -r 'for_space(' --include=*.rs . --exclude-dir=docs \
  | grep -v '^ring_wait/' || echo '(nobody)'
# control: the identical expression for the entry point they used instead
grep -r 'wait_until(' --include=*.rs . --exclude-dir=docs | grep -v '^ring_wait/'
```

Live output:

```
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
  wait_until( kind, spins, || pair.may_claim() ).map_err( | _ | RingError::Full )
1
(nobody)
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
```

The body is `wait_until( kind, spins, || pair.may_claim() ).map_err( | _ | RingError::Full )`
— one `map_err`, the only one in the crate, and the whole of what `for_space` adds
over spelling the predicate inline.

Nothing calls it. So every producer in the family that waits does so through
`wait_until` directly and receives `RingError::Empty` for a ring with no room —
`ring_barrier` by propagation, and `ring_shutdown` by rewriting the mapping into
its own body rather than reaching this one (WT3, WT46).

This is what [`pitfall/001`](001_reading_empty_as_nothing_to_do.md) is about,
located exactly: the crate identified the confusion, wrote the one-line fix,
placed it behind a fixed predicate, and the fix went where the callers are not.
The word a producer sees is `Empty` in every case that occurs in the tree.

### Pitfalls

| File | Relationship |
|------|--------------|
| [002_the_backoff_that_resets_every_eight_attempts.md](002_the_backoff_that_resets_every_eight_attempts.md) | The other thing that is not what its shape suggests |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seven_items_and_the_one_with_a_caller.md](../api/001_seven_items_and_the_one_with_a_caller.md) | WT1 — the two protected items are the two nobody calls |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | Two exits, one value |
| [../algorithm/002_two_wrappers_over_a_predicate_they_fix.md](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md) | The `map_err` asymmetry between the two wrappers |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_wrapper_that_had_to_be_rewritten.md](../integration/002_the_wrapper_that_had_to_be_rewritten.md) | The consumer that writes the correction out a second time |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_loop_the_wrapper_and_the_two_questions.md](../item/002_the_loop_the_wrapper_and_the_two_questions.md) | The two questions and their opposite failures |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_one_return_type_and_the_one_must_use.md](../type/002_one_return_type_and_the_one_must_use.md) | WT15 — two of `RingError`'s nine variants |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:256-259,285-286` | `Empty` kept on purpose, and a second source of it one line later |
| `ring_shutdown/src/lib.rs:627-629` | The `map_err` written out by hand |
| `ring_types/src/error.rs:42-80` | All nine variants, of which this crate produces two |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:255-268` | The two waits fail with different errors |
| `tests/wait_test.rs:270-278` | Neither is a configuration error |
| `tests/manual/readme.md` § W2 | Exactly two `RingError::` mentions, and why `for_data` has none |
