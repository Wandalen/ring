# Pattern: Retrying Against a Moving Target

### Scope

- **Purpose**: Name the pattern this crate is built on — read a shared value, compute against it, exchange, adopt the winner's value and recompute — and compare its two instantiations in the family.
- **Responsibility**: State the pattern's shape and its one non-obvious requirement, locate every occurrence family-wide, and record that the two occurrences decompose it in opposite directions.
- **In Scope**: The compare-exchange retry pattern in `ring_claim` and `ring_publish`.
- **Out of Scope**: The specific loop mechanics — see [`algorithm/001`](../algorithm/001_the_gate_inside_the_retry.md). The rejected `fetch_add` alternative is [`decisions/001`](../decisions/001_compare_exchange_rather_than_fetch_add.md).

### The Pattern

```
loop:
  observe   the shared value
  decide    whether and what to attempt, *from that observation*
  exchange  the observed value for the computed one
  on failure: adopt the value the exchange actually saw, and go to `decide`
```

The non-obvious requirement is the placement of `decide`. A CAS retry loop that
recomputes only the *new* value on failure, while keeping a decision made
against a stale observation, is the classic defect — the exchange succeeds and
the decision it encodes was made against a world that no longer exists.

This crate encodes the requirement structurally: the decision **is** the loop
condition, so it cannot be hoisted out of the retry by accident
([`algorithm/001`](../algorithm/001_the_gate_inside_the_retry.md)).

```rust
let mut current = self.claimed();
while count <= self.consumers.headroom( current )   // ← `decide`, re-evaluated per attempt
{
  let next = current.advanced_by( count as u64 );
  match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
  {
    Ok( _ ) => return Ok( Claim::new( current, count ) ),
    Err( actual ) => current = actual,               // ← adopt, then re-`decide`
  }
}
Err( RingError::Full )
```

The test file's header names the corresponding defect directly — "the failure a
naive CAS retry produces when it recomputes the range but not the start"
(`tests/claim_test.rs:19-21`) — and assigns
`claims_under_contention_lose_no_sequences` to it.

### CL39 — The Family Instantiates This Twice, Decomposed in Opposite Directions

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
for f in ring_*/src/*.rs; do
  n=$( grep -vE "^[[:space:]]*//" "$f" | grep -cE 'compare_exchange' )
  [ "$n" -gt 0 ] && printf '%-34s %s\n' "${f#ring/}" "$n"
done
command grep -rn -B3 'compare_exchange' ring_*/src/*.rs | command grep -E 'while|loop' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
ring_atomic/src/lib.rs             14
ring_claim/src/lib.rs              2
ring_cursor/src/lib.rs             2
ring_publish/src/lib.rs            1
ring_claim/src/lib.rs-437-    while count <= self.consumers.headroom( current )
ring_claim/src/lib.rs-488-    while let granted @ 1.. = max.min( self.consumers.headroom( current ) )
```

| Crate | `compare_exchange` sites | Wrapped in a retry |
|-------|-------------------------:|:------------------:|
| `ring_atomic` | 14 | — these *are* the primitive |
| `ring_cursor` | 2 | — forwarding impls |
| `ring_publish` | 1 | in the **caller-facing wrapper** |
| **`ring_claim`** | **2** | **both, inside the function** |

Only two crates apply the pattern rather than provide it, and they split it at
opposite seams:

| | `ring_claim::claim` | `ring_publish::try_publish` / `publish` |
|--|---------------------|------------------------------------------|
| Single-attempt entry point | **none** | `try_publish` — one CAS, no loop |
| Looping entry point | `claim` — loop inside | `publish` — loops over `try_publish` |
| What the failure arm carries | consumed internally | **returned to the caller** as `Err( Seq )` |
| Loop can give up | ✔ — `Err( Full )` when the gate refuses | ✘ — spins until success |
| Backoff hint | none | `core::hint::spin_loop()` |
| Retry is bounded by | other producers succeeding | the predecessor finishing its write |

`try_publish`'s error type is the tell (`ring_publish:141-146`):

> The current published position, when it is not `start` — meaning some earlier
> claim has not been published yet. **Deliberately not a `RingError`: this is
> not a failure, it is `compare_exchange`'s "try again", and the value returned
> is what to try against next.**

That is the retry pattern turned inside out and handed to the caller: the
observed value, the thing you must adopt before deciding again, promoted to the
function's return type. `ring_claim` does the opposite — the value the exchange
actually saw is bound to `actual`, consumed by the next iteration, and never
leaves the function.

The asymmetry is worth stating because each crate's stated argument would
support the other's choice. `ring_claim`'s thesis is caller-controlled
composition (`:25-35`):

> Every function here returns immediately … it is what lets the same primitive
> serve a spinning producer, a parking producer, and the tick path that must not
> block at all.

That argument is *for* a `try_claim`. And `ring_publish`'s argument for spinning
without a budget — the predecessor is committed and cannot abandon — is an
argument that its caller does **not** need to control the retry, yet it exposes
`try_publish` anyway.

So the crate that argues hardest for handing control to the caller hides its
retry loop, and the crate that argues its retry is safe to hide exposes a
single-attempt variant. Neither is wrong: `claim`'s retry never waits on a
consumer, so hiding it does not compromise `WaitKind::None`
([`non_functional_requirement/001`](../non_functional_requirement/001_nothing_allocates_nothing_waits_nothing_is_unsafe.md)).
But the two crates reached opposite API shapes from arguments that do not
distinguish them, and nothing in either records the comparison.

### CL40 — The Hidden Retry Is Why One Error Covers Two Situations

The decomposition has a concrete consequence, and it is the crate's one
observable API cost. Because the loop is internal, both of these produce the
identical value:

| Situation | Attempts made | Returned |
|-----------|--------------:|----------|
| the ring is full; the gate refuses on the first evaluation | 0 exchanges | `Err( RingError::Full )` |
| lost the exchange 40 times, then the gate refused | 40 exchanges | `Err( RingError::Full )` |

A caller cannot tell contention from back-pressure. The two call for opposite
responses — back off and wait for a consumer, versus retry immediately because
producers are making progress — and the crate that exists to serve "a spinning
producer, a parking producer, and the tick path" gives all three the same
answer.

`ring_publish` has no equivalent ambiguity, precisely because it exposes the
single attempt: a `try_publish` failure is unambiguously "not your turn", and
the returned `Seq` says whose turn it is.

What this is not: a defect requiring a fix. The information has a cost to
carry — a retry count in the return type, or a `try_claim` whose `Err` carries
the observed cursor, either of which widens `Result< Claim, RingError >` past
its current 24 bytes and the free niche it currently occupies
([`api/001`](../api/001_seventeen_items_and_nothing_that_drops_silently.md)).
And no caller has asked: `claim` has one external caller
([`item/002`](../item/002_the_seven_of_the_claimer.md)) and it retries on `Full`
without distinguishing.

What it is: a design consequence that follows from the decomposition, is
invisible from the signature, and is recorded nowhere in the crate.

### Where the Pattern Is Deliberately Absent

| Place a retry could have gone | Why it is not there |
|-------------------------------|---------------------|
| around `GatingSet::check` | the check is not an exchange; retrying it is the rejected design ([`integration/002`](../integration/002_four_predicates_and_the_one_that_is_called.md)) |
| around `headroom()` at the call site | same shape, now outside the crate where nothing can fix it |
| inside `ring_batch::claim_gated` | it uses `fetch_add`, which cannot fail and therefore cannot retry — that is exactly what makes it the rejected design |

The third row is the pattern's negative definition: `fetch_add` has no failure
arm, so there is no place for `decide` to be re-evaluated. The pattern requires
an operation that can *fail and tell you what it saw*, which is the one thing
compare-exchange offers that fetch-and-add does not.

### Patterns

| File | Relationship |
|------|--------------|
| [002_the_half_open_range_as_a_value.md](002_the_half_open_range_as_a_value.md) | What the successful attempt returns |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_gate_inside_the_retry.md](../algorithm/001_the_gate_inside_the_retry.md) | This pattern's mechanics in this crate |
| [../algorithm/002_two_loops_that_disagree_at_zero.md](../algorithm/002_two_loops_that_disagree_at_zero.md) | The second instantiation, and where it differs |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_compare_exchange_rather_than_fetch_add.md](../decisions/001_compare_exchange_rather_than_fetch_add.md) | Why an operation that can fail is required |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependents_that_split_one_feature.md](../integration/001_two_dependents_that_split_one_feature.md) | The two crates compared here |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_range_from_grant_to_publication.md](../lifecycle/001_a_range_from_grant_to_publication.md) | What `publish`'s unbounded spin rests on |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:422-437` | The pattern, with `decide` as the loop condition |
| `ring_claim/src/lib.rs:483-488` | The second instantiation |
| `ring_publish/src/lib.rs:141-166` | `try_publish` — the single attempt, error carrying the observation |
| `ring_publish/src/lib.rs:200-210` | `publish` — the retry, in the wrapper |
| `ring_batch/src/lib.rs:306-329` | The shape with no failure arm, and therefore no retry |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:356` — `claims_under_contention_lose_no_sequences` | The recompute-the-range-but-not-the-start defect |
| `tests/claim_test.rs:320` — `no_two_producers_are_ever_granted_the_same_sequence` | That the exchange linearises the decision |
| `ring_publish/tests/handshake_test.rs` | The other instantiation, under `loom` |
