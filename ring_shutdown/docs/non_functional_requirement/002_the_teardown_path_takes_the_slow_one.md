# Non-Functional Requirement: The Teardown Path Takes the Slow One

### Scope

- **Purpose**: State what the two drain shapes cost relative to each other, record which one the crate's headline teardown call routes through, and name the property that choice is paying for.
- **Responsibility**: `drain_all` against `discard_all` as work per record; `reset`'s routing between them; the allocation the crate declines to make and the one it declines to size.
- **In Scope**: `Stopped::drain_all`, `Stopped::discard_all`, `reset`.
- **Out of Scope**: Why the loop terminates at all (→ [`../invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)); the loop's shape as an algorithm (→ [`../algorithm/001`](../algorithm/001_drain_to_empty.md)); the publish side's cost (→ [`001`](001_what_a_guarded_push_costs.md)).

### The Requirement

**Teardown is off the hot path, so it may be slower than publication — but it
must not be gratuitously slower than the alternative sitting three lines away.**

### Two Shapes, One Chosen Twice

| | `drain_all` | `discard_all` |
|---|---|---|
| Underlying call | `consumer.try_recv_batch( out )` | `consumer.try_recv()` |
| Calls for *n* records | one per batch | *n* |
| Keeps the records | yes, into a caller `Vec` | no |
| Reached by `reset` | no | **yes** |

`reset` is documented as *"close, empty, and reopen — the whole teardown in one
call … the operation a test harness or a world recycle wants."* That is the
call a caller reaches for repeatedly, in a loop, between rounds. It routes
through `discard_all`, which takes one `try_recv` per record.

### What the Per-Record Shape Buys

`discard_all`'s doc gives the reason: *"Each record is dropped individually as
it is taken, so a `T` with a `Drop` impl still runs it."*

That is a correctness argument, and against a `T` with a destructor it would be
the right trade. It is worth checking what it costs and what it is protecting.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'what reset calls to empty:     %s\n' "$( awk '/^pub fn reset</{f=1} f&&/^\}$/{exit} f&&/stopped\./{ sub( /.*stopped\./, "" ); sub( /\(.*/, "" ); printf "%s ", $0 }' ring_shutdown/src/lib.rs )"
printf 'drain_all recvs with:          %s\n' "$( awk '/pub fn drain_all/{f=1} f&&/^  \}$/{exit} f' ring_shutdown/src/lib.rs | command grep -ohE 'try_recv[_a-z]*' | sort -u | tr '\n' ' ' )"
printf 'discard_all recvs with:        %s\n' "$( awk '/pub fn discard_all/{f=1} f&&/^  \}$/{exit} f' ring_shutdown/src/lib.rs | command grep -ohE 'try_recv[_a-z]*' | sort -u | tr '\n' ' ' )"
printf 'the reason the doc gives:      %s\n' "$( command grep -o 'so a .T. with a .Drop. impl still runs it' ring_shutdown/src/lib.rs )"
printf 'destructors in this crate:     %s\n' "$( command grep -c 'impl.*Drop for' ring_shutdown/src/lib.rs || true )"
printf 'destructors in its suite:      %s\n' "$( command grep -c 'impl.*Drop for' ring_shutdown/tests/shutdown_test.rs || true )"
printf 'what Drop names in the suite:  %s\n' "$( command grep -ohE 'Drop[A-Za-z]*' ring_shutdown/tests/shutdown_test.rs | sort -u | tr '\n' ' ' )"
printf 'family crates with a destructor: %s\n' "$( command grep -rl 'impl.*Drop for' ring_*/src 2>/dev/null | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'allocations outside doctests:  %s\n' "$( command grep -vE '^ *///' ring_shutdown/src/lib.rs | command grep -cE 'Vec::new|vec!|Box::|with_capacity|String::' || true )"
printf 'reserve calls before a drain:  %s\n' "$( command grep -c '\.reserve(' ring_shutdown/src/lib.rs || true )"
printf 'the count is available from:   %s\n' "$( command grep -n 'pub fn len( &self ) -> usize' ring_core/src/lib.rs | head -1 | awk -F: '{ print "ring_core/src/lib.rs:" $1 }' )"
printf 'what the surface claims:       %s\n' "$( command grep -ohE 'Nothing on this surface [a-z ]+\.' ring_shutdown/docs/api/001_shutdown_surface.md )"
printf 'crate-level attributes here:   %s\n' "$( command grep -ohE '^#!\[ .* \]' ring_shutdown/src/lib.rs | tr '\n' ' ' )"
printf 'the family test that checks:   %s\n' "$( command grep -rl 'GlobalAlloc' ring_*/tests 2>/dev/null | tr '\n' ' ' )"
```

Live output:

```
what reset calls to empty:     discard_all reopen 
drain_all recvs with:          try_recv_batch 
discard_all recvs with:        try_recv 
the reason the doc gives:      so a `T` with a `Drop` impl still runs it
destructors in this crate:     0
destructors in its suite:      0
what Drop names in the suite:  DropNewest 
family crates with a destructor: ring_mpsc ring_spsc 
allocations outside doctests:  0
reserve calls before a drain:  0
the count is available from:   ring_core/src/lib.rs:633
what the surface claims:       Nothing on this surface allocates or parks.
crate-level attributes here:   #![ deny( missing_docs ) ] 
the family test that checks:   ring_barrier/tests/allocation_test.rs ring_claim/tests/allocation_test.rs ring_consume/tests/allocation_test.rs ring_cursor/tests/allocation_test.rs ring_flush/tests/append_cost_test.rs 
```

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [`001_what_a_guarded_push_costs.md`](001_what_a_guarded_push_costs.md) | The publish side of the same question, and the measurement surface both findings run out of |

### Algorithms

| File | Relationship |
|------|--------------|
| [`../algorithm/001_drain_to_empty.md`](../algorithm/001_drain_to_empty.md) | The loop both shapes are instances of, and why one batch is not a drain |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_drain_terminates_because_close_preceded_it.md`](../invariant/002_drain_terminates_because_close_preceded_it.md) | Why either shape terminates, which is not a property of either loop |

### Items

| File | Relationship |
|------|--------------|
| [`../item/001_the_declared_surface_and_its_attributes.md`](../item/001_the_declared_surface_and_its_attributes.md) | The three items measured here, in the full inventory |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Both drain loops and `reset`'s routing between them |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `reset_empties_and_reopens` — asserts the count and the reopened state, nothing about the path taken |

### SD35 — The Headline Teardown Call Routes Through the Record-at-a-Time Loop

The crate ships two drains. `drain_all` calls `consumer.try_recv_batch( out )`
in a loop — one call per batch, whatever `ring_core`'s backend can hand over at
once. `discard_all` calls `consumer.try_recv()` in a loop — one call per
record, with the `Option` unwrap and the backend dispatch paid *n* times.

`reset` calls `discard_all`.

`reset` is the crate's headline teardown operation, documented as *"the whole
teardown in one call"* and as *"the operation a test harness or a world recycle
wants"* — which is to say the operation called repeatedly, in a loop, between
rounds, on a ring that may be full. It takes the per-record path.

The justification for that path is in `discard_all`'s own doc: *"so a `T` with a
`Drop` impl still runs it."* A batched drain into a `Vec` that is then dropped
would run the same destructors, so the argument is really about not needing the
`Vec` at all — which is a fair reason to have `discard_all`, and not a reason
for `reset` specifically to prefer it over draining into a scratch buffer it
could own.

What makes this a finding rather than a preference is that the trade is
undocumented and unmeasurable. Nothing in the crate says `reset` is the slow
path, nothing says why, and [SD33](001_what_a_guarded_push_costs.md) established
that there is no bench anywhere in the family that could tell a reader whether
the difference matters at all. The choice reads as accidental because there is
no record of it having been made.

And the property being bought is one the family barely exercises. This crate
declares no destructor and neither does its suite; across the whole
thirty-plus-crate family exactly two crates impl `Drop`. The word appears twice
in this crate's tests and both are `DropNewest` — an overflow policy, not a
destructor. So the destructor-safety argument, which is correct in principle,
is currently protecting nothing that exists in reach of this code.

### SD36 — The Surface Promises It Does Not Allocate, and the Family Already Built the Check It Does Not Use

[`api/001`](../api/001_shutdown_surface.md)'s fourth surface claim is
unambiguous: *"Nothing on this surface allocates or parks."* It is true —
outside doctests the crate contains no `Vec::new`, no `vec!`, no `Box::`, no
`with_capacity`, no `String::`. And it is load-bearing: the claim's own
sentence continues *"this is what makes the surface reachable from inside a
tick"*, tying it to the requirement that nothing reachable from a tick may
allocate or block.

Nothing enforces it. The crate's only crate-level attribute is
`#![ deny( missing_docs ) ]` — there is no `#![ no_std ]`, no allocator
assertion, no test. A `Vec` added to `drain_all` tomorrow compiles, ships, and
leaves the claim standing in a document the next reader may not open.

What makes the gap avoidable rather than merely unfortunate is that the family
has already solved it. `ring_flush/tests/append_cost_test.rs` faces the identical
problem — a "this path allocates nothing" requirement, and a workspace that
denies `unsafe_code` so a counting `GlobalAlloc` is off the table — and lands on
the strongest safe proxy: assert the buffer's *capacity does not change* across
N operations. That test exists, its reasoning is written out, and it transfers
here almost verbatim: drain a full ring twice into the same `Vec` and assert the
capacity is unchanged on the second pass. Nobody has.

The second half is sharper, because it is a cost the crate *does* impose.
`drain_all( &self, consumer, out : &mut Vec< T > )` pushes into the caller's
`Vec` through `try_recv_batch` and never calls `reserve`. Draining *n* records
into an empty `Vec` therefore pays the growth schedule — a logarithmic number of
reallocations and up to *n* extra moves — while the exact count is one public
call away: `ring_core::Consumer::len`, which the crate's own `reset` doctest
already uses to check the ring came back empty.

So the crate refuses to allocate on its own behalf, states that refusal as a
surface promise, and then declines the one line that would make the caller's
allocation cost *O(1)* instead of amortized. The promise is written down and
unchecked; the cost it pushes onto the caller is neither.
