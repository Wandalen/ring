# algorithm

Two executable statements decide everything this crate does. One is a branch on
a `bool` that returns early, and it is the entire disabled feature. The other is
a linear scan under a lock, and it is the entire counting feature. Neither is
more than a line, and both cost more than the crate says they do — the first
because a branch in another crate is a function call in the binary, the second
because a scan that grows with the log is a lock hold that grows with the log.

What links them is that the crate reasoned carefully about cost in both places
and reached its conclusions from the source rather than from a measurement. The
module doc prices the disabled path as "already a branch on a `bool`", which is
true until a compiler that cannot see across a crate boundary gets hold of it;
`entries()` refuses to hand out a guard because the lock is on the producers'
path, and then `count_of` holds that lock for two hundred microseconds. Both
readings are right about the algorithm and wrong about the binary.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_early_return_that_is_the_whole_feature.md) | The Early Return That Is the Whole Feature | The disabled branch, its measured cost, and the one attribute that removes it |
| [002](002_a_linear_scan_where_a_counter_would_do.md) | A Linear Scan Where a Counter Would Do | `count_of`'s hold on the producers' lock, and what a reader costs a producer |

## A Branch Nobody Can Inline

`record`'s guard clause is three lines and reads as free. Measured against a
loop that does nothing, the shipped call costs 2.75 ns and the loop costs
0.68 ns; an `#[ inline ]` replica of the identical branch also costs 0.68 ns. So
the branch is not measurable and the call is, and the call exists because the
family carries zero inline hints across all 33 crates and the workspace declares
no release profile, leaving link-time optimisation at cargo's default of off.

The number matters because of what this family is for. The benchmark's whole
output is a measured comparison between candidate ring designs, and a diagnostic
that is meant to be left compiled in while that comparison runs is the one place
where two nanoseconds of pure call overhead is worth a sentence.

## A Read That Stops the Writers

The crate identified the right hazard — a lock on the producers' path, held
across code the crate does not control — and closed it for `entries()` by
returning a copy. `count_of` then acquires the same lock and scans every entry
under it. At a hundred thousand entries that hold is about 200 µs, measured
twice; a producer running alongside one reader polling `count_of` lands roughly
three thousand records where alone it lands almost six million.

The suite's own consistency check calls `count_of` once per discriminant, which
is five acquisitions and five scans. Over fifteen entries that is free and reads
well, and it is also the only worked example of the method the crate ships, so it
is the idiom a reader carries to a log that is not fifteen entries long.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two statements --'
command grep -m1 -A4 -F '    if !self.enabled' ring_trace/src/lib.rs
command grep -m1 -F '    self.entries_guard().iter().filter( |e| e.op == op ).count()' ring_trace/src/lib.rs
echo '  -- the constant-time read beside them --'
command grep -m1 -F '    self.entries_guard().len()' ring_trace/src/lib.rs
echo '  -- inline hints in the family, and the release profile --'
t=0
for c in ring_*/; do
  k=$( command grep -c '#\[ inline' "$c"src/lib.rs 2>/dev/null || true )
  t=$(( t + k ))
done
echo "    inline hints across 33 crates: $t"
if command grep -q '^\[profile\.release\]' Cargo.toml; then echo '    a release profile exists'; else echo '    no [profile.release] in the workspace manifest'; fi
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR1 | `ring_trace` | **measured cost** | The module doc prices the disabled path as "already a branch on a `bool`", which is true of the source and false of the built artefact: measured over two independent runs the shipped `Trace::record` costs 2.75 ns per call against a 0.68 ns floor, while an `#[ inline ]` replica of the identical branch costs 0.68 ns — indistinguishable from the floor — so the branch is not measurable and the entire 2.07 ns is un-inlinable cross-crate call overhead, arising because `record` carries no `#[ inline ]`, the family carries zero inline hints across all 33 crates, and the workspace declares no `[profile.release]` so link-time optimisation sits at cargo's default of off; one attribute takes the disabled path to the floor and costs nothing when the trace is on, where a lock acquisition dwarfs a call |
| TR2 | `ring_trace` | n/a — doc gap | The acceptance criterion is a pair of counts — one entry per operation when enabled, zero when not — and constrains neither time nor space, which is how the nineteen tests read it, every assertion being about `len`, `count_of` or the contents of `entries()` and none about duration; the performance claim enters through the crate's own prose instead, the module doc arguing for the `Mutex` on the ground that the disabled path is already free and the test header calling a trace that recorded when off one that "would put a lock and an allocation on the path being measured", so it is the volunteered argument the measurement contradicts rather than the criterion, which is met exactly, and the repair is either TR1's attribute or one clause conceding that the branch is free and the call is not |
| TR3 | `ring_trace` | **measured cost** | `entries()` returns a copy and documents why — "handing out a guard would let a caller hold the lock across arbitrary code, and the lock is on the path producers take" — and the same reasoning is not applied to `count_of`, which acquires that lock and holds it across a full linear scan, about 200 µs at 100,000 entries in both runs, against `len`'s constant 40–80 ns; a producer recording for a fixed 200 ms alongside a single reader polling `count_of` lands roughly 2,300–3,500 records where the same producer alone lands about 5.9 million, a worst case rather than an average since the reader is a tight loop and the log grows under it, but polling a counter during a run is the ordinary use of a diagnostic and the producers' path is precisely what the crate said it was protecting |
| TR4 | `ring_trace` | n/a — doc gap | `the_per_kind_counts_sum_to_the_total` proves no entry is unaccounted for or double-counted by calling `count_of` once per discriminant and summing, which is five lock acquisitions and five full scans to answer what one pass could answer — free over its fifteen entries and good prose there, but it is the only worked example of `count_of` the crate ships and therefore the idiom that propagates to logs that are not fifteen entries long, against documentation that states neither the method's complexity nor its lock behaviour; either `count_of`'s doc should carry both, or the crate should offer the single-pass `counts()` shape `ring_stats` already uses, which would also make this assertion one acquisition |
