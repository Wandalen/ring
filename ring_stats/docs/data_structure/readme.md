# data_structure

`RingStats` is one struct with seven `AtomicU64` fields and no `repr`. There is
nothing else in the crate — no buffer, no index, no handle type. Everything
recordable about the layout is recordable about those seven fields: how they sit in
memory, and how three of them are addressed by an enum rather than by name.

The two instances take those two questions. The first measures the packing: 56 bytes
at alignment 8, counters written by different threads sharing one cache line, and a
paired measurement putting the shipped layout between 2.6× and 3.3× the cost of the
same counters given a line each — against the *opposite* result the same workspace
recorded one crate over. The second follows the three counters behind
`OverflowPolicy`, where the field names, the enum's own doc, and two predicates in
`ring_types` all agree that a refusal is not a loss, and the public reader and the
total both disagree.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_seven_counters_on_one_line.md) | Seven Counters on One Cache Line | 56 bytes, no `repr`, measured 2.6×–3.3×, and the unused padding crate next door |
| [002](002_three_drop_counters_behind_one_enum.md) | Three Drop Counters Behind One Enum, One of Which Is Not a Drop | The enum index, the break in field naming, and the total that erases it |

## The Same Question, Answered Twice, Oppositely

`ring_atomic` packs four counters beside a sequence cell and measured padding at 2×
*slower*, because every operation there touches its counter and the cell together.
`ring_stats` packs seven counters written by three different roles — producers,
consumers, the overflow path — and measured padding at roughly 2.9× *faster*, because
nothing there reads a neighbour.

Both results are correct for their struct. What distinguishes them is not size or
field count but whether the fields sharing a line are touched by one operation or by
different threads, and neither struct carries a comment saying which it is. The
workspace ships `ring_align::CacheAligned` for exactly this, safe and without
`unsafe`, and one crate in thirty-three declares it.

## Storage That Distinguishes What the Surface Merges

Three fields — `dropped_newest`, `dropped_oldest`, `failed` — are selected by
`OverflowPolicy` through a five-line `match` written out twice, once for recording
and once for reading. The naming is careful: the third is `failed`, not
`dropped_failed`, because `OverflowPolicy::Fail` returns the item to the caller
rather than losing it, and `ring_types` says so twice more with `reports_failure()`
and `drops_silently()`.

Above that storage sit `dropped( policy )` and `dropped_total()`, which name a
refusal as a drop and add it to a sum of losses. The module comment argues at length
that drop causes must not be collapsed into one bucket; `dropped_total` is that
bucket, and a test named for the distinction pins the number that ignores it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the layout, and its size --'
command grep -m1 -A10 -F '#[ derive( Debug, Default ) ]' ring_stats/src/lib.rs
echo '  -- the padding crate the workspace ships, and who declares it --'
command grep -n 'repr( align' ring_align/src/lib.rs
for f in */Cargo.toml ; do if command grep -q '^ring_align' "$f"; then echo "    declares ring_align: $( basename "$( dirname "$f" )" )/Cargo.toml"; fi; done
echo '  -- the two predicates ring_types ships for the three variants --'
command grep -n 'pub const fn reports_failure\|pub const fn drops_silently' ring_types/src/policy.rs
echo '  -- and the fold that consults neither --'
command grep -m1 -F '    OverflowPolicy::ALL.iter().map( | p | self.dropped( *p ) ).fold( 0, u64::saturating_add )' ring_stats/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST9 | `ring_stats` | **measured cost** | Seven `AtomicU64` with no `repr` pack into 56 bytes on one cache line while producers, consumers and the overflow path each write different fields and none reads a neighbour — measured as a median of nine paired ratios, the shipped layout costs 2.6×–3.3× the same counters given a line each, at 3, 6 and 12 threads, with all eighteen ratios above 2.1× |
| ST10 | `ring_stats` | n/a — doc gap | The workspace has measured cache-line packing twice with opposite answers — 2× *faster* packed for `ring_atomic`'s `CountingSeq`, ~2.9× *slower* packed here — and neither struct records which regime it is in, so both obvious readings ("pad these" and "the family packs counters") are correct for one and a regression for the other |
| ST11 | `ring_stats` | **misleading doc** | `dropped( OverflowPolicy::Fail )` is a method whose name asserts a loss, reading a field named `failed` because it is not one, keyed by a variant documented as handing the item back to the caller — the storage and `ring_types`' own `reports_failure()`/`drops_silently()` predicates all record the distinction, and the only public way to ask for the refusal count is to ask how many were dropped |
| ST12 | `ring_stats` | n/a — unenforced | The module comment argues that drop causes must never be collapsed into one bucket, and `dropped_total` folds all three counters into exactly that — including refusals, which lose nothing — while `OverflowPolicy::drops_silently()` sits in this crate's only dependency as a ready one-line filter, and `refusals_are_counted_even_though_nothing_is_lost` pins the conflated total its own name denies |
