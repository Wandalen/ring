# Algorithm: From A Step To An Outcome

### Scope

- **Purpose**: Describe what `Script::run` does, step by step, and where each counter moves.
- **Responsibility**: The setup, the ten step behaviours, and the teardown reading.
- **In Scope**: `Script::run`.
- **Out of Scope**: Why the record type is `u32` (→ [`api/001`](../api/001_the_script_surface.md)); why `Flush` is per-record (→ [`pitfall/003`](../pitfall/003_the_amortised_flush_has_no_ring.md)).

### Abstract

`Script::run` turns a list of ten possible steps into one `Outcome`. It is
deterministic and single-threaded: no spinning, no timing, no thread spawn, so a
script that ran once produces the same counters every time it runs again.

The whole procedure is three phases — build the ends and the guard, interpret each
step in order while moving counters, then take one final reading of what is left
in the ring — and the accounting law
(→ [`invariant/001`](../invariant/001_every_minted_record_is_somewhere.md)) is
what the counters exist to make checkable.

### Algorithm

#### Setup

```text
ends    := ring.ends()
p, c    := ends.split()
shutdown:= Shutdown::new()
guard   := shutdown.guard( p )
staging := TlsBuffer::with_capacity( script.stage_limit )
minted, accepted, refused_full, refused_closed, refused_staging := 0
received := []
```

**Every push in the run goes through `guard`, never through the raw producer.**
That is what makes `Step::Close` mean anything: `ring_shutdown` describes the
wrapper as "the difference between a rule and a guarantee", and a fixture that
kept the raw producer around could publish into a closed ring by accident.

The shutdown starts **open**, so a script with no `Close` behaves exactly as
though there were no shutdown at all — the guard costs one flag read per push
and changes no outcome.

#### The steps

| Step | Behaviour | Counters |
|---|---|---|
| `Push` | Mint one record, offer it to the guard | `minted`; then one of `accepted` / `refused_full` / `refused_closed` |
| `PushMany( n )` | The above, `n` times, in mint order | same, `n` times |
| `Recv` | Take one record from the consumer | `received` grows by 0 or 1 |
| `RecvMany( n )` | Take up to `n`, **stopping at the first empty read** | `received` grows by 0..=n |
| `Stage` | Mint one record into the staging buffer | `minted`; `refused_staging` if the buffer is full |
| `StageMany( n )` | The above, `n` times, **not stopping** on a full buffer | `minted` × n; `refused_staging` per refusal |
| `Flush` | Drain the buffer, offer each record to the guard in turn | `accepted` / `refused_full` / `refused_closed` per record |
| `Close` | Close the shutdown, discard the token | none |
| `Reopen` | Close, then reopen — see [`pitfall/002`](../pitfall/002_reopening_closes_first.md) | none |
| `DrainAll` | Close, then take every record still in the ring | `received` grows; the ring is left closed |

**Two asymmetries are deliberate.** `RecvMany` stops at the first empty read,
so its count is a ceiling — spinning would turn a fixture step into a busy-wait
whose result depends on nothing the script controls. `StageMany` does *not*
stop, so its count is exact: a full buffer refuses each remaining record
individually, and `refused_staging` records how many. One step is bounded by
what is available, the other by what was asked.

#### `Flush` in detail

```text
staged := staging.drain().collect()      # collect first: drain borrows the buffer
for record in staged:
    match guard.try_push( record ):
        Ok        -> accepted += 1
        Full( _ ) -> refused_full += 1
        Closed(_) -> refused_closed += 1
```

The collect is not an optimisation and not a style choice — `drain()` holds a
mutable borrow of `staging` for the iterator's life, and `guard.try_push` cannot
run inside it. The buffer is emptied whether or not the ring takes anything,
which mirrors `TlsBuffer::flush_into`'s own contract.

#### Teardown

```text
Outcome
{
  minted, accepted, refused_full, refused_closed, refused_staging, received,
  in_ring_at_end : consumer.len(),
  staged_at_end  : staging.len(),
  closed_at_end  : shutdown.is_closed(),
}
```

Three readings are taken **after** the last step rather than tracked during it,
because each is a fact about the final state that the ring and the buffer
already know. Tracking them incrementally would be a second copy of a number the
owning type maintains, free to drift.

### Complexity

Linear in the total number of records the steps name — `PushMany( n )` is `n`
pushes, `Flush` is one push per staged record. No step is quadratic and none
retries, so a script's cost is readable off the step list.

### Termination

Every step is bounded by a constant in the step itself or by the buffer's own
size. `RecvMany`'s break on an empty read is what makes it terminate on a ring
with fewer records than asked; `DrainAll` terminates because
`Stopped::drain_all` loops until a batch comes back empty, and publication has
stopped by then.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'Step variants declared:      %s\n' "$( awk '/^pub enum Step$/{f=1} f && /^  [A-Z]/{n++} f && /^}$/{exit} END{print n}' src/lib.rs )"
printf 'match arms in run:           %s\n' "$( awk '/pub fn run/{f=1} f && /^        Step::/{n++} END{print n}' src/lib.rs )"
printf 'variants named across them:  %s\n' "$( awk '/pub fn run/{f=1} f && /^        Step::/' src/lib.rs | command grep -o 'Step::[A-Za-z]*' | sort -u | wc -l )"
printf 'the suppression reason:      %s\n' "$( command grep -m1 -o 'reason = "[^"]*"' src/lib.rs )"
printf 'arms that call close():      %s\n' "$( awk '/pub fn run/{f=1} f && /^        Step::/,0' src/lib.rs | command grep -c 'shutdown.close()' )"
```

Live output:

```
Step variants declared:      10
match arms in run:           7
variants named across them:  10
the suppression reason:      reason = "seven arms cover ten variants, three collapsed into their Many form; the length comes from the three Many loops and the Flush body, not a one-arm-per-variant shape"
arms that call close():      3
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_script_surface.md](../api/001_the_script_surface.md) | The signatures this describes the body of |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_minted_record_is_somewhere.md](../invariant/001_every_minted_record_is_somewhere.md) | The accounting each counter movement preserves |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/001_the_states_a_script_moves_through.md](../lifecycle/001_the_states_a_script_moves_through.md) | The open/closed states `Close`, `Reopen` and `DrainAll` move between |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/003_the_amortised_flush_has_no_ring.md](../pitfall/003_the_amortised_flush_has_no_ring.md) | Why `Flush` is a loop rather than one claim |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `Script::run` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | `the_single_record_steps_match_a_many_of_one`, `a_receive_step_stops_at_the_first_empty_read` |

### TK1 — the suppression reason counts arms that are not there

`Script::run` carries
`#[ allow( clippy::too_many_lines, reason = "one match arm per Step; splitting it would hide the shape of the enum" ) ]`.
The enum has **ten** variants and the match has **seven** arms: `Push` shares one
with `PushMany`, `Recv` with `RecvMany`, and `Stage` with `StageMany`, each
collapsing the singular into the plural by rebinding `count` to `1`.

That collapse is the right implementation — it is why
`the_single_record_steps_match_a_many_of_one` can assert the two forms agree
rather than hoping they do — but it is the opposite of what the reason claims.
The stated justification for the length is a one-to-one correspondence with the
enum that the body deliberately does not have, so a reader auditing the
suppression finds the shape argument contradicted by the first three arms.

The line count the suppression is really defending comes from the three `Many`
loops and the `Flush` body, none of which the reason mentions.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
command grep -F 'seven arms cover ten variants, three collapsed into their Many form' src/lib.rs
```

Live output:

```
  #[ allow( clippy::too_many_lines, reason = "seven arms cover ten variants, three collapsed into their Many form; the length comes from the three Many loops and the Flush body, not a one-arm-per-variant shape" ) ]
```

**Disposition:** applied — the suppression reason now states the true split
(seven arms, ten variants, three collapsed) and attributes the length to the
three `Many` loops and the `Flush` body instead of a one-to-one correspondence
the match does not have.
Now prints: `seven arms cover ten variants, three collapsed into their Many form`

### TK2 — one return value, three treatments, one match

`Shutdown::close` returns a `Stopped` token, and the three shutdown steps do
three different things with it inside the same `match`:

| Step | Body | The token |
|---|---|---|
| `Close` | `{ shutdown.close(); }` | produced and dropped |
| `Reopen` | `shutdown.close().reopen()` | produced and immediately consumed |
| `DrainAll` | binds it, then `drain_all` | produced and used |

Three adjacent arms, three readings of one type. The middle one is the pitfall
[`pitfall/002`](../pitfall/002_reopening_closes_first.md) records; the first is
the one with no document here at all.

Dropping the token is sanctioned by the crate that produces it: `Stopped` carries
only `#[ derive( Debug ) ]` — no `#[ must_use ]`, and `ring_shutdown` declares no
`Drop` impl anywhere — and `Shutdown::close`'s own doc example writes
`shutdown.close();` with the comment `// no complaint`. So the `Close` arm is
correct, and the reason it is correct lives in another crate's doctest.

What is left is a legibility cost paid inside one `match`: a reader who learns
from `DrainAll` that the return value is the family's proof-of-closure, then
reads `Close` two arms above it, sees the same call written as though it returned
`()`. Nothing in this crate's step enum, its doc comments, or the arm itself
distinguishes the deliberate discard from an oversight.
