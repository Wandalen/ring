# Algorithm: Deciding Which Exit a Wait Took

### Scope

- **Purpose**: Record how `for_space_or_close` turns one bounded spin over two conditions into a three-way answer, and what each of the three answers is actually a statement about.
- **Responsibility**: The predicate's evaluation order, the side channel that carries the reason out of it, the precedence rule in the match, and the temporal status of each outcome.
- **In Scope**: `for_space_or_close`, `Wake`, and `wait_for_close` where the two differ.
- **Out of Scope**: The drain loop, which is the crate's other procedure (→ [`001`](001_drain_to_empty.md)); which error each waiter reports on exhaustion (→ [`../item/002`](../item/002_two_checks_and_two_waiters.md)).

### The Procedure

`ring_wait::wait_until` takes one predicate and returns `Ok( attempt )` or
`Err`. A close-aware wait needs to distinguish *two* successes, so the reason is
carried out of the predicate by a captured flag rather than through the return
value:

```
1. closed ← false
2. outcome ← wait_until( kind, spins ):
3.     if shutdown.is_closed():
4.         closed ← true
5.         return true
6.     return pair.may_claim()
7. match outcome:
8.     Ok(_) if closed → Ok( Wake::Closed )
9.     Ok(_)           → Ok( Wake::Ready )
10.    Err(_)          → Err( RingError::Full )
```

Two ordering decisions are load-bearing, and only one of them is stated.

**Inside the predicate**, the close check runs first and short-circuits, so a
tick where the ring is both closed and has room never evaluates `may_claim`.

**In the match**, `closed` takes precedence over the successful outcome — and
that one carries a comment: *"The close is reported even when room also
appeared: a producer told to stop must stop, and a `Ready` here would send it
back to publish."*

### What Each Answer Claims

| Outcome | Claim | Still true when the caller reads it? |
|---|---|---|
| `Wake::Closed` | The ring closed | Yes — close is one-way until a `Stopped::reopen` |
| `Wake::Ready` | There was room, at some attempt | Not necessarily |
| `Err( Full )` | The budget ran out | It was true at exhaustion |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'the side channel it captures:  %s\n' "$( awk '/pub fn for_space_or_close/{f=1} f&&/^\}$/{exit} f&&/let mut/{ sub( /^ */, "" ); print }' ring_shutdown/src/lib.rs )"
printf 'the predicate checks first:    %s\n' "$( awk '/pub fn for_space_or_close/{f=1} f&&/^\}$/{exit} f&&/is_closed\(\)|may_claim\(\)/{ sub( /^ */, "" ); printf "%s ", $0 }' ring_shutdown/src/lib.rs )"
printf 'the match arms in order:       %s\n' "$( awk '/pub fn for_space_or_close/{f=1} f&&/^\}$/{exit} f&&/=>/{ sub( /^ */, "" ); sub( /,$/, "" ); printf "[%s] ", $0 }' ring_shutdown/src/lib.rs )"
printf 'wait_until hands back:         %s\n' "$( command grep -ohE 'pub fn wait_until.*-> [A-Za-z<>, ]+' ring_wait/src/lib.rs | sed 's/.*-> //' )"
printf 'wait_for_close returns:        %s\n' "$( awk '/pub fn wait_for_close/{f=1} f&&/^-> /{ sub( /^-> /, "" ); print; exit }' ring_shutdown/src/lib.rs )"
printf 'for_space_or_close returns:    %s\n' "$( awk '/pub fn for_space_or_close/{f=1} f&&/^-> /{ sub( /^-> /, "" ); print; exit }' ring_shutdown/src/lib.rs )"
printf 'Wake variants:                 %s\n' "$( awk '/^pub enum Wake$/{f=1} f&&/^\}$/{exit} f&&/^  [A-Z]/{ sub( /,$/, "" ); sub( /^ */, "" ); printf "%s ", $0 }' ring_shutdown/src/lib.rs )"
printf 'the caveat on Closed:          %s\n' "$( command grep -o 'The condition may still be false.' ring_shutdown/src/lib.rs )"
printf 'the doc line on Ready:         %s\n' "$( awk '/^pub enum Wake$/{f=1} f&&/^  Ready,$/{ print d; exit } f&&/^ *\/\/\//{ d=$0; sub( /^ *\/\/\/ */, "", d ) }' ring_shutdown/src/lib.rs )"
printf 'advisory caveats elsewhere:    %s\n' "$( command grep -hE 'advisory|does not predict|deliberately not' ring_shutdown/src/lib.rs | command grep -v '\.md' | command grep -ohE 'advisory|does not predict|deliberately not' | sort | uniq -c | tr '\n' ' ' )"
printf 'of those, filename citations:  %s\n' "$( command grep -cE 'is_advisory_to_an_unguarded' ring_shutdown/src/lib.rs )"
```

Live output:

```
the side channel it captures:  let mut closed = false;
the predicate checks first:    if shutdown.is_closed() pair.may_claim() 
the match arms in order:       [Ok( _ ) if closed => Ok( Wake::Closed )] [Ok( _ ) => Ok( Wake::Ready )] [Err( _ ) => Err( RingError::Full )] 
wait_until hands back:         Result< usize, RingError >
wait_for_close returns:        Result< usize, RingError >
for_space_or_close returns:    Result< Wake, RingError >
Wake variants:                 Ready Closed 
the caveat on Closed:          The condition may still be false.
the doc line on Ready:         it, the condition may no longer hold.
advisory caveats elsewhere:          2 advisory       1 deliberately not       1 does not predict 
of those, filename citations:  2
```

### Algorithms

| File | Relationship |
|------|--------------|
| [`001_drain_to_empty.md`](001_drain_to_empty.md) | The crate's other procedure, and the other place a loop's spelling was chosen for a reason outside the loop |

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_stopped_proof_token.md`](../type/001_stopped_proof_token.md) | The other place the crate encodes a fact in a value rather than in a comment |

### Items

| File | Relationship |
|------|--------------|
| [`../item/002_two_checks_and_two_waiters.md`](../item/002_two_checks_and_two_waiters.md) | The same two waiters read for what they report on failure rather than on success |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [`../non_functional_requirement/001_what_a_guarded_push_costs.md`](../non_functional_requirement/001_what_a_guarded_push_costs.md) | The other accessor whose answer is a past instant, and what a caller pays to read it |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `for_space_or_close`, `wait_for_close`, and the `Wake` enum |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `for_space_or_close_names_the_exit_it_took` — both `Ok` arms, including the closed-and-has-room case |

### SD3 — One Waiter Returns the Spin Count and the Other Throws It Away

`ring_wait::wait_until` returns `Ok( attempt )` — the index of the attempt on
which the predicate came true. That number is the only feedback a caller has
for whether a spin budget is generously or barely sized.

`wait_for_close` passes it straight through: its return type is
`Result< usize, RingError >`, and a caller waiting on teardown learns how many
spins the wait actually took.

`for_space_or_close` discards it. Its match reads `Ok( _ )` twice, replacing
the count with a `Wake`, and its return type is `Result< Wake, RingError >`.

The information was available and the type has room for it —
`Result< ( Wake, usize ), RingError >` costs nothing at runtime and the tuple is
`Copy`. Nothing in either doc comment mentions the asymmetry.

What makes it worth recording is which side lost the number. `wait_for_close`
runs at teardown, once, where nobody is tuning anything. `for_space_or_close`
runs on the publish path, in a loop, under a budget a caller is expected to
choose — and that is the one the crate leaves blind. A producer that wants to
know whether it is spending 2 spins or 999 per publish has to instrument around
the call, in a crate whose surface promises to be reachable from inside a tick.

### SD4 — `Wake::Closed` Carries the Staleness Caveat and `Wake::Ready` Does Not

`Wake` has two variants and the crate documents them to different standards:

- `Closed` — *"The ring closed while waiting. **The condition may still be
  false.**"*
- `Ready` — *"The condition the caller was waiting for became true."*

`Closed` is the variant that does not need the caveat. A close is one-way until
a `Stopped::reopen`, which requires a token the waiting producer does not hold,
so `Wake::Closed` remains true for as long as the caller could act on it.

`Ready` is the variant that does. It means `pair.may_claim()` returned true at
some attempt inside the spin; by the time the caller reads it another producer
may have taken the slot, and `free_capacity`'s own contract is already advisory
at MPSC. `Wake::Ready` is a statement about a past instant presented as a
present fact, and the one that could mislead is the one with no note.

The crate is scrupulous about exactly this elsewhere, which is what makes the
omission legible rather than ordinary. `Guarded::is_blocked` says *"It does not
predict a refusal."* `Guarded::free_capacity` says *"binding at SPSC, advisory
elsewhere."* `Shutdown::admit`'s error is *"deliberately not transient."* Three
neighbouring items go out of their way to state when an answer stops being
true; the fourth states the opposite claim in the same paragraph as the one
variant that did not need it.

**Disposition:** applied — `Wake::Ready`'s doc comment now carries the same
staleness caveat as `Closed`, `is_blocked`, `free_capacity` and `admit`. Now
prints: `the condition may no longer hold`

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^pub enum Wake$/{f=1} f&&/^  Ready,$/{exit} f&&/^ *\/\/\//{ sub(/^ */,""); print }' ring_shutdown/src/lib.rs
```

Live output:

```
/// The condition the caller was waiting for became true, as of the instant
/// this was observed. Like [`Wake::Closed`], this is a statement about a
/// past instant, not a current guarantee — by the time the caller acts on
/// it, the condition may no longer hold.
```
