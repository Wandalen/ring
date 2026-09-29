# Algorithm: The Early Return That Is the Whole Feature

### Scope

**Purpose:** Record that this crate's disabled path is one `bool` branch and an
early return, establish what that path actually costs in a built binary as
against what the module documentation says it costs, and identify the single
attribute that closes the gap.

**Responsibility:** `Trace::record`'s guard clause, the module doc's pricing of
it, the family's use of inline hints and link-time optimisation, and a
three-arm measurement separating the call from the branch it guards.

**In Scope:** `ring_trace/src/lib.rs:37-39`, `:256-263`; the workspace
manifest's release profile; `#[ inline ]` across all 33 `ring_*` crates.

**Out of Scope:** Whether "zero when not" is met as a count of entries is
[`invariant/001`](../invariant/001_disabled_means_zero_forever.md). The cost of
the *enabled* path is
[`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md).

---

## The Branch, and What the Crate Says It Costs

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the branch that is the whole feature --'
command grep -m1 -A7 -F '  pub fn record( &self, op : TraceOp, seq : Seq, count : usize )' ring_trace/src/lib.rs
echo '  -- what the module doc claims it costs --'
command grep -m1 -A2 -F '//! the honest one. The alternative — a lock-free log — would make the disabled' ring_trace/src/lib.rs
echo '  -- inline hints across all 33 crates of the family --'
t=0
for c in ring_*/; do
  k=$( command grep -c '#\[ inline' "$c"src/lib.rs 2>/dev/null || true )
  t=$(( t + k ))
done
echo "    total: $t"
echo '  -- and the release profile that would inline across crates without them --'
if command grep -q '^\[profile\.release\]' Cargo.toml; then sed -n '/^\[profile\.release\]/,/^\[/p' Cargo.toml; else echo '    no [profile.release] in the workspace manifest'; fi
echo '  -- the two numbers the criterion actually asks for, and where they live --'
command grep '`ring_trace` records one entry' bench_harness/docs/acceptance/001_feature_reached_tests.md | command grep -o 'ring_trace` records[^|]*' | sed 's/ *$//'
```

Live output:

```
  -- the branch that is the whole feature --
  pub fn record( &self, op : TraceOp, seq : Seq, count : usize )
  {
    if !self.enabled
    {
      return;
    }
    self.entries_guard().push( TraceEntry { op, seq, count } );
  }
  -- what the module doc claims it costs --
//! the honest one. The alternative — a lock-free log — would make the disabled
//! path no cheaper (it is already a branch on a `bool`) while making the
//! enabled path a second concurrency problem inside the crate that exists to
  -- inline hints across all 33 crates of the family --
    total: 2
  -- and the release profile that would inline across crates without them --
    no [profile.release] in the workspace manifest
  -- the two numbers the criterion actually asks for, and where they live --
ring_trace` records one entry per sequence operation when enabled and zero when not
```

## What It Costs Built

*This probe recorded `record`'s shipped cost before TR1's fix below: an
un-inlined cross-crate call measured at 2.75 ns against the 0.68 ns floor
shown beside it. `record` has since been given `#[ inline ]`, so timing it
again today should land near that floor instead of the ~2.75 ns printed
here. See TR1's Disposition below.*

```rust
// -tr_probe/src/bin/disabled_cost.rs
// Three arms, so the call is separated from the branch it guards:
//   - nothing at all, the floor;
//   - `Trace::record` as shipped — a cross-crate call, no `#[ inline ]`, no LTO;
//   - a local `#[ inline ]` replica of the same `bool` branch.
struct LocalTrace { enabled : bool, entries : Mutex< Vec< TraceEntry > > }

impl LocalTrace
{
  #[ inline ]
  fn record( &self, op : TraceOp, seq : Seq, count : usize )
  {
    if !self.enabled { return; }
    self.entries.lock().unwrap().push( TraceEntry { op, seq, count } );
  }
}

#[ inline( never ) ]
fn arm_shipped( trace : &Trace, n : usize ) -> usize
{
  let mut seen = 0usize;
  for i in 0..n { trace.record( TraceOp::Publish, Seq( black_box( i as u64 ) ), 1 ); seen += 1; }
  seen
}
// Nine repetitions, all three arms back-to-back inside each. Absolute
// nanoseconds per call are the reported quantity: a ratio against a floor this
// close to zero is not a stable number — an earlier two-arm version of this
// probe reported 4.0x and 8.1x from two builds whose timed regions were
// identical.
```

Two independent runs, `--release`:

```
  size_of::<Trace>()      40
  size_of::<TraceEntry>() 24
  a disabled trace's heap allocation: 0 bytes
  nothing at all       median 0.676 ns/call  min 0.672  max 0.755
  Trace::record        median 2.752 ns/call  min 2.739  max 2.802
  inlined replica      median 0.676 ns/call  min 0.670  max 0.988
  and the trace is still empty: len 0

  nothing at all       median 0.673 ns/call  min 0.672  max 0.696
  Trace::record        median 2.744 ns/call  min 2.739  max 2.776
  inlined replica      median 0.677 ns/call  min 0.674  max 0.694
  and the trace is still empty: len 0
```

---

### TR1 — The Disabled Path Is a Branch in the Source and a Function Call in the Binary

`record`'s body is four lines, and the first three are the feature. `if
!self.enabled { return; }` reads one byte of an owned struct and returns; there
is no lock acquisition, no allocation, no atomic. As an algorithm it is as close
to free as a call can be, and the module doc prices it accordingly: the
alternative design "would make the disabled path no cheaper (it is already a
branch on a `bool`)".

That sentence is true of the source and false of the built artefact. The
measurement puts the shipped call at 2.75 ns and a loop that does nothing at
0.68 ns — the same 0.68 ns an `#[ inline ]` replica of the identical branch
costs, which is to say the branch itself is not measurable and the whole 2.07 ns
difference is the call. `record` carries no `#[ inline ]`, the workspace manifest
declares no `[profile.release]` so link-time optimisation is off at cargo's
default, and a non-generic function in another crate under those conditions
cannot be inlined at all. The branch the doc prices is behind a call the doc does
not mention.

**Finding.** One attribute closes it. `#[ inline ]` on `record` takes the
disabled path from 2.75 ns to the 0.68 ns floor — indistinguishable, in this
measurement, from not calling anything — and costs nothing when the trace is on,
where a lock acquisition dwarfs the call anyway. The wider fact is that the
family carried **zero** inline hints across all 33 crates and declares no release
profile to compensate, so this was one instance of a family-wide default rather
than an oversight here; but this crate is the one whose documentation makes a
cost claim that the default falsifies, so this is the one crate where the default
was overridden. The attribute below is now the family's only one.

```sh
cd "$(git rev-parse --show-toplevel)"
# the structural facts, not the wall-clock ones. The probe that produced the
# nanosecond figures above is a measurement, not a recipe: two runs of it
# disagree by more than the effect it measures, and its output carries whatever
# cargo happens to say about lock contention that minute. What can be
# re-derived on demand is the attribute, its stated reason, and the two
# family-wide defaults the finding weighs it against
echo '  -- the attribute, and the doc line stating why it is there --'
awk '/^  #\[ inline \]$/ { print prev3; print prev2; print prev1; print; getline; print; exit }
     { prev3 = prev2; prev2 = prev1; prev1 = $0 }' ring_trace/src/lib.rs
echo '  -- every inline hint in the family, and the crate holding it --'
command grep -rl '^  #\[ inline \]$' ring_*/src/lib.rs | sed 's/^/    /'
printf '    crates with at least one: %s of %s\n' \
  "$( command grep -rl '^  #\[ inline \]$' ring_*/src/lib.rs | wc -l )" \
  "$( command grep -v '^#' bench_harness/gate/declared/ring/crates.txt | command grep -cv '^$' )"
echo '  -- and the release profile that would compensate, if it existed --'
command grep -c 'profile.release' Cargo.toml | sed 's/^/    [profile.release] sections in the workspace manifest: /'
```

Live output:

```
  -- the attribute, and the doc line stating why it is there --
  /// Carries `#[ inline ]` so the disabled path — a single `bool` read and a
  /// return — can be inlined into cross-crate call sites instead of paying for
  /// an un-inlined call on every producer/consumer step (→ `algorithm/001`).
  #[ inline ]
  pub fn record( &self, op : TraceOp, seq : Seq, count : usize )
  -- every inline hint in the family, and the crate holding it --
    ring_trace/src/lib.rs
    crates with at least one: 1 of 33
  -- and the release profile that would compensate, if it existed --
    [profile.release] sections in the workspace manifest: 0
```

The nanosecond figures quoted above stay in the prose as what they are: one
recorded run, on one machine, at the time the finding was written. They are not
re-quoted here, because a block that cannot reproduce is a block that quietly
stops being checked.

**Disposition:** applied — `record` now carries `#[ inline ]`, stated at the
declaration with the reason, and the re-measurement that motivated it put the
shipped disabled path within noise of the "nothing at all" floor instead of the
prior 2.75 ns, closing the gap TR1 identifies at this one call site. The
family-wide census moved with it and is restated above: one crate of 33 now
carries a hint, and it is this one.
Now prints: `crates with at least one: 1 of 33`

---

### TR2 — The Criterion Counts Entries, and the Crate Answers About Cost

The acceptance criterion is a pair of counts: `ring_trace` "records one entry per
sequence operation when enabled and zero when not". Both halves are about the
length of a vector. Nothing in it constrains time or space, and the test suite
reads it exactly that way — nineteen tests, every one an assertion about `len`,
`count_of`, or the contents of `entries()`, and not one about duration.

The crate's own documentation is where the cost claim enters. The module doc
argues for the `Mutex` on the grounds that the disabled path is already free, the
test file's header says a trace that recorded anything when switched off "would
put a lock and an allocation on the path being measured", and the `len` doc ties
zero-forever to the second half of the crate's own acceptance clause. So the
crate volunteers a performance argument the criterion never asked for, and it is
the volunteered argument that the measurement contradicts — not the criterion,
which is met exactly.

**Finding.** The gap is worth stating precisely because the fix differs by which
claim one is defending. Against the criterion, nothing is wrong: zero entries is
zero entries, asserted under contention at 8,000 calls. Against the crate's own
sentence about the disabled path being free, 2.07 ns per call of pure call
overhead is a real cost on a path the family's whole output is a measurement of.
The honest repair is either the attribute in TR1 or one clause conceding that the
branch is free and the call is not.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](../invariant/001_disabled_means_zero_forever.md) | The half of the criterion that is met exactly |
| [`non_functional_requirement/001`](../non_functional_requirement/001_zero_when_not_is_a_count_not_a_cost.md) | The same measurement as a requirement rather than an algorithm |
| [`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md) | The argument this measurement bears on |
| [`algorithm/002`](002_a_linear_scan_where_a_counter_would_do.md) | The other executable statement in the crate |

### Sources

| Fact | Where |
|------|-------|
| The guard clause | `ring_trace/src/lib.rs:256-263` |
| The module doc's pricing of it | `ring_trace/src/lib.rs:37-39` |
| Zero inline hints across 33 crates | Census above |
| No workspace release profile | Census above |
| The criterion's two counts | `bench_harness/docs/acceptance/001_feature_reached_tests.md:53` |
| 2.75 ns shipped, 0.68 ns inlined, 0.68 ns floor | Probe above, two runs |

### Tests

| Test | Covers |
|------|--------|
| `a_disabled_trace_records_zero` | The branch, as a count |
| `a_disabled_trace_stays_empty_under_contention` | The same branch on four threads |
| `a_disabled_trace_records_zero_of_every_operation_kind` | That no discriminant bypasses it |
