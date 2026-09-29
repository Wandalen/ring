# pattern

Two patterns, and the crate is essentially the intersection of them: a retry
loop against a moving shared value, producing a half-open range as a plain
value. `claim` is where they meet — four lines of loop that return a `Claim` —
and everything else in the crate is either an accessor onto that range or a
variant of that loop.

Each file takes one pattern, states its shape and its one non-obvious
requirement, then locates every other instance of it in the family. That second
half is where both files earn their place, because in both cases the family
implements the pattern more than once and the implementations diverge — the
retry pattern twice, in opposite decompositions; the value-range pattern three
times, agreeing on almost nothing.

The two requirements are worth stating together, because they are the same kind
of constraint pointing in opposite directions. The retry pattern requires a
primitive that **can fail and report what it saw** — which is why `fetch_add`
cannot carry it, and why choosing `compare_exchange` was the crate's founding
decision. The value-range pattern requires a type that **cannot reach the
ring** — which is why Tier 6 does not use it, and why Tier 5 has no alternative.
One pattern is enabled by what the primitive beneath it offers; the other is
forced by what the tier above it withholds.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Retrying Against a Moving Target](001_retrying_against_a_moving_target.md) | CL39, CL40 — the two instantiations decomposed at opposite seams, and the error that covers two situations because the retry is hidden |
| 002 | [The Half-Open Range as a Value](002_the_half_open_range_as_a_value.md) | CL41, CL42 — three independent implementations of the value shape, and the annotation rule that holds for five of seven range types |

### Where the Seam Falls

Both patterns are decompositions, and both files arrive at the same structural
question: which half does the caller get?

| | Pattern 001 — the retry | Pattern 002 — the range |
|--|-------------------------|-------------------------|
| The seam | between one attempt and the loop | between the range and the ring |
| This crate's side | loop inside; no `try_claim` | range only; no borrow |
| The other side, in the family | `ring_publish::try_publish` exposes the attempt | `ring_spsc`/`ring_mpsc` guards hold the ring |
| What the choice costs here | `Err( Full )` cannot distinguish contention from back-pressure | the obligation rests on a lint, not a destructor |
| What it buys | one call site, no protocol for the caller to get wrong | a `Copy` value that outlives the scope that made it |
| Is the choice forced? | **no** — argued the other way in `:25-35` | **yes** — a `Claimer` has no ring to borrow |

The last row is the distinction the two files converge on. Pattern 002's shape
is determined by the tier: no type at Tier 5 can hold a ring, so no type at Tier
5 can be a guard, and the rule holds across all seven range types without
exception. Pattern 001's shape is not determined by anything — `ring_publish`
made the opposite choice one tier over, from an argument that would have
supported this crate's, and neither crate records the comparison.

### The Family's Rarest Idiom Lives Here

The messaged `#[ must_use ]` — an annotation carrying its own sentence rather
than the default warning — appears **twelve times in 33 crates**, and three of
those twelve are range types:

| Where | Type | Says |
|-------|------|------|
| `ring_atomic:140` | `fetch_add` (trait method) | dropping the returned sequence claims a range nobody will use |
| `ring_claim:95` | `Claim` | never published → strands its slots, stalls every consumer |
| `ring_flush:126` | `Outcome` | an ignored outcome is how a misconfigured barrier hides |
| `ring_shutdown:102` | `close` | the only route to a drain; skip it and no drain ever happens |
| `ring_shutdown:348` | `into_record` | this is the record itself, not a copy — dropping it loses it |
| `ring_shutdown:519` | `Wake` | a close is read as a publish |
| `ring_slot:153` | `take` | the only way a payload leaves the slot; dropping destroys the record |
| `ring_spsc:731` | `Reservation` | dropping publishes an unwritten slot |
| `ring_spsc:960` | `Batch` | dropping discards the records it covers |
| `ring_testkit:455` | `leak` | dropping the reference leaks the ring for good |
| `ring_testkit:489` | `leak_ends` | dropping the pair leaks both allocations for good |
| `ring_testkit:555` | `Script::run` | the Outcome is the measurement; discarding it discards what was observed |

So the idiom is, in practice, this family's convention for *a value that carries
an unfulfilled obligation*, and range types are where such obligations
concentrate. Which is what makes the two unannotated exceptions in
[002](002_the_half_open_range_as_a_value.md) legible as gaps rather than as a
different house style: there is a house style, it is used twelve times, and both
gaps are byte-for-byte twins of a type that uses it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# who provides the retry primitive and who applies it
for f in ring_*/src/*.rs; do
  n=$( grep -vE "^[[:space:]]*//" "$f" | grep -cE 'compare_exchange' )
  [ "$n" -gt 0 ] && printf '%-34s %s\n' "${f#ring/}" "$n"
done

# where a compare_exchange sits directly inside a retry loop
command grep -rn -B4 'compare_exchange' ring_*/src/*.rs | command grep -E 'while|loop \{' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'

# the alternative that cannot retry, because it cannot fail
for f in ring_*/src/*.rs; do
  n=$( grep -vE "^[[:space:]]*//" "$f" | grep -cE 'fetch_add' )
  [ "$n" -gt 0 ] && printf '%-34s %s\n' "${f#ring/}" "$n"
done

# every range-shaped type, and which half of the split it is on
grep -rn -A6 '^pub struct' ring_*/src/*.rs | grep -B1 'start : Seq' | grep 'pub struct' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
command grep -rn 'impl.*Drop for' ring_*/src/*.rs | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'

# the family's rarest annotation
command grep -rn 'must_use = ' ring_*/src/*.rs | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'

# the convention, and its one cross-crate citation
command grep -rn -i 'half-open' ring_*/src/*.rs | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
ring_atomic/src/lib.rs             14
ring_claim/src/lib.rs              2
ring_cursor/src/lib.rs             2
ring_publish/src/lib.rs            1
ring_claim/src/lib.rs-437-    while count <= self.consumers.headroom( current )
ring_claim/src/lib.rs-488-    while let granted @ 1.. = max.min( self.consumers.headroom( current ) )
ring_atomic/src/lib.rs             17
ring_batch/src/lib.rs              1
ring_bench/src/lib.rs              2
ring_cursor/src/lib.rs             2
ring_stats/src/lib.rs              5
ring_mpsc/src/lib.rs:impl< S > Drop for Reserved< '_, S >
ring_mpsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Reservation< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
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
ring_claim/src/lib.rs:/// Half-open: `start..end`, so an empty claim and a one-slot claim are not the
ring_claim/src/lib.rs:  /// assert!( !claim.contains( Seq( 6 ) ), "half-open" );
ring_consume/src/lib.rs:/// Half-open, like `ring_claim::Claim`, and for the same reason: `end` is
```

| | Value |
|--|------:|
| Patterns this crate applies | 2 |
| `compare_exchange` sites family-wide | 19 |
| …in crates that *provide* the primitive | 16 |
| …in crates that *apply* it | **3** |
| Crates wrapping a `compare_exchange` in a retry loop directly | **1** |
| …wrapping it one call removed, in the caller-facing method | 1 |
| `fetch_add` sites family-wide | 27 |
| …inside a retry loop | **0** |
| Range-shaped types | **7** |
| …value-shaped — `Copy`, no destructor | 3 |
| …guard-shaped — borrow + `Drop` | 4 |
| `Drop` impls family-wide | **4** |
| Value range types measuring 16 bytes | **3 of 3** |
| Value range types agreeing on a receiver | 2 of 3 |
| …on a width type | 2 of 3 |
| …on a method count | 2 of 3 |
| Messaged `#[ must_use ]` family-wide | **12** |
| …on a range type | **3** |
| Range types whose drop is costly and unannotated | **2** |
| Unannotated absences that are argued | **1** |
| `half-open` mentions in family source | 3 |
| …that cite another crate | **1** |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL39 | family | n/a — observation | Only two crates apply the CAS-retry pattern rather than provide it, and they split it at opposite seams: `ring_claim` hides the loop and exposes no single-attempt variant, `ring_publish` exposes `try_publish` and returns the observed value as `Err( Seq )` |
| CL40 | `ring_claim` | n/a — diagnostics | Because the loop is internal, a first-evaluation refusal and a 40-attempt loss both return `Err( RingError::Full )`, so a caller cannot distinguish back-pressure from contention though the two call for opposite responses |
| CL41 | family | n/a — duplication | Three crates implement the plain-value half-open range independently (`Claim`, `BatchClaim`, `Available`), all 16 bytes, agreeing on derives and on nothing else: two receivers, two width types, three method counts, one type-level annotation between them, and no file mentioning that the other two exist |
| CL42 | family | n/a — coverage | Annotation tracks the cost of dropping across five of the seven range types, and `ring_mpsc:927-931` spends five lines justifying an *absence*, proving the rule is deliberate; the two gaps are `ring_mpsc::Batch` and `ring_batch::BatchClaim`, each a twin of an annotated type whose warning sentence applies verbatim |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| Each crate's stated argument supports the other's choice — `ring_claim:25-35` argues for caller-controlled composition and hides its loop; `ring_publish:42-53` argues its spin is safe to hide and exposes the single attempt — and neither records the comparison | [001](001_retrying_against_a_moving_target.md) |
| The pattern's negative definition: `fetch_add` has no failure arm, so there is nowhere for the decision to be re-evaluated — 27 `fetch_add` sites family-wide, none inside a retry loop | [001](001_retrying_against_a_moving_target.md) |
| `Available`'s two omissions are correct — `contains` and `overlaps` exist to assert producer-side exclusivity, and a consumer that reads a run twice has broken nothing — while `BatchClaim`'s `&self` receiver on a 16-byte `Copy` type is strictly worse and is the only divergence of the three with no defence | [002](002_the_half_open_range_as_a_value.md) |
| The value/guard split is exactly by tier — three values at Tiers 2 and 5, four guards at Tier 6 — because a guard must reach a ring and no Tier 5 type can; `ring_spsc:615-621` reads as a criticism of this crate's shape until the tier column is added, and the rule is stated nowhere | [002](002_the_half_open_range_as_a_value.md) |
| The messaged `#[ must_use ]` appears twelve times in 33 crates and three are range types, making it the family's convention for a value carrying an unfulfilled obligation — which is what makes the two gaps legible as gaps rather than as a different style | here |

