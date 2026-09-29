# Invariant: Monotone Per Counter, and Only Per Counter

### Scope

**Purpose:** Record the one property a `Relaxed` reader of these counters can rely
on, the single operation that breaks it, and the derived reading it does not extend
to.

**Responsibility:** Monotonicity of the seven stored counters, `reset` as its only
exception, and which of the two derived readings inherits it.

**In Scope:** `RingStats::reset` and `RingStats::in_flight` in
`ring_stats/src/lib.rs`; the five contention tests in
`ring_stats/tests/stats_test.rs`.

**Out of Scope:** The ordering choice that makes monotonicity the strongest available
statement is
[`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md).
The relationship *between* two counters is
[`invariant/002`](002_claimed_never_trails_published.md).

---

## The Property, Its One Exception, and What Is Tested

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the first concurrent-read test, and what it claims --'
command grep -m1 -A4 -F '/// Every counter is monotone while writers run: a reader sampling twice never' ring_stats/tests/stats_test.rs
echo '  -- every counter it samples while a writer runs --'
awk -v t='fn each_counter_is_monotone_while_writers_run()' 'index( $0, t ) { on = 1 } on && /^}$/ { exit } on' ring_stats/tests/stats_test.rs | command grep -o 'stats\.[a-z_]*()' | sort | uniq -c
echo '  -- what every contention test in the file reads, writers excluded --'
for t in each_counter_is_monotone_while_writers_run a_snapshot_agrees_with_itself_while_writers_run a_reader_beside_a_reset_sees_only_values_the_writer_wrote counts_are_exact_under_contention distinct_policy_counters_do_not_interfere_under_contention; do
  printf '    %-58s %s\n' "$t" \
    "$( awk -v t="fn $t()" 'index( $0, t ) { on = 1 } on && /^}$/ { exit } on' ring_stats/tests/stats_test.rs | command grep -o 'stats\.[a-z_]*(' | command grep -v 'record_\|reset' | sort -u | tr -d '(' | tr '\n' ' ' )"
done
echo '  -- every operation in the crate that can lower a counter --'
printf '    fetch_sub occurrences %s, .store( occurrences %s\n' \
  "$( command grep -c 'fetch_sub' ring_stats/src/lib.rs || true )" \
  "$( command grep -c '\.store(' ring_stats/src/lib.rs || true )"
command grep -m1 -B2 -A1 -F '      counter.store( 0, Ordering::Relaxed );' ring_stats/src/lib.rs
echo '  -- and the one reading that is a gauge rather than a counter --'
command grep -m1 -F '    self.claimed().saturating_sub( self.published() )' ring_stats/src/lib.rs
```

Live output:

```
  -- the first concurrent-read test, and what it claims --
/// Every counter is monotone while writers run: a reader sampling twice never
/// sees the second reading below the first. This is the strongest statement
/// `Relaxed` supports per counter — it covers a display of claimed, published
/// or dropped, but not the derived `in_flight` gauge, whose subtraction moves
/// in both directions under this same kind of concurrent sampling.
  -- every counter it samples while a writer runs --
      2 stats.published()
  -- what every contention test in the file reads, writers excluded --
    each_counter_is_monotone_while_writers_run                 stats.published 
    a_snapshot_agrees_with_itself_while_writers_run            stats.snapshot 
    a_reader_beside_a_reset_sees_only_values_the_writer_wrote  stats.claimed stats.snapshot stats.wait_nanos 
    counts_are_exact_under_contention                          stats.claimed stats.in_flight stats.published 
    distinct_policy_counters_do_not_interfere_under_contention stats.dropped stats.dropped_total 
  -- every operation in the crate that can lower a counter --
    fetch_sub occurrences 0, .store( occurrences 1
    for counter in self.counters()
    {
      counter.store( 0, Ordering::Relaxed );
    }
  -- and the one reading that is a gauge rather than a counter --
    self.claimed().saturating_sub( self.published() )
```

---

### ST21 — The Only Test That Reads While Writers Run Reads the One Thing With No Seam

Monotonicity holds and holds strongly. Every write in the crate is a `fetch_add`;
there is no `fetch_sub` anywhere; the single `store` is inside `reset`. So absent a
reset, a counter sampled twice never goes backwards, whatever the ordering, and the
test's doc comment is right that this is the strongest statement `Relaxed` supports
per counter.

The test was also, when this finding was written, the crate's only one that read a
counter while another thread was writing. Everything else in the suite either ran
single-threaded or joined its producers before asserting — including
`counts_are_exact_under_contention`, which spawns four writers and reads after they
have all finished
([`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) § ST4).

**Finding.** The test is named `each_counter_is_monotone_while_writers_run` and its
doc opens "Every counter is monotone". It samples `published`, and nothing else —
one of seven, twice, in a 25-line body.

That is not a coverage complaint about the other six, which behave identically by
construction and would test nothing new. It is that the crate's one window into
concurrent behaviour was pointed at the single property that cannot fail: a lone
`Relaxed` load of a lone `fetch_add`-only counter is the one read in this crate with
no composition, no second counter, and no seam. Everything the corpus finds lived in
the readings this test's shape cannot reach, and the suite's name for it suggests
otherwise.

Two later tests read while writers run, and both were built to reach past that shape.
`a_snapshot_agrees_with_itself_while_writers_run` reads `snapshot` beside four writers
and asserts each derived field against the components returned with it — a composition,
a second counter, and a seam, which is exactly what this one has none of
([`pattern/002`](../pattern/002_the_derived_reading_from_separate_loads.md) § ST39).
`a_reader_beside_a_reset_sees_only_values_the_writer_wrote` reads `claimed`,
`wait_nanos` and `snapshot` across the one operation that breaks monotonicity
([`lifecycle/001`](../lifecycle/001_zero_to_reset_to_zero.md) § ST30). The census
above puts all five contention tests side by side, and this one is still the narrowest:
one counter, where the other two read three and one composite.

The name is unchanged, and still overstates what the body does. What no longer holds
is the part that made it matter — that the suite had nowhere else to look.

---

### ST22 — "The One a Live Progress Display Needs" Is the Reading That Does Not Have It

The doc comment closes with a purpose: monotonicity is "the one a live progress
display needs". A progress display is exactly the caller this crate was built for —
this crate's own stated purpose is to "see pressure before the sim dies" — so the sentence is
identifying the property that makes live sampling safe.

Monotonicity does extend to one derived reading. `dropped_total` folds three loads
taken in sequence, and since every call's first load happens after the previous
call's last, each component can only have grown; the sum is monotone across calls
even though the breakdown beside it tears.

`in_flight` does not, and cannot. It is `claimed` minus `published` — a gauge, not a
counter. It rises when a producer takes a slot and falls when one publishes, by
design, and the probe behind
[`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) § ST3 shows it
ranging over `0..=9` across two million samples with a fixed leak of eight held
underneath. Every one of those movements is legitimate.

**Finding.** So the property is stated on the counters, where it holds, and
justified by a use case that reaches for the one reading where it does not. A live
progress display showing claimed, published and dropped is on solid ground; the same
display showing in-flight is sampling a value that moves in both directions *and*
under-reports whenever it moves
([`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) § ST3), and
nothing in the crate distinguishes the two cases for it.

The other exception is `reset`, which is not a flaw but is the invariant's only real
boundary: a reader sampling across a reset sees every counter go backwards at once,
and there is no version, generation or epoch it could check to notice
([`lifecycle/001`](../lifecycle/001_zero_to_reset_to_zero.md)).

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats
command grep -F 'gauge, whose subtraction moves' tests/stats_test.rs
```

Live output:

```
/// or dropped, but not the derived `in_flight` gauge, whose subtraction moves
```

**Disposition:** applied — the test's doc comment now scopes its closing claim to
what it actually covers: a display of claimed, published or dropped, not the
derived `in_flight` gauge, which the same paragraph now states moves in both
directions under this exact kind of concurrent sampling — matching what
`algorithm/002` § ST3 measures.
Now prints: `gauge, whose subtraction moves`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/002`](002_claimed_never_trails_published.md) | The relation between two counters, assumed and unenforced |
| [`lifecycle/001`](../lifecycle/001_zero_to_reset_to_zero.md) | `reset`, the one operation that lowers a counter |
| [`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) | The derived reading that is a gauge, and what it costs |
| [`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md) | Why monotonicity is the strongest available statement |

### Sources

| Fact | Where |
|------|-------|
| No `fetch_sub`; one `store`, inside `reset` | Census above |
| The test's claim and its purpose | Census above |
| It samples `published` only | Census above |
| What the other four contention tests read | Census above |
| `in_flight` is a subtraction of two counters | Census above |
| `in_flight` observed ranging `0..=9` | `algorithm/002` § ST3 |

### Tests

| Test | Covers |
|------|--------|
| `each_counter_is_monotone_while_writers_run` | `published`, sampled twice against one writer |
| `in_flight_saturates_rather_than_wrapping` | That the gauge floors at 0 rather than wrapping |
| `consuming_does_not_affect_in_flight` | That a third counter cannot move the gauge |
| `reset_returns_every_counter_to_the_fresh_state` | The invariant's one exception, single-threaded |
| `a_snapshot_agrees_with_itself_while_writers_run` | A derived reading sampled while writers run — the composition this instance's own test has none of |
| `a_reader_beside_a_reset_sees_only_values_the_writer_wrote` | The invariant's one exception, read from a second thread while it happens |
