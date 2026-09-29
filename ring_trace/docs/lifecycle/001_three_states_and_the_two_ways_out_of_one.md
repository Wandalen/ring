# Lifecycle: Three States and the Two Ways Out of One

### Scope

**Purpose:** Enumerate the states a `Trace` can be in and the transitions
between them, record that one state is absorbing by design and one has no exit at
all at process end, and measure how fast the growing state grows.

**Responsibility:** The two constructors that fix the flag, the two methods that
write the log, the absence of any `Drop` impl or flush path, and the log's
growth rate under one and four producers.

**In Scope:** `ring_trace/src/lib.rs:19`, `:211-223`, `:262`, `:365`,
`:318`.

**Out of Scope:** Why the flag is frozen is
[`decisions/001`](../decisions/001_the_flag_is_fixed_at_construction.md). Who can
reach `clear` is
[`api/002`](../api/002_shared_reference_everywhere_and_what_it_forces.md). The
task file's own lifecycle stage is
[`lifecycle/002`](002_a_finished_crate_in_the_unverified_stage.md).

---

## The States and the Edges

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the states, and the two methods that move between them --'
printf '    constructors fixing the flag: %s   methods that write the log: %s\n' \
  "$( command grep -c 'pub const fn enabled()\|pub const fn disabled()' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'entries_guard().push\|entries_guard().clear' ring_trace/src/lib.rs || true )"
command grep 'entries_guard().push\|entries_guard().clear' ring_trace/src/lib.rs | sed 's/^/    /'
echo '  -- what happens at the end of the run --'
printf '    Drop impls in the crate: %s\n' "$( command grep -c '^impl Drop' ring_trace/src/lib.rs || true )"
printf '    methods named flush, dump, persist, write or save: %s\n' \
  "$( command grep -c 'fn flush\|fn dump\|fn persist\|fn write_\|fn save' ring_trace/src/lib.rs || true )"
printf '    the only way log content leaves the process: '
command grep 'pub fn entries' ring_trace/src/lib.rs | sed 's/^ *//'
echo '  -- and what the crate says about the growing state --'
command grep 'grows without' ring_trace/src/lib.rs | sed 's/^/    /'
printf '    caps, limits or reservations on the Vec: %s\n' \
  "$( command grep -c 'with_capacity\|truncate\|reserve\|capacity' ring_trace/src/lib.rs || true )"
```

Live output:

```
  -- the states, and the two methods that move between them --
    constructors fixing the flag: 2   methods that write the log: 2
        self.entries_guard().push( TraceEntry { op, seq, count } );
        self.entries_guard().clear();
  -- what happens at the end of the run --
    Drop impls in the crate: 0
    methods named flush, dump, persist, write or save: 0
    the only way log content leaves the process: pub fn entries( &self ) -> Vec< TraceEntry >
  -- and what the crate says about the growing state --
    //! order; a trace answers "which operations, in what order" and grows without
    caps, limits or reservations on the Vec: 0
```

## How Fast the Third State Grows

*This probe's growth-rate figures come from a one-off scratch binary
under `-tr_probe/`, run once and swept afterward per this project's
convention for temporary files — it cannot be re-run to reconfirm. TR30's
disposition below only edited a doc comment; `record`'s push and
`TraceEntry`'s 24-byte size are unchanged, so the numbers below stand as
a recorded measurement rather than a live guarantee.*

```rust
// -tr_probe/src/bin/growth_rate.rs
// One and four producers recording as fast as they can for a fixed window,
// reported as records and as live bytes.
while start.elapsed() < WINDOW
{
  trace.record( TraceOp::Publish, Seq( i ), 1 );
  i += 1;
}
let live = trace.len() * size_of::< TraceEntry >();
```

Two independent runs, `--release`, 200 ms window:

```
  1 producer(s): 5831296 records in 200 ms = 136671 KiB live, 667 MiB/s
  4 producer(s): 1766080 records in 200 ms = 41392 KiB live, 202 MiB/s
  one TraceEntry is 24 bytes; nothing caps the Vec

  1 producer(s): 5953024 records in 200 ms = 139524 KiB live, 681 MiB/s
  4 producer(s): 1629120 records in 200 ms = 38182 KiB live, 186 MiB/s
  one TraceEntry is 24 bytes; nothing caps the Vec
```

---

### TR29 — Three States, One Absorbing, and No Exit at Process End

The state space is small and completely determined. Two constructors fix the
flag, and there is no third writer of it, so a `Trace` is either disabled — where
`record` returns before touching anything and the log is empty forever — or
enabled, where it is empty or populated. Exactly two methods write the log:
`record` pushes, `clear` empties. So the graph is three nodes and three edges:
`record` moves empty→populated and populated→populated, `clear` moves
populated→empty, and disabled has no outgoing edge at all.

Disabled being absorbing is the crate's own decision and a good one. What is not
a decision, because nothing records it as one, is that the enabled states have no
exit either — not to disabled, which is intended, but out of the process. There
is no `Drop` impl in the crate, no `flush`, `dump`, `persist` or `save`, and
`entries()` is the only way log content leaves the `Trace` at all. The log lives
in process memory, is copied out only on demand, and is gone when the `Trace`
drops.

For most types that is unremarkable. For this one it is worth stating, because
the module doc positions the crate as the thing a diagnosis "reaches for" after
the counter — and the failures a sequence trace is most useful against are the
ones where nobody gets to reach for anything: an abort, a hang, a run under a
harness that kills the process. A trace whose entire output path requires a
living reader in the same process is a trace that cannot testify about how the
process ended.

**Finding.** Recorded as a design boundary that is real, defensible, and
unwritten. The crate does not claim persistence and should not grow a file
writer — that is squarely a caller's job and would drag `std::fs` into a crate
that already stands out for needing `std` at all. What is missing is the
sentence: the log is process-local and unflushed, `entries()` is the only way out,
and a caller who wants a trace to survive the run must copy it out before the end
they are trying to diagnose.

---

### TR30 — "Grows Without Bound" Is 667 MiB a Second

The module doc is honest about the direction: a trace "answers 'which operations,
in what order' and grows without bound", stated in the same breath as the
contrast with `ring_stats`' constant space. Nothing in the crate caps it —
zero occurrences of `with_capacity`, `truncate`, `reserve` or `capacity` — which
is consistent with the claim rather than a gap in it.

The rate is the part nobody has. One producer recording as fast as it can puts
5.83 and 5.95 million entries into the log in a 200 ms window across two runs —
133 and 136 MiB of live `TraceEntry`, or 667 and 681 MiB per second. Four
producers are slower in aggregate, 186–202 MiB/s, because the lock throttles
them; that is contention doing the crate a favour, not a design margin.

So an enabled trace on a hot producer will exhaust a gigabyte of memory in under
two seconds. That is not a defect — an unbounded log is what was asked for, and
the crate is off by default precisely so this cannot happen accidentally. It does
mean the enabled state is bounded in practice by wall-clock seconds rather than by
anything in the code, and that a caller who switches the trace on for "a
diagnostic run" needs to know the run has to be short.

**Finding.** The claim is accurate and the number is the missing half of it.
"Grows without bound" tells a reader the shape; 667 MiB/s from a single producer
tells them the budget, and the two together are what makes the crate safely
usable. One measured sentence beside the existing claim — with the caveat that
this is one machine's figure and the order of magnitude is what carries — turns a
correct warning into an actionable one, and pairs naturally with the same
treatment `decisions/002` wants for "expensive".

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A2 -F 'Unbounded is not abstract' ring_trace/src/lib.rs
```

Live output:

```
//! Unbounded is not abstract: one producer recording as fast as it can has
//! been measured to grow the log at roughly 667 MiB/s of live entries — this
//! machine's figure, its order of magnitude rather than its exact number is
```

**Disposition:** applied — the module doc's "Trace against stats" section now
carries the measured growth rate (≈667 MiB/s from one producer) beside the
"grows without bound" claim, with the same this-machine's-figure caveat given
the "expensive" treatment in `decisions/002`. `cargo test --release -p
ring_trace --doc` confirms 9/9 doctests still pass. Now prints: `Unbounded is not abstract`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/001`](../decisions/001_the_flag_is_fixed_at_construction.md) | Why disabled is absorbing |
| [`api/002`](../api/002_shared_reference_everywhere_and_what_it_forces.md) | Who can take the `clear` edge |
| [`data_structure/001`](../data_structure/001_forty_bytes_a_flag_and_a_vector_that_never_allocated.md) | The 24 bytes this rate is counted in |
| [`lifecycle/002`](002_a_finished_crate_in_the_unverified_stage.md) | The crate's own documentary lifecycle |

### Sources

| Fact | Where |
|------|-------|
| Two constructors, no third flag writer | `ring_trace/src/lib.rs:211-223` |
| Two log writers: `push` and `clear` | `ring_trace/src/lib.rs:262`, `:365` |
| No `Drop`, no flush path | Census above |
| `entries()` the only way out | `ring_trace/src/lib.rs:318` |
| The "grows without bound" claim | `ring_trace/src/lib.rs:19` |
| 667 and 681 MiB/s, one producer | Probe above, two runs |
| 186–202 MiB/s, four producers | Probe above, two runs |

### Tests

| Test | Covers |
|------|--------|
| `clearing_empties_the_log_without_switching_it_off` | The populated→empty edge |
| `clearing_a_disabled_trace_is_harmless` | That the edge is a no-op in the absorbing state |
| `the_default_trace_is_off` | Which state construction lands in by default |
| `an_enabled_trace_records_exactly_one_entry_per_operation` | The empty→populated edge |
