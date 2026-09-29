# Invariant: Every Minted Record Is Somewhere

### Scope

- **Purpose**: State the accounting law a run must satisfy, and be precise about the one thing it deliberately does not catch.
- **Responsibility**: The law, its enforcement, and the four failures it detects.
- **In Scope**: `Outcome::audit`, `audit_received`.
- **Out of Scope**: Records destroyed *inside* the ring by an overflow policy — legal, and detected by `vanished` instead (→ [`pitfall/001`](../pitfall/001_neither_the_count_nor_the_list_alone.md)).

### Invariant Statement

| # | Property | Holds when |
|---|---|---|
| R1 | Every minted record is accepted, refused, or still staged | `accepted + refused_full + refused_closed + refused_staging + staged_at_end == minted` |
| R2 | Every delivered record was minted | every value in `received` is `< minted` |
| R3 | Delivered records strictly ascend | no adjacent pair in `received` has `then <= previous` |

**R3 covers duplicates as well as reordering**, because a repeat is the equality
case of an ordering failure. Splitting them into two checks would be two ways of
finding one broken comparison, and would need two anomaly variants for a single
observable.

#### Why R1 is stated over five buckets

A record is minted at exactly one of two places — a `Push`-family step or a
`Stage`-family step — and from there it ends up in exactly one of five states:

| Bucket | Reached by |
|---|---|
| `accepted` | The ring answered `Ok`, whether it stored the record or discarded it |
| `refused_full` | The ring had no slot and the policy refuses |
| `refused_closed` | The shutdown was closed |
| `refused_staging` | The staging buffer was full; the record never reached the ring |
| `staged_at_end` | Still in the buffer when the script ended |

**`received` and `in_ring_at_end` are not buckets.** They are subsets of
`accepted`, which is why R1 does not mention them and why `vanished` is a
separate reading rather than a term in this sum.

### Enforcement Mechanism

| # | Mechanism | What it prevents |
|---|---|---|
| E1 | Minting increments a counter in the same two statements that create the record | A step cannot mint without the count rising |
| E2 | Every `try_push` result is matched exhaustively — `Ok`, `Full`, `Closed` | A refusal cannot be dropped on the floor; adding a fourth `Refusal` variant would fail to compile |
| E3 | `audit` checks R1 before R2 and R3 | A count failure is reported as a count failure, not as whatever record anomaly it happens to also produce |
| E4 | `audit_received` is a free function, so R2 and R3 can be checked with no `Outcome` | A concurrent test collecting records from a thread has no accounting to check, and would otherwise have to fabricate one |

### Violation Consequences

| # | Violation | Reported as |
|---|---|---|
| V1 | A step mints without accounting | `Anomaly::Unaccounted { minted, placed }` |
| V2 | A record comes out that was never created | `Anomaly::Unminted { value, minted }` |
| V3 | The same record comes out twice | `Anomaly::OutOfOrder { previous, then }` — `previous == then` only for an adjacent repeat; one separated by an intervening record reports as an ordinary reversal instead |
| V4 | Records come out reordered | `Anomaly::OutOfOrder { previous, then }` |

**V1 has no test that provokes it from a script**, because a script that
provoked it would be the bug. It is tested by constructing an `Outcome` whose
fields do not add up — `an_outcome_that_lost_a_record_fails_the_audit` — which
checks the detector rather than the thing detected.

#### What this law does not catch

A record the ring accepted and destroyed satisfies R1: it is counted in
`accepted`, and R1 asks nothing further of it. `audit` therefore **passes** on a
`DropNewest` run that lost half its records — deliberately, because dropping is
what that policy is for. The reading that sees it is `Outcome::vanished`, and
the two are kept apart so that a legal configuration is not reported as a
violation. → [`pitfall/001`](../pitfall/001_neither_the_count_nor_the_list_alone.md) P2.

A second, larger blind spot: R1 is checked in every scripted run and in
**none** of the loom interleavings — the concurrent model has no `Outcome` to
call `audit` on, only a bare `Vec` a consumer thread pushed into. →
[`002`](002_the_shape_a_delivered_list_must_have.md) TK23.

### Evidence

| # | Claim | Test |
|---|---|---|
| W1 | R1 holds across four differently-shaped scripts | `every_minted_record_is_accounted_for` |
| W2 | R1 holds when the staging buffer refuses | `a_full_staging_buffer_refuses_before_the_ring_is_reached` |
| W3 | R1 holds when nothing is ever flushed | `records_staged_and_never_flushed_are_still_accounted_for` |
| W4 | R1 holds with a zero-slot buffer | `a_zero_slot_staging_buffer_refuses_every_record` |
| W5 | V1 is detected | `an_outcome_that_lost_a_record_fails_the_audit` |
| W6 | V2 is detected | `a_record_that_was_never_minted_is_caught` |
| W7 | V3 and V4 are detected | `a_duplicate_and_a_reversal_are_one_anomaly` |
| W8 | R2/R3 are checked even when R1 passes | `an_outcome_that_adds_up_is_still_checked_for_its_records` |
| W9 | Gaps are legal — a dropped record leaves one | `an_ascending_list_of_minted_records_passes_the_audit` |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'terms in the R1 sum:          %s\n' "$( command grep -m1 'let placed' src/lib.rs | command grep -o 'self\.' | wc -l )"
printf 'try_push accounting sites:    %s\n' "$( command grep -c 'guard.try_push' src/lib.rs || true )"
printf 'accounting arms in total:     %s\n' "$( command grep -cE '^ +(Ok\( \(\) \)|Err\( Refusal::).*=>' src/lib.rs || true )"
printf 'Refusal variants:             %s\n' "$( awk '/^pub enum Refusal/{f=1} f && /^  [A-Z]/{n++} f && /^}$/{exit} END{print n+0}' ../ring_shutdown/src/lib.rs )"
printf 'non_exhaustive in ring_shutdown: %s\n' "$( command grep -c 'non_exhaustive' ../ring_shutdown/src/lib.rs || true )"
printf 'wildcard arms in run:         %s\n' "$( awk '/pub fn run\(/{f=1} f && /^  }$/{exit} f' src/lib.rs | command grep -c '_ =>' || true )"
printf 'how refused_staging is set:   %s\n' "$( awk '/refused_staging \+=/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'the duplicate W7 uses:        %s\n' "$( awk '/fn a_duplicate_and_a_reversal/{f=1} f && /audit_received/{ sub( /^ */, "" ); print; exit }' tests/testkit_test.rs )"
printf 'W-rows in the table above:    %s\n' "$( command grep -c '^| W[0-9]' docs/invariant/001_every_minted_record_is_somewhere.md )"
printf 'of those, tests that exist:   %s\n' "$( awk -F'|' '/^\| W[0-9]/ { gsub( /[^a-z_]/, "", $4 ); print $4 }' docs/invariant/001_every_minted_record_is_somewhere.md | while read -r n ; do command grep -q "fn $n" tests/testkit_test.rs && echo x ; done | wc -l )"
```

Live output:

```
terms in the R1 sum:          5
try_push accounting sites:    2
accounting arms in total:     6
Refusal variants:             2
non_exhaustive in ring_shutdown: 0
wildcard arms in run:         0
how refused_staging is set:   if staging.push( record ).is_err() { refused_staging += 1; }
the duplicate W7 uses:        assert_eq!( audit_received( &[ 1, 1 ], 4 ), Err( Anomaly::OutOfOrder { previous : 1, then : 1 } ) );
W-rows in the table above:    9
of those, tests that exist:   9
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_script_surface.md](../api/001_the_script_surface.md) | Where `audit` and `audit_received` sit |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_outcome_and_anomaly.md](../type/001_outcome_and_anomaly.md) | The four anomaly variants V1–V4 map onto |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_neither_the_count_nor_the_list_alone.md](../pitfall/001_neither_the_count_nor_the_list_alone.md) | The destruction this law is silent about, on purpose |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | W1–W9 |
| `tests/exhaustive_test.rs` | R2 and R3 under loom, where there is no `Outcome` to check R1 with |

### TK21 — the fifth bucket is the one E2 does not reach

E2 states the enforcement as *"Every `try_push` result is matched exhaustively —
`Ok`, `Full`, `Closed`"*, and it is exactly true: `run` has two `guard.try_push`
sites, six arms between them, and no wildcard. A new `Refusal` variant would
stop both from compiling.

R1 sums **five** buckets and four of them are filled at those two sites.
The fifth, `refused_staging`, is filled here:

```text
if staging.push( record ).is_err() { refused_staging += 1; }
```

`is_err()` on an `if`. `TlsBuffer::push` returns a `Result` whose error carries
the record, and `.is_err()` discards both the payload and any future distinction
the return type might grow. Where a fourth `Refusal` variant breaks the build in
two places, a second staging failure mode would compile silently and land in the
same counter as the first.

This is not a defect in the code — `TlsBuffer::push` has one failure mode and an
`if` is the honest way to write one. It is a gap between what the Enforcement
Mechanism table claims and what it covers: E2 reads as the mechanism behind R1,
and R1 has a term E2 says nothing about. The bucket it says nothing about is the
only one unique to this crate's staging path.

### TK22 — V3's stated signature only identifies an adjacent repeat

V3 says a duplicate is *"`Anomaly::OutOfOrder { previous, then }` with
`previous == then`"*, which tells a reader how to tell a repeat apart from a
reordering. `audit_received`'s second pass walks `received.windows( 2 )` and
returns on the first pair where `then <= previous`.

For `[ 1, 1 ]` that is `previous == then`, as stated. For `[ 0, 1, 0 ]` — the
same record delivered twice, one apart — the first failing pair is `( 1, 0 )`,
so the anomaly is `OutOfOrder { previous : 1, then : 0 }`. The values are
unequal, no other variant is produced, and the duplicate is reported in the
shape V3 assigns to V4.

R3 still *catches* it: strict ascent is violated by any repeat, wherever it
sits. What does not survive is the diagnostic — `previous == then` is a property
of adjacency, not of duplication, and the document offers it as the way to read
the two apart.

W7 is `a_duplicate_and_a_reversal_are_one_anomaly` and its two assertions are
`&[ 1, 1 ]` and `&[ 2, 1 ]`. Both are adjacent pairs, so the case where the
signature stops holding is the case the suite does not reach.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
command grep -m1 -F 'only for an adjacent repeat; one separated by an intervening record' docs/invariant/001_every_minted_record_is_somewhere.md
```

Live output:

```
| V3 | The same record comes out twice | `Anomaly::OutOfOrder { previous, then }` — `previous == then` only for an adjacent repeat; one separated by an intervening record reports as an ordinary reversal instead |
```

**Disposition:** applied — V3's row now states that `previous == then` only
identifies an adjacent repeat, and that a repeat separated by an intervening
record — `[ 0, 1, 0 ]` — reports as an ordinary reversal instead, matching what
`audit_received` actually returns.
Now prints: `only for an adjacent repeat; one separated by an intervening record`
