# Decision: Compare-Exchange Rather Than `fetch_add`

### Scope

- **Purpose**: Record the crate's founding decision — that the cursor advances by compare-exchange with the gate inside the retry, not by `fetch_add` behind a prior check — state precisely what the rejected design breaks, and record that the family ships the rejected design in another crate.
- **Responsibility**: Reproduce the argument, correct the common misreading of what `fetch_add` claiming actually costs, locate `ring_batch::claim_gated`, and describe the mutation check that makes the argument evidence.
- **In Scope**: Both designs, their failure modes, and every place in the family each appears.
- **Out of Scope**: The loop's mechanics — see [`algorithm/001`](../algorithm/001_the_gate_inside_the_retry.md).

### The Decision

`src/lib.rs:52-64`:

> A `fetch_add` claim is shorter and wrong. It advances the cursor
> unconditionally, so the gating check has to happen *before* it — and between
> that check and the add, another producer can take the space the check just
> found. The result is a producer holding a range that overlaps a slot a
> consumer is still reading, which no later check can undo because the range is
> already granted.
>
> The compare-exchange loop re-reads the gate inside the retry, so the decision
> to grant and the granting itself are one atomic step.

| | `fetch_add` | `compare_exchange` loop |
|--|-------------|-------------------------|
| Instructions on the success path | 1 | 1 |
| Can fail | no | yes — retried |
| Gate consulted | before, separately | inside the retry, per attempt |
| Grant and decision | two steps | one step |
| Producers get distinct ranges | **yes, both** | yes |
| Grant can pass the consumer limit | **yes** | no |

### CL19 — The Rejected Design Fails Against the *Consumer*, Not Against Other Producers

This is the part most readings get backwards, and `tests/manual/readme.md § C1`
states it explicitly:

> `fetch_add` produces no duplicate grants (the add is atomic, so every producer
> still gets a distinct start), so the *only* symptom is a grant that runs past
> the gate.

A `fetch_add` is atomic. Two producers calling it get 0..4 and 4..8, never
0..4 twice. Exclusivity *between producers* survives the rejected design
intact.

What does not survive is the gate. Both producers check headroom, both see four
slots free, both add four, and the cursor lands eight past a consumer that has
released four. The second producer's range covers slots the consumer has not
finished reading — so the overlap is producer-against-consumer, and it manifests
as torn or overwritten records rather than as two producers writing the same
slot.

That distinction changes what a test must look for. A test asserting "every
sequence is granted exactly once" passes against the broken implementation. The
test that catches it is `no_grant_ever_passes_the_limit_under_contention`,
which asserts a relationship between the grant and `GatingSet::limit` — and it
is the only test in the file that does.

### CL20 — The Family Ships the Rejected Design, in `ring_batch`

```sh
cd "$(git rev-parse --show-toplevel)"
# The single-producer clause this finding asked for, in `claim_gated`'s own doc
# comment. Anchored on the heading rather than on a line address, so an edit
# above it cannot silently re-target this recipe at a different section.
command grep -m1 -A7 -F '/// # One producer only' ring_batch/src/lib.rs
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -m1 -A23 -F 'pub fn claim_gated< P : SeqCell, C : SeqCell >' ring_batch/src/lib.rs
command grep -r 'claim_gated' */src/*.rs */tests/*.rs
```

Live output:

```
/// # One producer only
///
/// The gate and the advance are two separate operations and nothing holds the
/// cursor still between them: a second producer can take the free slots this
/// call just counted, before this call's `fetch_add` runs, and both claims are
/// then granted past the limit. Safe for a single producer, racy for several —
/// and the restriction is a real one rather than a caveat, because the shape
/// that makes it racy is the shape `ring_claim` exists to reject.
pub fn claim_gated< P : SeqCell, C : SeqCell >
(
  producer : &P,
  consumer : &C,
  count : usize,
  capacity : Capacity,
  order : Ordering,
)
-> Result< BatchClaim, RingError >
{
  if count > capacity.get()
  {
    return Err( RingError::BatchTooLarge { requested : count, capacity : capacity.get() } );
  }

  let at = producer.load( Ordering::Acquire );
  let behind = consumer.load( Ordering::Acquire );
  if ( free_slots( at, behind, capacity ) as usize ) < count
  {
    return Err( RingError::Full );
  }

  Ok( claim( producer, count, order ) )
}
ring_batch/src/lib.rs:/// can outrun the ring; [`claim_gated`] checks first — exactly for a single
ring_batch/src/lib.rs:/// use ring_batch::claim_gated;
ring_batch/src/lib.rs:/// let batch = claim_gated( &producer, &consumer, 8, capacity, Ordering::AcqRel ).unwrap();
ring_batch/src/lib.rs:///   claim_gated( &producer, &consumer, 1, capacity, Ordering::AcqRel ),
ring_batch/src/lib.rs:///   claim_gated( &producer, &consumer, 9, capacity, Ordering::AcqRel ),
ring_batch/src/lib.rs:pub fn claim_gated< P : SeqCell, C : SeqCell >
ring_cursor/src/lib.rs://! rather than take a parameter — the same choice `ring_batch::claim_gated`
ring_batch/tests/batch_test.rs://! `claim_gated` must distinguish "ask for less" from "wait". Collapsing both
ring_batch/tests/batch_test.rs:use ring_batch::{ claim, claim_gated, drain_order, BatchClaim };
ring_batch/tests/batch_test.rs:  let batch = claim_gated( &producer, &consumer, 8, capacity, Ordering::AcqRel )
ring_batch/tests/batch_test.rs:  claim_gated( &producer, &consumer, 8, capacity, Ordering::AcqRel ).unwrap();
ring_batch/tests/batch_test.rs:    claim_gated( &producer, &consumer, 1, capacity, Ordering::AcqRel ),
ring_batch/tests/batch_test.rs:  let outcome = claim_gated( &producer, &consumer, 9, capacity, Ordering::AcqRel );
ring_batch/tests/batch_test.rs:  assert!( claim_gated( &producer, &consumer, 5, capacity, Ordering::AcqRel ).is_err() );
ring_batch/tests/batch_test.rs:  claim_gated( &producer, &consumer, 4, capacity, Ordering::AcqRel ).unwrap();
ring_batch/tests/batch_test.rs:    claim_gated( &producer, &consumer, 1, capacity, Ordering::AcqRel ),
ring_batch/tests/batch_test.rs:  let batch = claim_gated( &producer, &consumer, 2, capacity, Ordering::AcqRel )
ring_batch/tests/batch_test.rs:    claim_gated( &producer, &consumer, 1, capacity, Ordering::AcqRel ),
ring_batch/tests/batch_test.rs:  claim_gated( &producer, &consumer, 2, capacity, Ordering::AcqRel ).unwrap();
ring_batch/tests/batch_test.rs:  let batch = claim_gated( &producer, &consumer, 0, capacity, Ordering::AcqRel )
```

`ring_batch::claim_gated` (`:306-329`), a public function in a Tier 2 crate:

```rust
if count > capacity.get()
{
  return Err( RingError::BatchTooLarge { requested : count, capacity : capacity.get() } );
}

let at = producer.load( Ordering::Acquire );
let behind = consumer.load( Ordering::Acquire );
if ( free_slots( at, behind, capacity ) as usize ) < count
{
  return Err( RingError::Full );
}

Ok( claim( producer, count, order ) )   // → BatchClaim::new( cursor.fetch_add( … ), count )
```

Guard, gate, then `fetch_add`. It is the rejected design, line for line, with
the same two error variants in the same order. And its stated contract has no
producer-count restriction:

| Function | Documented restriction |
|----------|------------------------|
| `ring_batch::claim` (`:181-206`) | "Performs **no gating**. A claim taken without consulting a consumer barrier can outrun the ring; [`claim_gated`] is the form that checks first." |
| `ring_batch::claim_gated` (`:212-282`) | "reads the slowest consumer's position, checks that `count` slots are free, and only then advances the cursor" — followed now by a `# One producer only` section that was absent when this was written |

The ungated form was always honest: it says it can outrun the ring and names the
gated form as the fix. The gated form is the one that does not hold under two
producers, and until this finding was dispositioned nothing in its documentation
said so.

Two facts keep this from being a live defect today:

1. **`claim_gated` has zero callers.** Not in any library, not in any test
   outside `ring_batch`'s own. The one crate that depends on `ring_batch` is
   `ring_tls`, and `ring_tls:44` imports the *ungated* `claim`.
2. **The thread-local path is single-producer by construction.**
   `ring_tls`'s whole design is one buffer per thread, so the caller that
   exists cannot race itself.

So the family contains both designs, each correct in its own crate's intended
use, and for as long as this finding stood open only one of them documented the
boundary. The gap was one sentence in `ring_batch`: `claim_gated` is safe for a
single producer and racy for several, and the crate that proves why is
`ring_claim`, two tiers up.

The argument still belongs here rather than only in `ring_batch`, because this
is where it is made. `ring_claim`'s module documentation is the family's
statement of *why* check-then-add is wrong, and it names no crate — so the
sentence added to `ring_batch` has to carry the pointer back, which is what its
closing paragraph does.

**Disposition:** applied — `claim_gated` now carries a `# One producer only`
section in its own doc comment, above the `# Errors` section, so the restriction
reaches `cargo doc` rather than living only in a consumer crate's corpus. It
names the mechanism rather than asserting the conclusion: the gate and the
advance are two separate operations, nothing holds the cursor still between them,
so a second producer can take the free slots this call just counted before its
`fetch_add` runs and both claims are granted past the limit. It then points at
`ring_claim::Claimer::claim` as the multi-producer form and says what that form
does differently — the second guard moved inside a compare-exchange retry, so the
space is re-checked against the value it is about to exchange against. Nothing in
either crate's behaviour changed; `claim_gated` is still correct for the one
caller shape it has. Now prints: `/// # One producer only`

### The Check That Makes the Argument Evidence

`tests/manual/readme.md § C1` is a mutation check, and it is the only one in the
family that runs the test suite against a deliberately broken implementation
rather than reading source:

| Step | What it does |
|------|--------------|
| 1 | copy `src/lib.rs` aside |
| 2 | replace `claim`'s CAS loop with the check-then-`fetch_add` form |
| 3 | run `no_grant_ever_passes_the_limit_under_contention` **five times** |
| 4 | restore, and run the full suite |

**Expected: `exit 100` on every mutated run, then a clean 28/28.** The
five-run requirement is the point — a concurrency test that fails four times in
five is a test a real regression can ship past. `§ C1` records the measurement:

> measured at 4/5 before the consumer thread gained its `yield_now`, and 8/8
> after, which is why that yield is load-bearing rather than cosmetic.

A `yield_now` in a test is normally noise. Here it is the difference between a
check that catches the crate's founding defect every time and one that catches
it 80% of the time, and the manual plan says so with the numbers that motivated
it.

`§ C2` is the cheap structural companion: the comment-filtered source must
contain exactly two atomic mutations, both `compare_exchange`, with no
`fetch_add` and no `store`.

### What Was Not Chosen

| Alternative | Why not |
|-------------|---------|
| `fetch_add` after a check | this decision — grants past the gate |
| `fetch_add` then rewind on failure | rewinding hands out sequences twice (`:44-49`) |
| a lock around check-and-add | correct, and forces a wait strategy into a crate that must not have one (`:25-35`) |
| per-slot stamps instead of a claim cursor | `ring_mpsc`'s choice for *publication*; exclusivity still needs the cursor |

The third is the one worth pausing on: a mutex would be correct and simple, and
it is refused for a reason unrelated to performance. A claim that can block
makes `WaitKind::None` unimplementable above this crate, so the tick path — the
one that must never block — would have no primitive to call.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_gate_inside_the_retry.md](../algorithm/001_the_gate_inside_the_retry.md) | The loop this decision produced |

### Decisions

| File | Relationship |
|------|--------------|
| [002_must_use_without_drop.md](002_must_use_without_drop.md) | The other refusal — a guard the family built twice elsewhere |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_four_predicates_and_the_one_that_is_called.md](../integration/002_four_predicates_and_the_one_that_is_called.md) | `GatingSet::check`, the same shape in a third crate, uncalled |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_no_two_producers_hold_one_sequence.md](../invariant/001_no_two_producers_hold_one_sequence.md) | The property the rejected design *does* preserve |
| [../invariant/002_a_refused_claim_moves_nothing.md](../invariant/002_a_refused_claim_moves_nothing.md) | The property it does not |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_half_open_range_as_a_value.md](../pattern/002_the_half_open_range_as_a_value.md) | `BatchClaim`, this crate's range type built the rejected way |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:25-35` | Why a lock is refused |
| `ring_claim/src/lib.rs:44-49` | Why rewinding is refused |
| `ring_claim/src/lib.rs:52-62` | The decision, in full |
| `ring_batch/src/lib.rs:192-199` | The ungated form, and its honest warning |
| `ring_batch/src/lib.rs:306-329` | `claim_gated` — the rejected design, public, uncalled |
| `ring_tls/src/lib.rs:44` | The one dependent, importing the ungated form |
| `ring_gating/src/lib.rs:283-294` | The same guards again, in a third crate |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:476` — `no_grant_ever_passes_the_limit_under_contention` | The only test that catches the rejected design |
| `tests/manual/readme.md § C1` | The mutation, the five runs, and the `yield_now` measurement |
| `tests/manual/readme.md § C2` | Exactly two `compare_exchange`, no `fetch_add`, no `store` |
