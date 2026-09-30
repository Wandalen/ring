# Integration: How the Fold Crossed Four Tiers, and Then Stopped At Two

### Scope

- **Purpose**: Work out why `ring_seqno`'s free functions were shared rather than reimplemented, when the family has repeatedly failed to share exactly this shape.
- **Responsibility**: Give the chain, name the mechanism, test it against both a positive and a negative case within the family — and record what happened when the chain's own positive case forked anyway.
- **In Scope**: How this crate's functions reach crates that do not depend on it.
- **Out of Scope**: The manifest census — see [`001`](001_one_dependency_and_five_declared_dependents.md).

### The Problem This Crate Should Have Had

`ring_seqno` exports five free functions and nothing else — no types, no constants,
no traits ([`api/001`](../api/001_five_functions_and_no_types.md)). That is the
shape the family has been worst at sharing:

| Decision | Shape | Outcome |
|----------|-------|---------|
| Cache-line size | `ring_align::on_distinct_lines( a : usize, b : usize )` — free function | **Forked.** `ring_mpsc:860-866` wrote its own, with a strictly stronger predicate |
| Gating ordering | `ring_cursor::GATING` — a `const` | **Restated four times** across three crates |
| `SeqCell` | a trait, re-exported | Crossed — 8 of 10 consumers take the re-export |
| The three readings | `CursorPair` methods | Crossed — `ring_wait`, `ring_shutdown`, `ring_spsc` all use them |

The pattern recorded in
[`ring_cursor` `integration/002`](../../../ring_cursor/docs/integration/002_who_reads_a_cursor.md)
is that a decision crosses a manifest boundary **on a type the consumer already
holds**, and does not cross as a free function or as a constant.

By that rule `ring_seqno::slowest` should have forked. For most of this crate's
life it had not, and this document was written to explain why. It has since
forked:

```sh
cd "$(git rev-parse --show-toplevel)"
# `| sort`: the family's grep returns matches in nondeterministic order, so an
# unsorted census quotes stably only by luck
grep -r '\.min()' ring_*/src/*.rs | sort
```

Live output:

```
ring_cursor/src/lib.rs:    cursors.iter().map(|c| c.load(GATING)).min()
ring_seqno/src/lib.rs:    cursors.iter().copied().min()
```

**Two hits, where the whole argument below was built on there being one.**
`ring_cursor`'s appeared in commit `b7e075ca`, which removed a heap allocation from
`ring_cursor::slowest` by folding the loads in place instead of collecting them
into a `Vec` for tier 1 to fold. The allocation is genuinely gone — measured, now,
by `ring_cursor/tests/allocation_test.rs`. What went with it is the call.

The rest of this document is kept as written, because being wrong about this is
the most useful thing it does. Its mechanism was not refuted; its confidence
that the mechanism was *sufficient* was. § "What the Fork Actually Cost the Rule"
below states the missing clause.

### The Chain

| Tier | Item | Takes | Body |
|:----:|------|-------|------|
| 1 | `ring_seqno::slowest` | `&[ Seq ]` | `.iter().copied().min()` |
| 2 | `ring_cursor::slowest` | `&[ PaddedCursor ]` | load each at `GATING` and fold — **was** "then tier 1", until `b7e075ca` |
| 3a | `Barrier::frontier` | `&self` | `ring_cursor::slowest( self.dependencies )` |
| 3b | `GatingSet::slowest` | `&self` | `ring_cursor::slowest( &self.cursors )` |
| 4 | `GatingSet::headroom`, `Barrier::available`, `Consumer::available` | `&self` + a `Seq` | tier 3, then arithmetic |

Four tiers, and the count of implementations is the whole subject of this
document: **one until `b7e075ca`, two since.** Tiers 3a, 3b and 4 still reach
tier 2 exactly as drawn. The break is the one edge between tier 2 and tier 1,
and it is the edge everything else here was written about.

### The Mechanism: a Type Change Is What Buys a Wrapper

Read the *Takes* column downward. It changes at every tier.

- Tier 2 cannot call tier 1 directly — it holds atomics, not values, and turning one into the other requires choosing a memory ordering. That is a decision, so a function has to exist to make it.
- Tier 3 cannot call tier 2 directly in a way its own callers would want — the slice is a private field.

At each step the types did not line up, so somebody had to write an adapter, and
**an adapter is a place where calling down is easier than reimplementing.** Once
you are already writing a function, `ring_seqno::slowest( &positions )` is one line
and `.iter().copied().min()` is also one line — but the first is obviously right
and the second requires deciding all over again whether a minimum or a maximum is
wanted, and what an empty set means.

That comparison is the load-bearing sentence, and it is where this document was
wrong. The two lines are not equal-cost, because the first one needs `positions`
to exist. Calling down cost an allocation per call; reimplementing cost nothing
per call and one duplicated `.min()` forever. Under contention the per-call term
wins any argument about clarity, and it did.

Now the negative case. `ring_align::on_distinct_lines( a : usize, b : usize ) -> bool`
takes two plain integers. Every prospective caller in the family already has two
`usize` addresses. **There is no adaptation to perform, so nobody wrote a
wrapper** — and with no wrapper there was no natural home for the call, so
`ring_mpsc`, which does not depend on `ring_align`, wrote three lines inline:

```rust
// ring_mpsc/src/lib.rs:860-866
pub fn on_distinct_lines( &self ) -> bool
{
  let claim = self.claimer.cursor().addr();
  let consume = self.ring.consumer_cursor().addr();
  claim.abs_diff( consume ) >= 64
}
```

Not the same predicate — `abs_diff >= 64` is strictly stronger than
`a / 64 != b / 64`, since 63 and 64 differ by one and sit on different lines. The
two are latently divergent, and the literal `64` is a second copy of a constant
the family holds precisely once.

**The rule, sharpened:** a free function crosses a manifest boundary when each
tier has a reason to *wrap* it, and a type change is what supplies that reason.
A function that is directly callable at every tier has no wrapper anywhere, and
without a wrapper the next author reimplements.

This is counterintuitive in a useful way. The convenient signature — plain
scalars, callable from anywhere — is the one that fragmented. The awkward one,
which forced an adapter at every level, is the one that stayed single — for a
while.

### What the Fork Actually Cost the Rule

The rule above is a rule about *opportunity*: a wrapper has to exist for the call
to have a home. It is silent on *price*, and price is what decided this case.

An adapter keeps calling down only while calling down stays cheaper than
reimplementing. `ring_cursor::slowest` was exactly the wrapper the rule
predicts — a real type change, a real reason to exist, a natural home for the
call. It called down for as long as the call was free to make. The moment
someone measured the `Vec` that call required, the wrapper had a motive the rule
never modelled, and it took it: the fold that had been in one crate is now in
two, four lines apart across a manifest boundary, with nothing keeping them
equal.

**The rule, corrected:** a free function crosses a manifest boundary when each
tier has a reason to wrap it *and* the wrapper can reach it without paying for
the type change. A type change buys a wrapper. It does not buy a call, and if the
adaptation the wrapper must perform costs anything per call, the wrapper is the
place where the shared implementation is most likely to be abandoned — because
the wrapper is the only party that pays.

Two things follow that are worth stating flatly:

- **This is not an argument for reverting the fix.** The allocation sat in the
  condition of a compare-exchange retry loop. Removing it was right, and it is
  now pinned by a test rather than by prose.
- **The parameter, not the fold, is what should have moved.** `slowest< I : IntoIterator< Item = Seq > >`
  was weighed and declined in
  [`data_structure/002`](../data_structure/002_the_slice_that_slowest_reads.md)
  as too invasive for the run it came up in. It would have made calling down free
  and kept one implementation. Declining it did not keep the chain whole; it
  chose which way the chain would break.

### The Rule Predicts a Second Case, and the Case Holds

If the mechanism is right, then a *method* that requires no adaptation should
also be bypassed — the type-change test should matter more than free-function
versus method.

`ring_barrier::Barrier::available` is that test:

```rust
// ring_barrier/src/lib.rs:216-219
pub fn available( &self, from : Seq ) -> u64
{
  self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
}
```

`ring_consume` holds a `Barrier` (`src/lib.rs:213`) and depends on `ring_barrier`.
It wants exactly this quantity. It wrote:

```rust
// ring_consume/src/lib.rs:338-344
let position = self.position();
let readable = self.barrier.frontier()
  .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );
```

**Finding SQ20.** `pending( frontier, position )` is `position.distance_to( frontier )`,
so this is `self.barrier.available( position )` with the body re-expanded. And:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r '\.available(' ring_*/src/*.rs | grep -vE ':\s*///'
```

Live output:

```
ring_barrier/src/lib.rs:        count <= self.available(from)
ring_consume/src/lib.rs:        let run = self.available();
ring_consume/src/lib.rs:        let run = self.available();
ring_consume/src/lib.rs:        let run = self.available();
ring_core/src/lib.rs:            ConsumerInner::Spsc(consumer) => consumer.available(),
ring_core/src/lib.rs:            ConsumerInner::Mpsc(consumer) => consumer.available(),
ring_mpsc/src/lib.rs:        self.available() == 0
```

`Barrier::available` has **no caller outside `ring_barrier`** — only its own
`admits` at `:240` uses it. Every other `.available(` hit in the family is
`Consumer::available`, an unrelated method on a different type.

The prediction holds. `Barrier::available` takes a `Seq` and returns a `u64`;
`ring_consume` had a `Seq` and wanted a `u64`. No adaptation, no wrapper, and the
author reached past it to the lower-level function they were already importing.

Being a method was not enough. **What matters is whether the call site needs
something the existing function does not already give it.**

### What This Predicts for `ring_seqno`

The crate's protection is a property of its *consumers*, not of its own design,
so it can lapse — and the first row below is no longer a prediction:

| If a consumer appears that… | Then |
|------------------------------|------|
| already holds `&[ Seq ]` | `slowest` becomes directly callable, no wrapper gets written, and the next crate after that reimplements `.min()` |
| **finds the adaptation too expensive** | **happened.** `ring_cursor` reimplemented `.min()` without any new consumer appearing at all — the existing wrapper was enough, once the price was noticed |
| already holds two `Seq` and a `Capacity` | the same for `may_claim` and `free_slots` — `ring_batch:323` is already this shape, calling `free_slots` directly with no wrapper |
| wants a lap count | nothing exists, because `laps_between` has no callers to have grown a wrapper — see [`workaround/002`](../workaround/002_laps_between_has_no_caller.md) |

`ring_batch` is the one to watch under the original rule. It imports `free_slots`
and calls it directly at `:275` with no wrapper of its own — the exact
configuration that preceded `ring_mpsc`'s fork. Nothing has forked yet; the
structural precondition is there.

Under the corrected rule there is a second watch-list, and it is the opposite
shape: every wrapper in the family that pays something per call to reach the
function it wraps. That list currently has no entries — `b7e075ca` cleared its
only one — which is worth re-checking rather than assuming, because a wrapper
joins it silently, by a change to the callee's signature that no consumer
reviews.

### SQ19 — Why the Fold Needed Four Crates, and Why It Now Needs Three

Each tier adds exactly one thing, and each addition is a type change:

```
ring_seqno::slowest    ( &[ Seq ] )                 the arithmetic — no longer reached
ring_cursor::slowest ( &[ PaddedCursor ] )        adds the loads, and now the arithmetic too
GatingSet::slowest   ( &self )                    adds the ownership
Barrier::frontier    ( &self )                    adds the opposite default
```

**Finding, and its correction.** A free function crosses a manifest boundary when
each tier has a reason to wrap it — a type change supplies that reason, but it
does not by itself keep the call. Tier 2 had every reason the rule names and
still stopped calling tier 1, because the adaptation cost a heap allocation per
call and reimplementing the one-line fold cost nothing. The wrapper is the party
that pays for the type change, so the wrapper is where a shared implementation is
abandoned first.

---

### SQ20 — The Method Its Own Consumer Declines to Call

One crate defines it, one crate holds the type, and the call never crosses:

```
ring_barrier/src/lib.rs   pub fn available( &self, from : Seq ) -> u64
                          count <= self.available( from )        the only call
ring_consume/src/lib.rs   pub fn available( &self ) -> Available  a different function
```

**Finding.** `Barrier::available` has no callers outside its own crate, while `ring_consume` — which holds a `Barrier` — re-expands its body inline.

---

### SQ21 — Four of Fourteen, in One Hundred and Thirty-Six Lines

The primitive is used across the family, and concentrated here:

```
ring_seqno/src/lib.rs      4
ring_spsc/src/lib.rs     4
ring_types/src/id.rs     3
ring_mpsc/src/lib.rs     2
ring_barrier/src/lib.rs  1
                        --
                        14
```

**Finding.** The family calls `Seq::distance_to` fourteen times and four of those are here, making a 136-line crate the densest consumer of the one span primitive everything else is built on.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_slowest_fold.md](../algorithm/002_the_slowest_fold.md) | The fold itself, and the chain as an algorithm |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_five_functions_and_no_types.md](../api/001_five_functions_and_no_types.md) | The shape that should have fragmented |

### Integrations

| File | Relationship |
|------|--------------|
| [001_one_dependency_and_five_declared_dependents.md](001_one_dependency_and_five_declared_dependents.md) | The manifest census the chain runs through |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_two_readings_without_a_capacity.md](../item/002_the_two_readings_without_a_capacity.md) | `pending` and `slowest`, and the duplicate SQ20 names |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_shared_fold_that_declines_an_identity.md](../pattern/002_the_shared_fold_that_declines_an_identity.md) | Why the `Option` is what made the wrappers safe to write |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:133-136` | Tier 1 — now reached only from this crate's own tests |
| `ring_cursor/src/lib.rs:120-123` | Tier 2, where the type change happens, and where the chain broke |
| `ring_cursor/tests/allocation_test.rs` | What pins the fix the break was made for |
| `ring_barrier/src/lib.rs:191-194, 216-219` | Tier 3a, and the bypassed method |
| `ring_gating/src/lib.rs:197-200, 222-228` | Tier 3b and tier 4 |
| `ring_consume/src/lib.rs:213, 338-344` | The bypass |
| `ring_align/src/lib.rs:138` | The negative case |
| `ring_mpsc/src/lib.rs:860-866` | What happened to it |
| `ring_batch/src/lib.rs:37, 323` | The direct call with no wrapper |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:113-130` | What every tier in the chain ultimately asserts |
| `ring_cursor/src/lib.rs:106-118` | Tier 2's doctest, the only place the two tiers are checked together |
