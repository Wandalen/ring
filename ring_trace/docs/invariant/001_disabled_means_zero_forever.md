# Invariant: Disabled Means Zero Forever

### Scope

**Purpose:** Record the crate's central invariant — a disabled trace's log is
empty for the life of the process — establish exactly what holds it, and record
the corollary the crate's own reasoning implies but never draws.

**Responsibility:** The single gate in `record`, the census of every site that
reaches the log, where the invariant is written down against where it is
enforced, how the suite asserts it, and whether an empty log can be told apart
from a disabled one.

**In Scope:** `ring_trace/src/lib.rs:256-263`, `:284`, and every
`entries_guard()` call site; the three disabled tests in
`ring_trace/tests/trace_test.rs`.

**Out of Scope:** That the disabled trace also never allocates is
[`data_structure/001`](../data_structure/001_forty_bytes_a_flag_and_a_vector_that_never_allocated.md).
The cost of the disabled path is
[`algorithm/001`](../algorithm/001_the_early_return_that_is_the_whole_feature.md).
Ordering is [`invariant/002`](002_the_order_is_the_thing_a_counter_lacks.md).

---

## One Gate, Five Doors

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the one gate the invariant rests on --'
command grep -m1 -A7 -F '  pub fn record( &self, op : TraceOp, seq : Seq, count : usize )' ring_trace/src/lib.rs
echo '  -- every method that reaches the log, and what it does there --'
command grep 'entries_guard()' ring_trace/src/lib.rs | sed 's/^/    /'
printf '    of those, sites that insert: %s   sites gated on enabled: %s\n' \
  "$( command grep -c 'entries_guard().push' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'if !self.enabled' ring_trace/src/lib.rs || true )"
echo '  -- where the invariant is written down --'
command grep 'Zero forever' ring_trace/src/lib.rs | sed 's/^/    /'
echo '  -- and how the suite asserts it --'
d=0
for t in a_disabled_trace_records_zero a_disabled_trace_records_zero_of_every_operation_kind a_disabled_trace_stays_empty_under_contention; do
  command grep "fn $t()" ring_trace/tests/trace_test.rs | sed 's/^/    /'
  if command grep -A 24 "fn $t()" ring_trace/tests/trace_test.rs | command grep -q 'trace.record('; then d=$(( d + 1 )); fi
done
printf '    disabled tests driving the log through record: %s of 3\n' "$d"
echo '  -- what a dumped log carries about which state produced it --'
command grep 'impl fmt::Display for' ring_trace/src/lib.rs | sed 's/^/    /'
```

Live output:

```
  -- the one gate the invariant rests on --
  pub fn record( &self, op : TraceOp, seq : Seq, count : usize )
  {
    if !self.enabled
    {
      return;
    }
    self.entries_guard().push( TraceEntry { op, seq, count } );
  }
  -- every method that reaches the log, and what it does there --
        self.entries_guard().push( TraceEntry { op, seq, count } );
        self.entries_guard().len()
        self.entries_guard().clone()
        self.entries_guard().iter().filter( |e| e.op == op ).count()
        self.entries_guard().clear();
    of those, sites that insert: 1   sites gated on enabled: 1
  -- where the invariant is written down --
      /// Zero forever on a disabled trace — the second half of feature 185's
  -- and how the suite asserts it --
    fn a_disabled_trace_records_zero()
    fn a_disabled_trace_records_zero_of_every_operation_kind()
    fn a_disabled_trace_stays_empty_under_contention()
    disabled tests driving the log through record: 3 of 3
  -- what a dumped log carries about which state produced it --
    impl fmt::Display for TraceOp
    impl fmt::Display for TraceEntry
```

---

### TR21 — The Invariant Is Held by One Gate and Asserted Only Through It

Five methods reach the log, all through the private `entries_guard`. Exactly one
of the five inserts, and exactly one `if !self.enabled` exists in the crate — the
same site. So "a disabled trace's log is empty forever" is not a property of the
type; it is a property of one early return in one function, and it holds because
`Vec::push` appears exactly once in the file.

That is a good design and it is also a narrow one. `entries_guard` is private, so
only in-crate code could add a second insertion — but the guard is what makes
that easy: any future method wanting to write reaches for the same accessor,
which returns a `MutexGuard< Vec< TraceEntry > >` with the whole `Vec` API on it
and no memory of whether the trace is on. A `record_batch` or a `record_all` added
next year and written the obvious way would compile, pass, and silently break the
crate's headline claim.

The suite would not catch it. All three disabled tests drive the log through
`record` — 50 publishes, one of each of the five kinds, and four threads
contending — so they assert the gate rather than the invariant. Every one of them
would still pass with an ungated second writer in place.

**Finding.** Recorded as a real invariant with a one-site enforcement and no
structural assertion behind it. Two cheap repairs, either sufficient: a comment
on `entries_guard` saying that callers which insert must check `self.enabled`
first and that `record` is the only such caller today, or a test that constructs
a disabled trace, exercises *every* public method on it, and asserts the log is
still empty — which is a statement about the type rather than about `record`, and
would fail the moment a second door opened.

---

### TR22 — An Empty Log Is Exactly the Misreading the Crate Argued Against

`Trace::disabled()` produces a log with no entries. So does `Trace::enabled()` on
a run where nothing happened. From the outside the two are identical: `len()` is
0, `is_empty()` is true, `entries()` is an empty `Vec` for both, and no method
that returns log content returns the flag beside it.

The crate has already reasoned about precisely this class of error, and reasoned
about it better than most of the corpus. Its argument for freezing the flag is
that a trace switchable mid-run "would produce a log with a silent hole at the
front, which reads exactly like a run where nothing happened early — the one
misreading a diagnostic tool must not invite". A whole-run hole is the same
misreading with the same shape, and it is reachable by simply forgetting to call
`Trace::enabled()` — which, given the disabled `Default`, is what happens on
every path that constructs the type without thinking.

`is_enabled()` exists and answers the question. What is missing is anything
pairing it with the log: `TraceOp` and `TraceEntry` both implement `Display`, so
a caller can format entries directly, but `Trace` does not, so the one place a
dump gets written is caller code that has to remember the flag on its own.

**Finding.** Not a defect in the invariant, which is sound — a corollary of the
crate's own strongest argument that the crate never draws. Two lines close it: a
sentence at `len` or `entries` saying an empty log means either state and that
`is_enabled` distinguishes them, and either a `Display for Trace` that leads with
the flag or an explicit note that formatting a whole trace is the caller's job
and must carry it. The first is documentation; the second is the difference
between a dump that can be misread and one that cannot.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/002`](002_the_order_is_the_thing_a_counter_lacks.md) | The property that does *not* hold under contention |
| [`decisions/001`](../decisions/001_the_flag_is_fixed_at_construction.md) | The argument this finding extends |
| [`data_structure/001`](../data_structure/001_forty_bytes_a_flag_and_a_vector_that_never_allocated.md) | The stronger, unclaimed form of the same invariant |
| [`api/002`](../api/002_shared_reference_everywhere_and_what_it_forces.md) | The other way the log empties |

### Sources

| Fact | Where |
|------|-------|
| The single gate | `ring_trace/src/lib.rs:256-263` |
| One insertion site, five log accesses | Census above |
| The invariant, stated at `len` | `ring_trace/src/lib.rs:284` |
| Three disabled tests, all through `record` | Census above |
| `Display` on `TraceOp` and `TraceEntry`, not on `Trace` | Census above |
| The misreading argument | `ring_trace/src/lib.rs:183-188` |

### Tests

| Test | Covers |
|------|--------|
| `a_disabled_trace_records_zero` | The gate, 50 times |
| `a_disabled_trace_records_zero_of_every_operation_kind` | The gate across all five kinds |
| `a_disabled_trace_stays_empty_under_contention` | The gate under four threads |
| `the_default_trace_is_off` | The construction that reaches the invariant by accident |
