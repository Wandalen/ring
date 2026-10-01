# decisions

Two refusals, and they have the same root. The cursor cannot go backwards — so
the claim cannot be granted before the gate is re-read, and a granted claim
cannot be given back. Everything distinctive about this crate follows from that
one property of a monotonic shared counter.

The first file is the founding decision: compare-exchange with the gate inside
the retry, rather than the shorter `fetch_add` behind a prior check. It corrects
the usual reading of what the rejected design breaks — exclusivity between
producers survives it intact; what fails is the gate — and locates the rejected
design shipping, publicly and uncalled, in a Tier 2 crate two tiers down.

The second is the refusal of a destructor. `Claim` carries the family's most
severe `must_use` message and the only one with no runtime mechanism behind it,
in a family that built the guard shape twice and wrote down an argument for it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Compare-Exchange Rather Than `fetch_add`](001_compare_exchange_rather_than_fetch_add.md) | CL19, CL20 — what the rejected design actually breaks, `ring_batch::claim_gated` shipping it, the mutation check that makes the argument evidence, and the three alternatives that were not chosen |
| 002 | [`must_use` Without `Drop`](002_must_use_without_drop.md) | CL21, CL22 — twelve messaged annotations and the ten nothing backs up, the family's four destructors and the two crates holding all of them, and the unwind case where a lint offers nothing |

### The Two Refusals

| | Refused | Because | Cost |
|--|---------|---------|------|
| 001 | `fetch_add` behind a prior check | a grant decided before the exchange can pass the gate | a retry loop, and a mutation check to prove it matters |
| 002 | a `Drop` that releases the claim | rewinding the cursor hands out sequences twice | the strongest warning in the family, enforced by a lint |

Both are refusals of something that would compile, look reasonable, and be
adopted by a reader following the family's own precedent — `GatingSet::check`
exists and is shaped for the first, `ring_spsc::Reservation` exists and argues
for the second. That is why both have a manual check pointed at them.

### What Each Refusal Costs, Concretely

| | If the refusal were reversed |
|--|------------------------------|
| 001 → `fetch_add` | producers still get distinct ranges; a grant runs past the consumer limit and overwrites unread slots |
| 002 → `Drop` releases | the cursor rewinds over a range a later producer already holds; two producers write the same slot |

The second is the worse failure and the more tempting change, which is why
`§ C4`'s expected output is *no output at all* and the plan spells out that a
`Drop` here "would look like careful resource handling and would be a
correctness bug."

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# the cursor is only ever moved by compare-exchange — never fetch_add, never store
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs \
  | grep -E "fetch_add|compare_exchange|\.store\("

# the rejected design, where it ships, and the restriction CL20 put on it
command grep -m1 -A7 -F '/// # One producer only' ring_batch/src/lib.rs
command grep -m1 -A23 -F 'pub fn claim_gated< P : SeqCell, C : SeqCell >' ring_batch/src/lib.rs
command grep -r 'claim_gated' */src/*.rs */tests/*.rs

# every messaged must_use, and every Drop impl, family-wide
command grep -r '#\[ must_use = ' ring_*/src/*.rs
command grep -r '^impl.*Drop for' ring_*/src/*.rs

# this crate must produce nothing from the second — reported as a count, because
# `grep` exits 1 on no match and the expected answer here is no match
printf '  Drop impls in ring_claim outside comments: %s\n' \
  "$( grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -cE "impl.*Drop" )"
```

Live output:

```
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
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
ring_atomic/src/lib.rs:  #[ must_use = "the returned sequence is the claim — dropping it claims a range nobody will use" ]
ring_claim/src/lib.rs:#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
ring_flush/src/lib.rs:#[ must_use = "an ignored outcome is exactly how a misconfigured OnBarrier stays silent" ]
ring_shutdown/src/lib.rs:  #[ must_use = "a Stopped is the only route to a drain; bind it, or bind `_` to close and nothing else" ]
ring_shutdown/src/lib.rs:  #[ must_use = "this is the record itself, not a copy — dropping it loses it" ]
ring_shutdown/src/lib.rs:#[ must_use = "a Wake::Closed means stop, not publish" ]
ring_slot/src/lib.rs:  #[ must_use = "this is the only way a payload leaves the slot; dropping it here destroys the record" ]
ring_spsc/src/lib.rs:#[ must_use = "a reservation publishes on drop; dropping it immediately publishes an unwritten slot" ]
ring_spsc/src/lib.rs:#[ must_use = "a batch commits on drop; dropping it immediately discards the records it covers" ]
ring_testkit/src/lib.rs:#[ must_use = "nothing frees this — dropping the reference leaks the ring with no way to reach it again" ]
ring_testkit/src/lib.rs:#[ must_use = "nothing frees either allocation — dropping the pair leaks both with no way to reach them again" ]
ring_testkit/src/lib.rs:  #[ must_use = "the Outcome is the measurement — `script.run( &mut ring );` as a statement drives the ring and discards everything it observed" ]
ring_mpsc/src/lib.rs:impl< S > Drop for Reserved< '_, S >
ring_mpsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Reservation< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
  Drop impls in ring_claim outside comments: 0
```

| | Value |
|--|------:|
| Atomic mutations in the crate | 2 — both `compare_exchange` |
| `fetch_add` / `store` in the crate | **0** |
| Instructions on the success path, either design | 1 |
| Producers granted overlapping ranges by the rejected design | **0** — exclusivity survives it |
| Tests that catch the rejected design | **1** |
| Mutation runs `§ C1` requires to pass | 5, all failing |
| …measured sensitivity before the test's `yield_now` | 4/5 |
| …after | 8/8 |
| Public functions in the family shipping the rejected design | **1** — `ring_batch::claim_gated` |
| …with a documented producer-count restriction | **1** — since CL20 |
| …with any caller | **0** |
| `#[ must_use ]` attributes, family-wide | 281 |
| …carrying a message | 12 |
| …that warn about dropping | 3 |
| …whose type has a `Drop` that acts | 2 |
| `Drop` impls, family-wide | 4 |
| …in a Tier 5 primitive | **0** |
| …in this crate | **0** |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL19 | `ring_claim` | n/a — doc gap | The rejected `fetch_add` design does *not* hand out overlapping ranges between producers; the add is atomic, so every producer gets a distinct start, and the only symptom is a grant that runs past the gate into slots a consumer has not released |
| CL20 | `ring_batch` | **latent hazard** | `ring_batch::claim_gated` is the rejected design line for line — guard, gate, `fetch_add` — public and uncalled, and its documentation carried no producer-count restriction until a `# One producer only` section was added; the argument for why the shape is wrong still lives two tiers up, so that section has to carry the pointer back |
| CL21 | `ring_claim` | n/a — unenforced | Ten of the family's twelve messaged `must_use` annotations have no runtime mechanism behind them; `Claim`'s and `ring_atomic`'s name the same permanent consequence — a range stranded, every consumer stalled — and neither can carry a destructor; `let _ = …` silences the lint and nothing else happens, ever |
| CL22 | family | n/a — observation | All five `Drop` impls in the family are in `ring_mpsc` and `ring_spsc`, on borrow-carrying guard types, and none is in a Tier 5 primitive; `ring_spsc`'s `Producer::claim` argues for the guard shape by naming "a bare `claim`/`publish` pair" — which is this crate |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| That changes what a test must assert: "every sequence granted exactly once" passes against the broken implementation, and only the one test comparing a grant against `GatingSet::limit` fails | [001](001_compare_exchange_rather_than_fetch_add.md) |
| `§ C1` is the family's only manual check that runs the suite against a deliberately broken implementation, and it records the measurement that made a test's `yield_now` load-bearing rather than cosmetic | [001](001_compare_exchange_rather_than_fetch_add.md) |
| A mutex around check-and-add would be correct and is refused for a non-performance reason: a claim that can block makes `WaitKind::None` unimplementable above this crate | [001](001_compare_exchange_rather_than_fetch_add.md) |
| A dropped guard costs one empty record, an observable defined outcome; a dropped `Claim` costs a permanent stall — the whole difference between a claim held by a type that can reach the ring and one held by two integers | [002](002_must_use_without_drop.md) |
| `#[ must_use ]` and `Drop` are different enforcement classes, not different ergonomics: a lint is silenceable by the idiom that also destroys the value, and a destructor is not | [002](002_must_use_without_drop.md) |

