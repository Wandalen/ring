# pitfall

Two ways to use this crate wrongly without being told. One is catastrophic and
essentially unguarded; the other is trivial and merely undocumented. They are
here together because they fail the same way — the code is right, the
documentation is silent, and the reasoning that would have prevented both is
already written down somewhere a caller never reads.

The crate is three comparisons and a compare-exchange, so there is not much
surface to misuse. What there is divides cleanly: you can take a range and never
give it back ([001](001_dropping_a_claim.md)), or you can ask for zero and get an
answer that contradicts the sibling method ([002](002_claiming_zero.md)).
Everything else the type system handles — a `Claim` cannot be forged with a bad
width, a `Claimer` cannot be used after its gating set, and no method takes
`&mut self`, so there is no aliasing mistake available to make.

Both files reach the same structural conclusion by different routes, and it is
the one worth carrying out of this definition: **this crate's guards are aimed
at the cases that do not happen.** The `must_use` fires on a bare discarded
temporary and on none of the four routes a real producer takes. The `# Errors`
sections document the conditions a caller will meet least often and omit the two
they will meet first. Nothing is wrong; the aim is off.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Dropping a Claim](001_dropping_a_claim.md) | CL43, CL44 — four routes to a stranded range, zero warnings on all four, and a wedge that surfaces in another thread as a different error |
| 002 | [Claiming Zero](002_claiming_zero.md) | CL45, CL46 — `claim( 0 )` succeeds on a full ring, `claim_up_to( 0 )` fails on an empty one, and the documented `Full` condition is false for the second |

### The Two, Side by Side

| | 001 — dropping a claim | 002 — claiming zero |
|--|------------------------|---------------------|
| Cost when hit | the whole ring, permanently | one spurious retry loop |
| How often | rare — needs an early return or unwind | first thing a batching caller writes |
| What guards it | `#[ must_use ]`, which misses all four routes | nothing |
| Where the reasoning lives | nowhere — no test can reach it | `claim_test.rs:224` and `:249`, in an `expect` message and a comment |
| How it surfaces | a hang, then an ordinary-looking `Full` | an immediate, ordinary-looking `Full` |
| Fix | a stall detector, or a tier that permits a guard | two clauses of prose |

The last row is why they sit at opposite ends of this definition. 002 is fixable
by writing two sentences that already exist elsewhere in the repository. 001 is
not fixable inside this crate at all — the guard shape needs a ring to borrow and
Tier 5 has none ([`decisions/002`](../decisions/002_must_use_without_drop.md)) —
so the honest remedy is detection rather than prevention, and detection belongs
to a diagnostic crate.

### `Full` Means Three Things

Both files end up at the same overloaded value, and combined with
[`pattern/001`](../pattern/001_retrying_against_a_moving_target.md) § CL40 the
count is three:

| `Err( RingError::Full )` returned when | Correct response | Documented as |
|---------------------------------------|------------------|---------------|
| the gate refuses on the first evaluation — genuine back-pressure | wait for a consumer | ✔ *"a retry loop should keep going"* |
| the gate refused after losing *k* exchanges — contention | retry immediately | ✘ same text |
| `claim_up_to( 0 )` — the caller asked for nothing | fix the call site | ✘ same text |
| a claim was stranded and the ring is dead | stop; nothing will free | ✘ same text — and retrying is now the failure |

One value, four situations, four different correct responses, and one piece of
advice covering all of them. The advice is right for the first and actively wrong
for the last. Widening `RingError` is not obviously the answer — it is a
`ring_types` enum shared across 33 crates
([`type/001`](../type/001_a_seq_a_usize_and_three_casts.md)) — but nothing in the
crate records that the variant is carrying four meanings.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# what the must_use actually catches: bind, discard, unwind, or skip
grep 'must_use' ring_claim/src/lib.rs

# every mention of zero in the source a caller reads
grep 'zero\|Zero' ring_claim/src/lib.rs

# the two contracts, and what neither says
sed -n '/^  \/\/\/ Claim exactly `count` contiguous sequences, or fail\.$/,/^  \/\/\/ back-pressure, so a retry loop should keep going\.$/p;/^  \/\/\/ Claim as many of `max` sequences as are available, down to one\.$/,/^  \/\/\/ simply more than will be granted\.$/p' ring_claim/src/lib.rs

# every route to Full
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep 'RingError::Full'

# why no test can reach the stranded state
sed -n '/dependencies/,$p' ring_claim/Cargo.toml
grep -c 'publish' ring_claim/tests/claim_test.rs
```

Live output:

```
//! ## Why a claim is `#[must_use]` and carries no destructor
//! thing `docs/feature/172` forbids outright. The type is `#[must_use]` so the
#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
  // No `#[ must_use ]` here: `Claim` itself already carries one *with a
  #[ must_use ]
  #[ must_use ]
  #[ must_use ]
  #[ must_use ]
  #[ must_use ]
  #[ must_use ]
  #[ must_use ]
  #[ must_use ]
  #[ must_use ]
  #[ must_use ]
  #[ must_use ]
  /// A claimer starting at sequence zero, gated by `consumers`.
  /// A `count` of zero always succeeds, even on a full ring — there is
  /// nothing for back-pressure to block. [`claim_up_to`] treats a zero grant
  /// A `max` of zero is also `Full`, since there is no partial success at
  /// zero to report — this differs from [`claim`], which treats a `count`
  /// of zero as always satisfiable.
    // zero — no room, or a `max` of zero — exits to the `Full` below.
  /// Claim exactly `count` contiguous sequences, or fail.
  ///
  /// Never waits and never claims fewer than asked — see
  /// [`claim_up_to`] for the partial variant.
  ///
  /// [`claim_up_to`]: Self::claim_up_to
  ///
  /// # Errors
  ///
  /// [`RingError::BatchTooLarge`] when `count` exceeds the ring's capacity: a
  /// configuration error no consumer's progress can fix, so a retry loop must
  /// stop. [`RingError::Full`] when the space is not available *right now* —
  /// back-pressure, so a retry loop should keep going.
  /// Claim as many of `max` sequences as are available, down to one.
  ///
  /// For a batching producer that would rather write four items now than wait
  /// for room for eight.
  ///
  /// # Errors
  ///
  /// [`RingError::Full`] when not even one slot is free. Never
  /// `BatchTooLarge` — a `max` wider than the ring is not an error here, it is
  /// simply more than will be granted.
    Err( RingError::Full )
    Err( RingError::Full )
[dependencies]
ring_types = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }
ring_gating = { path = "../ring_gating" }

[lints]
workspace = true
8
```

| | Value |
|--|------:|
| Routes to a stranded claim | **4** |
| …that produce a compiler warning | **0** |
| Call shapes that do warn | 3 |
| …that print `Claim`'s own message | 2 |
| …that a producer would plausibly write | **0** |
| `rustc` suggestions that silence the warning and strand the claim | **1** |
| Dependencies on `ring_publish`, normal or dev | **0** |
| Tests that publish anything | **0** |
| Tests covering any strand route | **0** |
| Mentions of "zero" in the source | 2 |
| …in either method's documented contract | **0** |
| …in a `///` comment, about the starting cursor rather than the claim | 1 |
| …in a stripped `//` implementation comment | 1 |
| Methods answering `0` with `Ok` | 1 |
| Methods answering `0` with `Err( Full )` | 1 |
| Atomic RMWs issued by `claim( 0 )` | **1** |
| …by `claim_up_to( 0 )` | **0** |
| `Err( RingError::Full )` return sites in the source | 2 |
| Distinct situations they report | **3** |
| …the documented advice is correct for | **1** |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL43 | `ring_claim` | **latent hazard** | All four routes to a stranded claim (explicit discard, `?` early return, unwind, loop skip) compile with **zero warnings**; `#[ must_use ]` fires only on an unused value, and binding the claim satisfies it immediately, before any of the four routes begin |
| CL44 | `ring_claim` | **latent hazard** | The producer that drops its claim never fails; `claim` keeps returning `Ok`, `claimed()` keeps advancing and `headroom()` keeps reporting a healthy number, while the published frontier is pinned forever at the stranded sequence, so the thread that hangs is a later one that did nothing wrong |
| CL45 | `ring_claim` | n/a — doc gap | `claim( 0 )` succeeds on a completely full ring and is the only call that does; `claim_up_to( 0 )` fails on a completely empty one and is the only call that does; both behaviours are deliberate and correct against their contracts, and both are explained only in `claim_test.rs` |
| CL46 | `ring_claim` | **wrong doc** | `claim_up_to`'s `# Errors` states `Full` means *"not even one slot is free"*, which is false for `max = 0` on an empty ring; the natural drain loop that trusts it backs off forever waiting for a consumer to free space in a ring that is already empty |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| On the one call shape that does warn — `claimer.claim( 4 );` — the note printed is `Result`'s stock *"may be an `Err` variant"*, not `Claim`'s crafted sentence, because `Result` is the outer type and its own `must_use` wins; the message is reachable only via `Claim::new( … );` or `claim( … ).unwrap();` as bare statements | [001](001_dropping_a_claim.md) |
| `rustc`'s own `help:` on that warning suggests `let _ = …`, which is route 1 — the compiler's remedy silences the only signal and strands the claim | [001](001_dropping_a_claim.md) |
| Once the ring fills behind the strand the symptom degrades from a hang into `Err( RingError::Full )` — documented as retryable back-pressure, which is the one response guaranteed never to terminate | [001](001_dropping_a_claim.md) |
| No test covers any route and none can: `ring_claim` has no dependency on `ring_publish`, normal or dev, so the suite cannot observe a publication that never arrives — the test header says as much | [001](001_dropping_a_claim.md) |
| "zero" appears twice in the source and neither is in a doc comment — once about the initial cursor, once in a `//` implementation comment at `:447` that is stripped from `cargo doc` and from this corpus's own greps | [002](002_claiming_zero.md) |
| `claim( 0 )` issues a real `compare_exchange( current, current, … )` to return its empty claim, while both refusals issue none — the cheapest request is the only one that pays | [002](002_claiming_zero.md) |
| Across both files plus `pattern/001` § CL40, `RingError::Full` carries four distinct meanings with four different correct responses, and the single documented interpretation is right for one of them | here |

