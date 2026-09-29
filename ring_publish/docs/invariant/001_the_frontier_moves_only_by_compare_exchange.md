# Invariant: The Frontier Moves Only By Compare-Exchange

### Scope

- **Purpose**: State the crate's central structural invariant — one mutation site, forward only, from the exact current value — and the mechanical check that enforces it.
- **Responsibility**: State the invariant in three parts, show that each is structural rather than behavioural, give the grep that proves it, and name what each part prevents.
- **In Scope**: Every write to `Publisher`'s cursor.
- **Out of Scope**: The read side — see [`invariant/002`](002_is_published_is_exclusive_of_the_frontier.md).

### The Invariant, in Three Parts

For every `Publisher` and every possible sequence of operations on it:

1. **One site.** The cursor is written at exactly one place in the crate,
   `src/lib.rs:164`, and by exactly one operation, `compare_exchange`.
2. **From the exact current value.** That exchange's `current` argument is the
   caller's `start`, so the write succeeds only when the cursor already reads
   `start`.
3. **Therefore forward only.** `end = start.advanced_by( len as u64 )` and
   `len : usize`, so `end >= start` for every input, and the cursor's value is
   non-decreasing over its whole lifetime.

Part 3 is a consequence of parts 1 and 2 plus `advanced_by`'s signature; it is
not separately enforced. The three together give the property everything else in
the crate rests on: **`published()` is monotonically non-decreasing, and every
sequence below it was published by a producer that owned it.**

### PB19 — Enforced By Absence, Checked By Grep

`SeqCell` — the trait `PaddedCursor` implements — offers four methods:
`load`, `store`, `fetch_add`, `compare_exchange` (`ring_atomic:83-103`). Three of
them can move the cursor, and two of those can move it *anywhere*:

| Method | Could this crate call it | Would break |
|--------|-------------------------|-------------|
| `store( value, order )` | yes — the field is in scope | parts 1, 2 **and** 3: any value, including backwards |
| `fetch_add( n, order )` | yes | parts 1 and 2: advances from wherever it is, so two producers both "succeed" and one range is skipped |
| `compare_exchange( … )` | this is the one | — |
| `load( order )` | read-only | — |

Nothing type-level prevents the first two. The invariant holds because the crate
does not call them, and `tests/manual/readme.md § P2` is what keeps it that way:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -E "fetch_add|compare_exchange|\.store\("
```

Live output:

```
    self.cursor.compare_exchange( start, end, PUBLISH, GATING ).map( | _ | end )
```

**Expected: exactly one hit** — the `compare_exchange` in `try_publish`. Run
today it produces exactly that, at the filtered file's line 38. The check's own
statement of what it is protecting, at `:65-68`:

> The structural form of the invariant `try_publish`'s contract rests on: a
> publication that can be *assigned* rather than exchanged is a publication that
> can move backwards, and a published cursor moving backwards un-publishes slots
> a consumer may already be reading.

`:76-78` covers the second half explicitly: *"`publish` must not have its own
advance: it is a retry loop around `try_publish` and nothing else, so there is
exactly one place in the crate where the cursor moves."*

The check is a grep rather than a test because the property is about *what code
exists*, not about what it computes. A `store`-based `publish` would pass every
behavioural test in the suite under a single producer, and most of them under
several.

### The Hole the Invariant Does Not Close

`cursor()` hands out `&PaddedCursor`, and `PaddedCursor` implements `SeqCell`
publicly. Any holder of that reference can call `store` or `fetch_add` and move
the frontier arbitrarily — including backwards.

That is deliberate and unavoidable: the reference exists so a consumer's
`Barrier` can read the cursor
([`data_structure/002`](../data_structure/002_the_four_cursors_of_the_handshake.md)),
and there is no read-only cursor type in the family to hand out instead. So the
invariant's scope is precisely *"within this crate"*, and its guarantee to a
caller is *"nothing here will move your frontier except a publication"* — not
*"your frontier cannot be moved otherwise"*.

`tests/publish_test.rs:54-67` and `handshake_test.rs`'s barrier wiring both use the
handed-out reference read-only. Nothing in the family writes through a borrowed
publisher cursor, and no check would notice if something started to.

### What Each Part Prevents

| Part | If it failed | Symptom |
|------|--------------|---------|
| One site | a second write path exists | the grep fires; nothing else does, until two producers race |
| From the exact value | `fetch_add`-style advance | two producers each advance by their own `len` from wherever the cursor is; the frontier passes a range nobody published, and a consumer reads unwritten slots |
| Forward only | `store` of a lower value | the frontier retreats; sequences already handed to a consumer become unpublished, and the producer re-issues them — duplicated items |

The middle row is the one the loom model catches from the other direction.
`tests/handshake_test.rs:77-165` explores every interleaving of one claim and one
drain and asserts the consumer is never offered a claimed-but-unwritten slot;
`tests/manual/readme.md § P1`'s mutation 2 breaks it deliberately — publishing
before writing — and records that both mutations fail at the same assertion with
`left: 0`, the slot's initial value.

The third row is covered behaviourally by `tests/publish_test.rs:96-108`, which
asserts `"the frontier never retreats"` after two refused publications from
behind it.

### Why This Is Stronger Than It Looks

A monotone frontier advanced only from its exact current value means the cursor's
value sequence over the ring's whole life is exactly the sequence of publication
endpoints, in order, with no gaps and no repeats. That is what lets
[`invariant/002`](002_is_published_is_exclusive_of_the_frontier.md)'s
`seq < published()` be a complete answer rather than an approximation, and it is
what makes
[`algorithm/002`](../algorithm/002_a_loop_with_no_budget.md)'s termination
argument work: a spinning producer waiting for `start` is guaranteed the frontier
will *pass through* `start` rather than jump over it.

It is also what a per-slot bitmap or a highest-contiguous scan would give up —
both let the frontier advance past ranges published by others in an order the
cursor does not record ([`decisions/001`](../decisions/001_refused_rather_than_reordered.md)).

### PB50 — The Pairing the Invariant Rests On Is Half-Absent From the Crate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_publish
# every mention of Acquire in the crate — the control for the empty result below
grep -c 'Acquire' src/lib.rs
# … and every one of them is a doc line, so filtering those out leaves nothing
grep -n 'Acquire' src/lib.rs | grep -vE ':\s*(///|//!)' \
  || echo '(no matches — Acquire is never written outside the doc lines)' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
2
(no matches — Acquire is never written outside the doc lines)
```

Two mentions, and both are documentation: the prose explaining the pairing, and
a doctest calling `load( Ordering::Acquire )`. Filter the doc lines out and the
crate has no `Acquire` left in any position the compiler reads.

The invariant is stated in terms of that pairing. `src/lib.rs:60-66` calls
`Release` "paired with the consumer's `Acquire` read of the same cursor" and
names the pair "the entire happens-before edge between a producer's slot writes
and a consumer's reads of them". Only one half of that edge is expressible
here. The `Release` half is `PUBLISH`, declared in this crate and used on the
exchange's success path. The `Acquire` half belongs to whoever reads the
cursor, and the only `Acquire` this crate can name is `GATING`, imported from
`ring_cursor` and passed on the exchange's *failure* path — where it orders a
read that found the frontier unmoved.

So the crate that owns the invariant supplies one ordering and can only
describe the other. The consumer holding up the far end is `ring_barrier`,
which this crate does not depend on and cannot see. What actually checks the
edge is neither constant but the loom model in `tests/handshake_test.rs`, which
is why [`lifecycle/002`](../lifecycle/002_the_four_operation_handshake.md)
treats that test rather than the source as the place the guarantee lives.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_compare_exchange_that_refuses.md](../algorithm/001_the_compare_exchange_that_refuses.md) | The one site |
| [../algorithm/002_a_loop_with_no_budget.md](../algorithm/002_a_loop_with_no_budget.md) | The termination argument this invariant supplies step 2 of |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_padded_cursor_and_nothing_else.md](../data_structure/001_one_padded_cursor_and_nothing_else.md) | The private field, and the reference that is the hole |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_refused_rather_than_reordered.md](../decisions/001_refused_rather_than_reordered.md) | The choice that makes "from the exact value" possible |

### Invariants

| File | Relationship |
|------|--------------|
| [002_is_published_is_exclusive_of_the_frontier.md](002_is_published_is_exclusive_of_the_frontier.md) | What a monotone exact frontier lets the read side promise |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_conflating_the_two_cursors.md](../pitfall/002_conflating_the_two_cursors.md) | The failure a `fetch_add` advance would reintroduce |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_loom_seam_and_its_only_user.md](../workaround/001_the_loom_seam_and_its_only_user.md) | The only check that the invariant holds under real reordering |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:161-165,200-210` | The one mutation site, and the loop that adds none |
| `ring_atomic/src/lib.rs:108-151` | The four `SeqCell` methods, three of which could break this |
| `ring_types/src/id.rs:65-68` | `advanced_by`, which makes part 3 fall out of the types |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:96-108` | The frontier never retreats — two refusals from behind |
| `tests/publish_test.rs:223-247` | The frontier's whole value sequence, checked point by point |
| `tests/handshake_test.rs:77-165` | Every interleaving of one claim and one drain, under `loom` |
| `tests/manual/readme.md § P1` | Two mutations, both failing at the slot assertion |
| `tests/manual/readme.md § P2` | Exactly one hit: the compare-exchange, no `store`, no `fetch_add` |
