# data_structure

Three structs, and none of them holds anything but integers. `AtomicSeq` is one
`AtomicU64`. `CountingSeq` is that cell plus four `AtomicUsize` counters, packed
into forty bytes with no padding. `OpCounts` is those four counters read out, plus
a fifth field holding their sum.

What makes the layouts worth documenting is that each carries a decision nothing
in the crate records. The packing of five contended atomics onto one cache line
looks like an oversight in a workspace that exports `CacheAligned< T >` for exactly
this — and is measurably the right choice, for a reason found only by measuring.
The stored `total` looks like a convenience and is an invariant with no enforcement
and no way to enforce it after the fact.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_one_word_and_five.md) | One Word, and Five on One Line | Both cells' sizes and alignment, the missing `repr`, and packed-vs-padded measured at four thread counts |
| [002](002_opcounts_and_the_total_it_stores.md) | `OpCounts`, and the `total` It Stores | The five public fields, the redundant one, and what derived equality does with it |

## Forty Bytes, Twice, for Different Reasons

`CountingSeq` and `OpCounts` are both forty bytes, which is a coincidence worth
naming because it invites the wrong mental model. `CountingSeq`'s forty are five
live atomics written by every thread on every operation. `OpCounts`' forty are a
`Copy` snapshot with no synchronization at all, four of whose five fields were read
at four different instants.

The one that must be laid out carefully is the first. The one that must be
*constructed* carefully is the second, and neither the type nor the compiler helps.

## What the Family Does Elsewhere

`ring_align` exists to solve the first problem and `ring_cursor` uses it — a
production cursor is 64 bytes at alignment 64 so that a producer's writes never
invalidate the consumer's line. This crate, one tier below and a sibling of
`ring_align`, uses neither.

That is defensible here and only here: a cursor's two users touch different words,
so separating them is free gain, while this shim's every operation touches its
counter and the cell together, so separating them doubles the lines acquired. The
distinction is real, measured, and written down nowhere in the source.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the three structs --'
command grep -nE '^pub struct' ring_atomic/src/lib.rs
echo '  -- alignment attributes here, and one tier up --'
echo "    ring_atomic repr(...) : $( command grep -c 'repr' ring_atomic/src/lib.rs || true )"
echo "    ring_align  repr(...) : $( command grep -c 'repr' ring_align/src/lib.rs || true )"
echo '  -- and every public field in the crate --'
command grep -nE '^  pub [a-z_]+ :' ring_atomic/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT9 | `ring_atomic` | n/a — observation | `CountingSeq` packs five contended atomics into 40 bytes at alignment 8, with no `repr` anywhere in the crate, in a workspace whose `ring_align` exports `CacheAligned< T >` and whose `ring_cursor` uses it |
| AT10 | `ring_atomic` | **measured cost** | Padding the four counters onto their own cache lines is 2× *slower* at 8 and 16 threads (0.50×, 0.49×) because every operation touches its counter and the cell together — the packing is correct, undocumented, and one plausible "fix" from a halving |
| AT11 | `ring_atomic` | n/a — unenforced | `total` is documented as a definition ("Every operation above, summed") and enforced by nothing — the crate's own test hand-maintains it in a struct literal, and writing `total : 2` there would pass every assertion |
| AT12 | `ring_atomic` | n/a — observation | Derived `PartialEq` compares the redundant field, so two reports with identical counters can be unequal — against a type whose own test comment says it is comparable "so a test can assert a whole shape at once" |
