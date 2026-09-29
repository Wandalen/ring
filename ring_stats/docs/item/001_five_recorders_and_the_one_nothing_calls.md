# Item: Five Recorders, and the One Nothing Calls

### Scope

**Purpose:** Record the five write-side methods as a family — the shape they share,
the one that breaks it — and trace each to its callers.

**Responsibility:** `record_claim`, `record_publish`, `record_consume`,
`record_drop` and `record_wait`; their signatures, their units, and who invokes them.

**In Scope:** the five `record_*` signatures and `RingStats::wait_nanos` in
`ring_stats/src/lib.rs`; `ring_wait/Cargo.toml`.

**Out of Scope:** The two read-side methods whose contracts exceed one operation are
[`item/002`](002_the_two_methods_that_are_not_one_operation.md). Why `record_drop`
takes a policy is
[`data_structure/002`](../data_structure/002_three_drop_counters_behind_one_enum.md).

---

## Five Signatures, and Who Actually Calls Them

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the five recorders, by signature --'
command grep 'pub fn record_[a-z]*(' ring_stats/src/lib.rs
echo '  -- every call of each, outside this crate --'
for m in record_claim record_publish record_consume record_drop record_wait; do
  printf '    %-16s %s\n' "$m" \
    "$( command grep -rn "\.$m(" --include=*.rs . | command grep -v '^ring_stats/' | sed 's|ring/||' | tr '\n' ' ' )"
done
echo '  -- what the crate says the fourth counter is for --'
command grep -m1 -A9 -F '  /// Record nanoseconds spent waiting for space or data.' ring_stats/src/lib.rs
echo '  -- and how the reader beside it is documented --'
command grep -m1 -B5 -A1 -F '  pub fn wait_nanos( &self ) -> u64' ring_stats/src/lib.rs
echo '  -- and what the crate that waits depends on --'
sed -n '/\[dependencies\]/,/^$/p' ring_wait/Cargo.toml
```

Live output:

```
  -- the five recorders, by signature --
  pub fn record_claim( &self, n : u64 )
  pub fn record_publish( &self, n : u64 )
  pub fn record_consume( &self, n : u64 )
  pub fn record_drop( &self, policy : OverflowPolicy, n : u64 )
  pub fn record_wait( &self, nanos : u64 )
  -- every call of each, outside this crate --
    record_claim     ring_bench/src/lib.rs:944:  stats.record_claim( received as u64 ); 
    record_publish   ring_bench/src/lib.rs:945:  stats.record_publish( received as u64 ); 
    record_consume   ring_bench/src/lib.rs:946:  stats.record_consume( received as u64 ); 
    record_drop      ring_overflow/src/lib.rs:199:  stats.record_drop( policy, 1 ); ring_bench/tests/bench_test.rs:1069:/// `run`'s own `stats.record_drop( ..., ( offered - received ) as u64 )` ring_bench/tests/bench_test.rs:1113:  let call = "stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 );"; ring_bench/src/lib.rs:961:  stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 ); 
    record_wait      
  -- what the crate says the fourth counter is for --
  /// Record nanoseconds spent waiting for space or data.
  ///
  /// **No crate calls this.** `wait_nanos` is the fourth of the four counters
  /// `docs/feature/185_ring_stats.md` asks for, and `ring_wait` — the crate that
  /// spins, yields and sleeps — declares `ring_types` and `ring_cursor` in its
  /// manifest, not `ring_stats`. The edge that would let the waiting crate
  /// report its waiting does not exist, so [`RingStats::wait_nanos`] reads zero
  /// in every configuration this workspace can be built in. Zero is also the
  /// legitimate reading for "nothing waited", and nothing distinguishes the two.
  ///
  -- and how the reader beside it is documented --
  /// Nanoseconds spent waiting.
  ///
  /// Structurally zero — see [`RingStats::record_wait`] for why nothing writes
  /// it.
  #[ must_use ]
  pub fn wait_nanos( &self ) -> u64
  {
  -- and what the crate that waits depends on --
[dependencies]
ring_types = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }
```

---

### ST25 — One Shape, Five Times, With the Amount Always Explicit

Every recorder takes `&self` and a `u64` amount, adds it to one counter, and returns
nothing. `record_drop` alone carries a second parameter, and it is an index rather
than an amount. There is no `record_*` that increments by an implicit one.

That last point is the design choice worth naming. A counting API could reasonably
have offered `record_claim( &self )` meaning "one", and this one does not: the caller
always states the number. It makes `record_claim( 0 )` legal and meaningful — tested
as `recording_zero_changes_nothing` — and it makes the batched and single forms
provably interchangeable, which `batched_and_single_recording_agree` asserts
directly. It is also what lets `ring_bench` write a whole run's totals in four calls
([`integration/002`](../integration/002_the_read_path_and_the_removed_edge.md)).

**Finding.** The uniformity is complete and the two contracts it creates — zero is a
no-op, and `n` calls of one equal one call of `n` — are both tested. For a crate
whose findings are otherwise concentrated in what the docs do not say, this is the
part where the shape, the contract and the tests agree with each other exactly, and
it is worth recording as the baseline the rest is measured against.

---

### ST26 — `wait_nanos` Is One of the Four Requested Counters and Has No Writer

Four of the five recorders are called from outside this crate. `record_wait` is
called from nowhere: not `ring_bench`, not `ring_overflow`, not any `src/` in the
workspace. Its only invocations anywhere are five lines in `ring_stats`' own test
file and one doctest — three lines when this was written, and the two added since are
both in tests of `snapshot` and `reset`, not of waiting.

`wait_nanos` is not an incidental counter. This crate's own contract names four things to count and
it is the fourth — "nanoseconds spent waiting" — sitting beside claimed, published
and dropped under an earlier name, `wait_ns`. The crate's own reader
documented it flatly, in four words: "Nanoseconds spent waiting."

`ring_wait` is the crate that waits. It spins, it yields, and on `WaitKind::Park` it
sleeps 50µs at a time in a documented substitute for real parking. It measures none
of that, and its manifest declares `ring_types` and `ring_cursor` — not `ring_stats`.
The edge that would let the waiting crate report its waiting does not exist.

**Finding.** So `wait_nanos()` returns `0` in every configuration this workspace can
be built in, and `0` is a legitimate reading meaning "nothing waited". A monitor
reading the four requested counters gets three real numbers and one that is
structurally zero, and at the time nothing marked the difference.

Set beside
[`integration/001`](../integration/001_the_write_path_and_two_callers_that_are_not_there.md)'s
sibling finding — that `record_drop`'s only non-benchmark caller lives in a function
no `src/` imports — half of this crate's four requested counters had no live producer, and the
crate read as complete because every method exists and every method is tested.

Wiring it is not this crate's edit. It would put `ring_stats` in `ring_wait`'s
manifest and a `&RingStats` through `ring_wait`'s public signatures, which is that
crate's API and that crate's corpus. What was in reach here, and is what a monitor
actually needs, is the difference between "nothing waited" and "nothing counts":
both recorder and reader now say the counter has no writer and name the missing
manifest edge, so a zero read off `wait_nanos` is marked as structural where before
it was indistinguishable from a measurement.

**Disposition:** applied — `RingStats::record_wait` gained a paragraph naming
`ring_wait`'s two declared dependencies, `ring_types` and `ring_cursor`, as the
manifest that does not include this crate, and `RingStats::wait_nanos` now points at
it rather than reading as an ordinary counter. The census above is unchanged in what
it measures — `record_wait` still has no caller outside this crate — and that is the
point: the fix is that the reading says so. Now prints: `  /// in every configuration this workspace can be built in. Zero is also the`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](002_the_two_methods_that_are_not_one_operation.md) | The read-side methods whose contracts exceed one operation |
| [`integration/001`](../integration/001_the_write_path_and_two_callers_that_are_not_there.md) | The other requested counter with no reachable producer |
| [`decisions/002`](../decisions/002_per_policy_drop_counters_stated_and_delivered.md) | What this crate was originally asked to count, against what it ships |
| [`data_structure/002`](../data_structure/002_three_drop_counters_behind_one_enum.md) | `record_drop`'s second parameter, and what it selects |

### Sources

| Fact | Where |
|------|-------|
| The five signatures | Census above |
| `record_wait` has no caller outside this crate | Census above |
| `ring_wait`'s two dependencies | `ring_wait/Cargo.toml` |
| `ring_wait` sleeps rather than parks, and times nothing | `ring_wait/src/lib.rs:132-142` |
| `wait_nanos` as the fourth of four originally-requested counters | Census above |

### Tests

| Test | Covers |
|------|--------|
| `each_recorder_moves_exactly_one_counter` | All five, including `record_wait` |
| `recording_zero_changes_nothing` | The zero contract on every recorder |
| `batched_and_single_recording_agree` | That `n` singles equal one batch |
| `reset_returns_every_counter_to_the_fresh_state` | `record_wait`'s counter, via reset |
| *(to create)* | Nothing checks that a counter has a producer — `wait_nanos` passes every test it has |
