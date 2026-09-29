# Algorithm: The Compare-Exchange That Refuses

### Scope

- **Purpose**: Specify the crate's one state-changing operation — two statements, one atomic — and what it does in each of the three positions a caller's `start` can be in relative to the frontier.
- **Responsibility**: State the algorithm, justify the asymmetric orderings, place it against the family's other compare-exchange sites, and show which of its three outcomes each test covers.
- **In Scope**: `try_publish` (`src/lib.rs:161-165`).
- **Out of Scope**: The retry loop built over it — see [`algorithm/002`](002_a_loop_with_no_budget.md).

### The Whole Operation

```rust
pub fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >
{
  let end = start.advanced_by( len as u64 );
  self.cursor.compare_exchange( start, end, PUBLISH, GATING ).map( | _ | end )
}
```

Two statements. The first is pure arithmetic on `Seq` — `ring_types`' own
`advanced_by`, a `const fn` over `u64` — and the second is the crate's only
atomic read-modify-write. There is no branch, no loop, and no allocation.

The `.map( | _ | end )` is the only sleight of hand: `compare_exchange` returns
`Result< Seq, Seq >` whose `Ok` payload is the *previous* value, which the caller
already knows because it supplied it as `start`. Replacing it with `end` makes
both arms of the returned `Result` answer the same question — *where is the
frontier now* — rather than `Ok` answering "where it was" and `Err` answering
"where it is".

### The Three Positions and What Each Gets

`compare_exchange( start, end, … )` succeeds exactly when the cursor reads
`start`. That single condition splits the caller's whole input space into three:

| `start` vs frontier | Meaning | Result | Frontier after |
|---------------------|---------|--------|----------------|
| `start > published` | a predecessor has not published yet | `Err( published )` | unchanged |
| `start == published` | it is this producer's turn | `Ok( start + len )` | advanced |
| `start < published` | already published, or a range this producer does not own | `Err( published )` | unchanged |

The two error rows are the same code path and different bugs. The first is
ordinary contention and is expected — it is what `publish` loops on. The second
is a caller error, and the reason it is *also* refused is the more important
half: an advance from a `start` behind the frontier would move the published
cursor **backwards**, un-publishing slots a consumer may already be reading.
`tests/publish_test.rs:88-99` is the test for that direction and says so at
`:90-92`; the doc example at `src/lib.rs:148-160` covers only the first.

### PB7 — Three Compare-Exchange Sites, and Only This One Discards the Failure Value

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'compare_exchange' */src/*.rs \
  | grep -vE '^\S+:\s*///' | grep -vE 'fn compare_exchange|compare_exchanges'
```

Live output:

```
ring_atomic/src/lib.rs:      .compare_exchange( current.0, new.0, success, failure )
ring_atomic/src/lib.rs:    self.cell.compare_exchange( current, new, success, failure )
ring_claim/src/lib.rs:      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
ring_claim/src/lib.rs:      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
ring_cursor/src/lib.rs:    self.0.get().compare_exchange( current, new, success, failure )
ring_publish/src/lib.rs:    self.cursor.compare_exchange( start, end, PUBLISH, GATING ).map( | _ | end )
```

Six hits. Three are forwarding implementations of `SeqCell::compare_exchange`
itself — `ring_atomic:228` (`AtomicSeq`), `ring_atomic:494` (`CountingSeq`), and
`ring_cursor:219` (`PaddedCursor`). The genuine call sites are the other three,
in two crates:

| Site | Loop shape | On `Err( actual )` | Bound |
|------|-----------|--------------------|-------|
| `ring_claim:437` (`claim`) | `while count <= headroom( current )` | `current = actual` | headroom; exits to `Full` |
| `ring_claim:488` (`claim_up_to`) | `while let granted @ 1.. = …` | `current = actual` | headroom; exits to `Full` |
| `ring_publish:164` (via `publish:202`) | `loop` | *discarded* | none |

Both crates retry, and they retry against opposite things. `ring_claim` retries
against a **moved target**: another producer took the cursor, so the correct next
attempt starts from wherever it now is, and the failure value *is* the next
input. This crate retries against a **fixed target**: a producer may publish only
the range it claimed, so `start` cannot change, and the failure value is not an
input to anything.

That difference is the whole distinction between a race and a turn-gate on the
same primitive. `ring_claim`'s exchange is a contest several producers can win in
any order; this one is a queue with a fixed order, and losing it means *not yet*
rather than *try elsewhere*.

### PB8 — The Failure Value Is Asserted Five Times and Consumed Zero Times

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'Err( Seq' ring_publish/src/lib.rs ring_publish/tests/*.rs
```

Live output:

```
ring_publish/src/lib.rs:  /// assert_eq!( publisher.try_publish( Seq( 4 ), 4 ), Err( Seq::ZERO ) );
ring_publish/tests/handshake_test.rs:      Err( Seq::ZERO ),
ring_publish/tests/handshake_test.rs:      Err( Seq::ZERO ),
ring_publish/tests/publish_test.rs:  assert_eq!( publisher.try_publish( Seq( 4 ), 4 ), Err( Seq::ZERO ) );
ring_publish/tests/publish_test.rs:  assert_eq!( publisher.try_publish( Seq( 4 ), 2 ), Err( Seq( 8 ) ) );
ring_publish/tests/publish_test.rs:  assert_eq!( publisher.try_publish( Seq::ZERO, 8 ), Err( Seq( 8 ) ), "not even re-publishing" );
```

Five occurrences — `src/lib.rs:155` (the doc example), `tests/publish_test.rs:80`,
`:96`, `:97`, and `tests/handshake_test.rs:370`. Every one is an equality
assertion against a literal. Nothing anywhere binds the value and uses it.

The documentation describes it as more than that. `src/lib.rs:143-146`:

> The current published position, when it is not `start` — meaning some earlier
> claim has not been published yet. Deliberately not a `RingError`: this is not
> a failure, it is `compare_exchange`'s "try again", and the value returned is
> what to try against next.

and `tests/publish_test.rs:75-77`:

> The error value is not decoration — it is what B retries against, so a version
> returning `RingError` or `()` would force B to re-read separately and race
> again.

Both are borrowing `compare_exchange`'s own idiom, where the failure value is
literally the next call's `current` argument. Here it cannot be: `try_publish`'s
parameter is `start`, the range the caller owns, and no caller may substitute a
different one. What the value actually supplies is *how far behind the frontier
is* — usable for a back-off decision, a diagnostic, or a progress check, none of
which any caller currently makes.

The design is still right for the reason stated: returning `RingError` or `()`
would force a separate `published()` read, which is a second atomic load and a
second chance for the answer to be stale. The narrower true claim is that the
value is free to return and nothing yet needs it — the same shape as
[`api/001`](../api/001_six_methods_and_no_caller.md)'s finding about the surface
as a whole.

### The Orderings Are Deliberately Asymmetric

`PUBLISH` on success, `GATING` on failure — `Release` and `Acquire`, from two
different crates:

```rust
const PUBLISH : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;  // :67
use ring_cursor::{ PaddedCursor, SeqCell, GATING };                                    // :57
```

`tests/manual/readme.md:93-95` states why the asymmetry is correct rather than
an oversight:

> A failed exchange published nothing, so it needs no release — but it did read
> the cursor, and the value it returns is what the caller retries against.

`Release` is the entire happens-before edge between this producer's slot writes
and a consumer's reads of them (`src/lib.rs:62-66`). It is needed only on the
path that actually makes those writes visible. The failure path performs a read,
so it needs `Acquire` and nothing more — and `Acquire` is what `GATING` is, named
once in `ring_cursor:89` and shared by every consumer-side read in the family.

Neither ordering appears inline. `tests/manual/readme.md § P3` is the check, and
`:89-91` gives the reason in one sentence: *"an ordering chosen at the point of
use is an ordering nobody will find when they go looking for why the ring races."*

### The Zero-Length Case Is Accepted, Not Refused

`try_publish( start, 0 )` computes `end = start`, so the exchange is
`compare_exchange( start, start, … )` — a successful no-op when `start` is the
frontier, and a refusal otherwise. It is not special-cased anywhere; it falls out
of the arithmetic.

`tests/publish_test.rs:101-112` covers it and `:104-106` gives the design reason:

> Accepted rather than refused, because a producer that claimed nothing has
> nothing to wait for and no reason to be told to retry — and because
> `claim_up_to` can legitimately grant a shorter range than asked for.

The second clause is the one that matters. `ring_claim::Claimer::claim_up_to`
(`:449`) grants `max.min( headroom )`, which can be less than requested and — at
the loop's own exit condition — zero. A publication path that rejected zero would
turn a legitimate partial grant into an error at the far end of the handshake.

### What This Algorithm Does Not Do

`tests/manual/readme.md § P4` checks the absences by grep, because the module
documentation rejects two named alternatives and *"the rejection is only real if
the code contains neither"*:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -iE "bitmap|contiguous|while|for |max\(|highest" \
  || echo '(no matches — neither rejected alternative appears in the code)'
# control: the identical expression over ring_claim, which does loop
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs \
  | grep -iE "bitmap|contiguous|while|for |max\(|highest"
```

Live output:

```
(no matches — neither rejected alternative appears in the code)
    while count <= self.consumers.headroom( current )
    while let granted @ 1.. = max.min( self.consumers.headroom( current ) )
```

No output. There is no scan for the highest contiguous published position and no
per-slot availability bitmap — both of which would let producer B publish while
producer A is still writing, and both of which are
[`decisions/001`](../decisions/001_refused_rather_than_reordered.md)'s rejected
alternatives, deferred to `ring_mpsc`.

The check is worth more than it looks because the filter drops comment lines: the
module documentation *argues about* bitmaps and contiguity at length, so an
unfiltered grep would report the rejected designs as implementations. `§ P4` names
that at `:15-18`.

### PB47 — The Arithmetic Runs Before the Exchange, on the Path That Discards It

```sh
cd "$(git rev-parse --show-toplevel)"
# the whole body of try_publish — the arithmetic, then the exchange
grep -F -A 4 'pub fn try_publish( ' ring_publish/src/lib.rs
# no branch stands between them …
grep -F -A 4 'pub fn try_publish( ' ring_publish/src/lib.rs \
  | grep -cE '\bif\b|\bmatch\b' || true
# … control: the identical expression over `publish`, whose body does branch
grep -F -A 6 'pub fn publish( ' ring_publish/src/lib.rs \
  | grep -cE '\bif\b|\bmatch\b'
```

Live output:

```
  pub fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >
  {
    let end = start.advanced_by( len as u64 );
    self.cursor.compare_exchange( start, end, PUBLISH, GATING ).map( | _ | end )
  }
0
1
```

Two statements, in that order, with nothing between them — zero branches
against `publish`'s one. `advanced_by` is evaluated unconditionally, before the
exchange gets its chance to refuse, so a refused publication computes
`start + len` and throws it away.

The wasted addition is not the point; it is one instruction on a path that is
about to spin anyway. What travels with it is `advanced_by`'s overflow
behaviour, which [`type/001`](../type/001_a_seq_a_usize_and_the_one_cast.md)
records as undocumented on this method and documented wrongly on its sibling.
In a debug build that behaviour is a panic, and this ordering puts the panic on
the path that was going to return `Err`.

So the failure a caller sees for an over-large `len` does not depend on whether
it was that caller's turn. Producer B, arriving out of order with a `len` that
overflows, panics rather than being told *not yet* — the refusal it was owed is
computed one line too late to reach it. Reversing the two statements would fix
that and cost a second `start` comparison, which is why it is recorded rather
than proposed.

The condition needs a `len` near `u64::MAX`, which
[`type/001`](../type/001_a_seq_a_usize_and_the_one_cast.md) prices at 584 years
of continuous publication, so this is a note about where the check sits rather
than a reachable defect. `publish` inherits it at most once per call rather
than once per spin: the overflow is a function of `start` and `len`, both
loop-invariant, so the loop either panics on its first iteration or never.

### Algorithms

| File | Relationship |
|------|--------------|
| [002_a_loop_with_no_budget.md](002_a_loop_with_no_budget.md) | The retry loop this operation is the body of |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_a_result_whose_error_is_not_an_error.md](../api/002_a_result_whose_error_is_not_an_error.md) | The return type, and why it is not `RingError` |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_padded_cursor_and_nothing_else.md](../data_structure/001_one_padded_cursor_and_nothing_else.md) | The cell this operation exchanges on |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_refused_rather_than_reordered.md](../decisions/001_refused_rather_than_reordered.md) | Why refusal, and what the alternatives would have cost |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_ten_crates_name_it_and_none_depends_on_it.md](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) | `ring_cursor` and `ring_types`, the two crates this operation is built from |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_frontier_moves_only_by_compare_exchange.md](../invariant/001_the_frontier_moves_only_by_compare_exchange.md) | The structural property this being the only mutation establishes |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_two_publications.md](../item/002_the_two_publications.md) | This operation and its looping sibling, contract by contract |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_named_ordering_constant.md](../pattern/002_the_named_ordering_constant.md) | `PUBLISH` and `GATING`, and the family's other nine |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_seq_a_usize_and_the_one_cast.md](../type/001_a_seq_a_usize_and_the_one_cast.md) | `len as u64`, the crate's only cast, on this line |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:57,60-67,138-165` | The imports, the constant with its rationale, and the operation with its `# Errors` contract |
| `ring_claim/src/lib.rs:432-445,483-496` | The two compare-exchange sites that retry against a moved target |
| `ring_cursor/src/lib.rs:89,216` | `GATING`, and the forwarding `compare_exchange` |
| `ring_types/src/id.rs:65-68` | `Seq::advanced_by`, the operation's first statement |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:62-70` | Advancing from the exact frontier |
| `tests/publish_test.rs:72-85` | A start past the frontier is refused, and the frontier does not move |
| `tests/publish_test.rs:87-99` | A start behind the frontier is refused — the direction the doc example never shows |
| `tests/publish_test.rs:101-112` | A zero-length publication is accepted |
| `tests/manual/readme.md § P2` | The cursor is advanced only by compare-exchange — no `store`, no `fetch_add` |
| `tests/manual/readme.md § P3` | The orderings are named once, and the asymmetry is deliberate |
| `tests/manual/readme.md § P4` | Neither rejected alternative is present in the code |
