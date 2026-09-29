# Invariant: A Drain Terminates Because a Close Preceded It

### Scope

- **Purpose**: State the termination argument for `drain_all` and `discard_all`, and name the exact premise it rests on — which is not a property of the loop.
- **Responsibility**: The argument, the mechanism that enforces its ordering, and the one condition that invalidates it.
- **In Scope**: `Stopped::drain_all`, `Stopped::discard_all`, and the `Stopped` token's role in the argument.
- **Out of Scope**: The loop's own shape (→ [`algorithm/001`](../algorithm/001_drain_to_empty.md)).

### Invariant Statement

**A drain loop over a ring terminates if and only if publication into that ring
has stopped.** The loop itself has no bound: `try_recv_batch` returns whatever
was available at that instant, and against a producer that keeps up, that is
never zero.

So termination is not established by reading `drain_all`. It is established by
the fact that `drain_all` cannot be called before `close`.

### Enforcement Mechanism

`drain_all` is a method on [`Stopped`](../type/001_stopped_proof_token.md), and
`Shutdown::close` is the only constructor of a `Stopped`. There is no
expression that reaches a drain without a close first. `Stopped::reopen`
takes `self` by value, which retires *that specific token* — but `close`
takes `&self` and mints without limit, so a second token obtained before the
reopen remains live afterward, and a drain through it still compiles (→
SD23 below).

This is the difference between a documented precondition and an enforced one.
A free `drain_all( &shutdown, &mut consumer )` would carry the same comment and
none of the guarantee — a caller who forgot the close would get a hang rather
than a compile error, at teardown, with no stack pointing at the omission.

### Violation Consequences

The argument's premise is "publication has stopped", and `close` does not
establish that on its own. It sets a flag. Publication stops only for producers
that consult the flag — that is, for [`Guarded`](../api/001_shutdown_surface.md)
ones.

| Producers on the ring | Does the drain terminate? |
|---|---|
| All `Guarded` | **Yes** — no further push is accepted after the close, so the remainder is what was already published |
| A raw `Producer` exists, and is idle | Yes, incidentally |
| A raw `Producer` exists, and is publishing | **No** — the loop never sees an empty batch |

Row 3 is the failure, and it is the same one
[`pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)
describes, arriving at its most expensive symptom: a teardown that hangs rather
than an error that reports.

**The window is not zero even in row 1.** A guarded producer that read the flag
as `false` and has not yet completed its push can still land one record after
the close returns. The remainder is therefore bounded by the number of
producers in flight, not by zero — the loop terminates *eventually*, not
*immediately*, and `drain_all` looping rather than taking a single batch is
what makes that difference invisible to the caller.

#### How termination is asserted

Termination cannot be asserted directly — a test that hangs does not report a
failure, it reports nothing. What is testable is the loop's *sufficiency*: a
ring holding more records than one `try_recv_batch` returns must still come
back empty.

`drain_all_loops_until_the_ring_is_actually_empty` builds exactly that case —
fill, half-drain, refill, so the occupied region wraps — and asserts both that
the drain recovers all four records and that a second drain returns 0 rather
than looping.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'how a Stopped is minted:       %s\n' "$( command grep -o 'pub fn close.*' ring_shutdown/src/lib.rs )"
printf 'how it is spent:               %s\n' "$( command grep -o 'pub fn reopen.*' ring_shutdown/src/lib.rs )"
printf 'what enforcement claims:       %s\n' "$( awk '/^### Regenerate/{ exit } { print }' ring_shutdown/docs/invariant/002_drain_terminates_because_close_preceded_it.md | command grep -o 'so there is none that reaches a drain .after. a reopen' )"
printf 'the test that mints two:       %s\n' "$( awk '/close_is_idempotent/{ f=1 } f&&/^fn /{ print $2; exit }' ring_shutdown/tests/shutdown_test.rs )"
printf 'tokens it binds:               %s\n' "$( awk '/^fn close_is_idempotent/{f=1} f&&/^\}/{exit} f&&/= shutdown\.close\(\)/{ n++ } END{ print n+0 }' ring_shutdown/tests/shutdown_test.rs )"
printf 'tokens it spends:              %s\n' "$( awk '/^fn close_is_idempotent/{f=1} f&&/^\}/{exit} f&&/\.reopen\(\)/{ n++ } END{ print n+0 }' ring_shutdown/tests/shutdown_test.rs )"
printf 'rows in the premise table:     %s\n' "$( awk '/^### Regenerate/{ exit } /^\| A raw|^\| All /{ n++ } END{ print n+0 }' ring_shutdown/docs/invariant/002_drain_terminates_because_close_preceded_it.md )"
printf 'the row that does not terminate: %s\n' "$( awk '/^### Regenerate/{ exit } /never sees an empty batch/{ sub( /^\| /, "" ); sub( / \|.*/, "" ); print }' ring_shutdown/docs/invariant/002_drain_terminates_because_close_preceded_it.md )"
printf 'the only test reaching that row: %s\n' "$( command grep -o 'fn an_unguarded_producer[a-z_]*' ring_shutdown/tests/shutdown_test.rs )"
printf 'how it obtains a raw producer: %s\n' "$( command grep -o 'guarded\.into_inner()' ring_shutdown/tests/shutdown_test.rs )"
printf 'and whether it then drains:    %s\n' "$( awk '/^fn an_unguarded_producer/{f=1} f&&/^\}/{exit} f&&/drain_all|discard_all/{ n++ } END{ print n+0 }' ring_shutdown/tests/shutdown_test.rs )"
printf 'decision open on that method:  %s\n' "$( command grep -c 'into_inner' ring_shutdown/docs/decisions/001_should_into_inner_exist.md )"
printf 'option rows in that decision:  %s\n' "$( awk '/^### Regenerate/{ exit } /^\| \*\*/{ n++ } END{ print n+0 }' ring_shutdown/docs/decisions/001_should_into_inner_exist.md )"
printf 'of those, naming a test cost: %s\n' "$( awk '/^### Regenerate/{ exit } /^\| \*\*/{ print }' ring_shutdown/docs/decisions/001_should_into_inner_exist.md | command grep -ci 'test' || true )"
```

Live output:

```
how a Stopped is minted:       pub fn close( &self ) -> Stopped< '_ >
how it is spent:               pub fn reopen( self )
what enforcement claims:       
the test that mints two:       close_is_idempotent_and_admit_reports_it()
tokens it binds:               2
tokens it spends:              1
rows in the premise table:     3
the row that does not terminate: A raw `Producer` exists, and is publishing
the only test reaching that row: fn an_unguarded_producer_publishes_straight_through_a_close
how it obtains a raw producer: guarded.into_inner()
and whether it then drains:    0
decision open on that method:  38
option rows in that decision:  4
of those, naming a test cost: 0
```

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_drain_to_empty.md](../algorithm/001_drain_to_empty.md) | The loop this invariant supplies the bound for |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_stopped_proof_token.md](../type/001_stopped_proof_token.md) | The mechanism enforcing the ordering the argument needs |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_close_is_advisory_to_an_unguarded_producer.md](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md) | Row 3 of the premise table, worked out as a trap |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `drain_all_loops_until_the_ring_is_actually_empty` — sufficiency, which is the testable half |
| `tests/shutdown_test.rs` | `discard_all_empties_the_ring_and_counts_what_it_dropped` — the same for the sink-free variant |

### SD23 — The Enforcement Claim Is Stated as an Absolute and the Crate's Own Suite Builds the Counterexample

The Enforcement Mechanism above says *"`Stopped::reopen` takes `self` by value,
so there is none that reaches a drain **after** a reopen either"* — "none"
meaning no expression, anywhere, ever. The first clause is true and the
conclusion does not follow from it.

`Shutdown::close( &self )` takes a shared borrow, so it mints without limit;
`reopen( self )` consumes only the receiver it was called on. Two tokens can
coexist and spending one leaves the other live. `close_is_idempotent_and_admit_reports_it`
does exactly this — it binds two tokens, spends one, and its last three lines
run with `first` still in scope on a ring that is open again. `first.drain_all( … )`
placed there compiles.

What it would then do is the reason this matters. The premise table above says a
drain against a live producer *"never sees an empty batch"*, so the compile that
should have been rejected produces the invariant's own worst outcome — a hang at
teardown with no stack pointing at the omission, which is precisely the failure
the Enforcement section opens by claiming the type system prevents.

This is the same defect [`decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md)
is open about and [`../api/002`](../api/002_the_surface_the_table_does_not_grade.md)
reaches from the other side, and its appearance *here* is the most serious of
the three. A decision record that overstates its options costs a reader some
time. An invariant whose Enforcement Mechanism overstates what is enforced is
the document other documents cite when they stop reasoning — and three of them
do: [`../algorithm/001`](../algorithm/001_drain_to_empty.md) for the loop's
bound, [`../type/001`](../type/001_stopped_proof_token.md) for the token's
purpose, and [`../pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)
for the trap's boundary.

**Disposition:** applied — the Enforcement Mechanism paragraph now names the
qualification instead of the flat "none... either": `close` takes `&self`
and mints without limit, so a token obtained before a `reopen` outlives that
specific reopen, distinct from the token consumed by it. Now prints:
`pub fn close( &self ) -> Stopped< '_ >`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
command grep 'pub fn close(\|pub fn reopen(' src/lib.rs
command grep 'let first = shutdown.close();\|let second = shutdown.close();\|second.reopen();' tests/shutdown_test.rs
```

Live output:

```
  pub fn close( &self ) -> Stopped< '_ >
  pub fn reopen( self )
  let first = shutdown.close();
  let second = shutdown.close();
  second.reopen();
```

### SD24 — The Only Witness for the Non-Terminating Row Is Built From a Method Under Deliberation for Removal, and Stops One Step Short

Row 3 of the premise table — *"A raw `Producer` exists, and is publishing"* —
is the row the whole invariant is about. Exactly one test reaches it:
`an_unguarded_producer_publishes_straight_through_a_close`, whose raw producer
comes from `guarded.into_inner()`.

That method is the subject of an open decision
([`decisions/001`](../decisions/001_should_into_inner_exist.md), which names it
42 times) whose four option rows cost out reasons, guarantees and ergonomics and
mention a test **zero** times. Option 2 — *"Remove it → `Guarded` becomes a
one-way wrapper"* — would delete the only construction in the suite that reaches
this invariant's failing row. That is not an argument against removing it;
`shutdown.guard( producer )` could simply be omitted and the raw producer kept.
It is an argument that a decision proposing to remove a method should say which
tests are written against it, and this one does not.

The second half is the shortfall the test itself carries. It asserts the
*cause* — a raw producer publishes after a close, `try_push` returns `Ok`, the
record arrives — and then reads it back with `try_recv`. It never drains: zero
calls to `drain_all` or `discard_all` appear in its body. So the row is
witnessed up to the point where the documented consequence begins, and the
consequence is left to the prose.

The section above explains why: *"a test that hangs does not report a failure,
it reports nothing"*, and it is right that termination cannot be asserted
directly. But *non*-termination can be bounded and observed — drain a fixed
number of batches against a live producer and assert the ring is still not
empty, which fails loudly in milliseconds and never hangs. Nothing does that,
and the reason given for the gap is a property of the naive test, not of the
claim.
