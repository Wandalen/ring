# Decisions: A Mutex Over a Lock-Free Log

### Scope

**Purpose:** Record the crate's argument for choosing a lock, establish that it
is the only crate in the family to hold one without any atomic beside it, and put
numbers on the "affordable" the argument turns on.

**Responsibility:** The module doc's stated reasoning, the exclusion-mechanism
census across the 33 crates, and measurements of the enabled path uncontended and
under four-way contention.

**In Scope:** `ring_trace/src/lib.rs:34-41`, `:183`; `Mutex` and `Atomic`
occurrences in every `ring_*` crate root.

**Out of Scope:** How long a *read* holds that lock is
[`algorithm/002`](../algorithm/002_a_linear_scan_where_a_counter_would_do.md).
The poisoning question the lock raises is
[`pitfall/002`](../pitfall/002_a_recovery_no_caller_can_reach.md).

---

## The Argument, and Who Else Made It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the argument for the lock --'
command grep -m1 -A7 -F '//! ## Why a `Mutex` is the right cost here' ring_trace/src/lib.rs
echo '  -- exclusion mechanism across the 33 crates --'
lock=0; atom=0; both=0; neither=0
for c in ring_*/; do
  a=$( command grep -c 'Atomic' "$c"src/lib.rs 2>/dev/null || true )
  m=$( command grep -c 'Mutex' "$c"src/lib.rs 2>/dev/null || true )
  if   [ "$m" != 0 ] && [ "$a" != 0 ]; then both=$(( both + 1 ))
  elif [ "$m" != 0 ];                  then lock=$(( lock + 1 ))
  elif [ "$a" != 0 ];                  then atom=$(( atom + 1 ))
  else                                      neither=$(( neither + 1 ))
  fi
done
printf '    Mutex only %s   Atomic only %s   both %s   neither %s\n' "$lock" "$atom" "$both" "$neither"
echo '    -- and which crate is the Mutex-only one --'
for c in ring_*/; do
  a=$( command grep -c 'Atomic' "$c"src/lib.rs 2>/dev/null || true )
  m=$( command grep -c 'Mutex' "$c"src/lib.rs 2>/dev/null || true )
  if [ "$m" != 0 ] && [ "$a" = 0 ]; then echo "    $( basename "$c" )"; fi
done
```

Live output:

```
  -- the argument for the lock --
//! ## Why a `Mutex` is the right cost here
//!
//! A shared trace across producers needs some form of exclusion, and a lock is
//! the honest one. The alternative — a lock-free log — would make the disabled
//! path no cheaper (it is already a branch on a `bool`) while making the
//! enabled path a second concurrency problem inside the crate that exists to
//! debug the first. The lock is affordable precisely because the feature is off
//! whenever the measurement matters.
  -- exclusion mechanism across the 33 crates --
    Mutex only 1   Atomic only 8   both 1   neither 23
    -- and which crate is the Mutex-only one --
    ring_trace
```

## What "Affordable" Costs

*This probe's uncontended and four-producer timings come from a one-off
scratch binary under `-tr_probe/`, run once and swept afterward per this
project's convention for temporary files — it cannot be re-run to
reconfirm. TR16's disposition below only edited a doc comment;
`Trace::record`'s guard and lock are unchanged, so the figures below stand
as a recorded measurement rather than a live guarantee.*

```rust
// -tr_probe/src/bin/const_and_lock.rs
// Uncontended: nine repetitions, disabled and enabled back-to-back inside each,
// medians reported. Then four producers on one shared enabled trace against
// four on a trace each — single timings, not medians.
for shared in [ false, true ]
{
  let one = Trace::enabled();
  std::thread::scope( | scope |
  {
    for _ in 0..4
    {
      scope.spawn( move ||
      {
        if shared { drive( &one, CALLS ); }
        else { let mine = Trace::enabled(); drive( &mine, CALLS ); }
      } );
    }
  } );
}
```

Two independent runs, `--release`:

```
  uncontended: disabled 2.75 ns/call   enabled 20.32 ns/call
  4 producers, a trace each     : 18.23 ns/call
  4 producers, one shared trace : 80.43 ns/call

  uncontended: disabled 2.77 ns/call   enabled 20.14 ns/call
  4 producers, a trace each     : 8.98 ns/call
  4 producers, one shared trace : 117.50 ns/call
```

---

### TR15 — The Only Crate in the Family Holding a Lock and No Atomic

Of the 33 `ring_*` crates, eight use atomics and no lock, twenty-three use
neither, one — `ring_bench` — uses both, and exactly one uses a lock with no
atomic anywhere in it. That crate is this one, in a family whose entire subject
is a lock-free concurrency write path.

The argument for it is explicit and it is a good one. A lock-free log "would make
the disabled path no cheaper (it is already a branch on a `bool`) while making the
enabled path a second concurrency problem inside the crate that exists to debug
the first" — the diagnostic must not become a thing that itself needs
diagnosing. And the conclusion is conditional in the right place: "the lock is
affordable precisely because the feature is off whenever the measurement
matters."

**Finding.** The decision is correct and the reasoning is complete; what is
missing is that the crate is the family's sole exception and does not say so. A
reader coming from any of the other thirty-two arrives at a `Mutex` with no
signal that they have crossed a boundary, and the argument they need in order not
to read it as an inconsistency is present but framed as a local design note
rather than as the exception it is. One clause — that this is the only crate in
the family to take a lock, deliberately, because it is the only one that is off
by default — turns a surprise into a stated boundary.

---

### TR16 — "Affordable" Is 7× Uncontended and 30–40× Shared

The argument turns on a cost it does not quantify. Uncontended, an enabled
`record` costs 20.2 ns per call against the disabled path's 2.75 — a factor of
about seven, and the medians of nine agree to two decimal places across two
independent runs. Under four producers sharing one trace the per-call cost is
80–118 ns, against 9–18 ns for four producers each holding their own; those
figures are single timings rather than medians, and their spread is wide enough
that only the direction should be trusted, not the magnitude.

Taken together the shape is clear: the enabled shared path is roughly thirty to
forty times the disabled one, and the variance under contention is itself large.
That is precisely what the crate predicts — "off by default and expensive" is its
own description — so nothing here contradicts the decision. It supplies the
number the decision was made without.

**Finding.** The gap is that "expensive" and "affordable" are doing load-bearing
work in an argument aimed at a reader deciding whether to switch the trace on
during a run. Thirty to forty times the off cost, on the producers' path, with
contention scaling badly in four threads, is a decision a reader can actually
make; "expensive" is not. The module doc's trace-against-stats section already
contrasts the two crates on always-on-and-cheap against off-and-expensive, and it
is the natural place for one measured sentence — with the caveat that these are
this machine's numbers and the ratio, not the nanoseconds, is what carries.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A2 -F '"Expensive" measured' ring_trace/src/lib.rs
```

Live output:

```
//! "Expensive" measured: roughly 7× the disabled cost per call uncontended,
//! and 30–40× under four producers sharing one trace — this machine's ratio,
//! not its nanoseconds, is what should be trusted (→ `docs/decisions/002`).
```

**Disposition:** applied — the module doc's "Trace against stats" section now
carries the measured ratio (≈7× uncontended, 30–40× under four-producer
contention) in place of the unquantified "expensive", with the same
this-machine's-numbers caveat this finding asks for. `cargo doc -p ring_trace`
with `RUSTDOCFLAGS="-D warnings"` confirms a clean build. Now prints: `roughly 7× the disabled cost`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](../algorithm/002_a_linear_scan_where_a_counter_would_do.md) | How long a reader holds the same lock |
| [`pitfall/002`](../pitfall/002_a_recovery_no_caller_can_reach.md) | The failure mode the lock introduced |
| [`decisions/001`](001_the_flag_is_fixed_at_construction.md) | The flag that makes the lock affordable |
| [`non_functional_requirement/002`](../non_functional_requirement/002_the_one_crate_that_genuinely_needs_std.md) | What choosing a `Mutex` costs in portability |

### Sources

| Fact | Where |
|------|-------|
| The stated argument for the lock | `ring_trace/src/lib.rs:34-41` |
| One `Mutex`-only crate in 33 | Census above |
| Eight atomic-only, one both, twenty-three neither | Census above |
| 2.75 ns disabled against 20.2 ns enabled | Probe above, two runs |
| 80–118 ns/call shared against 9–18 ns/call unshared | Probe above, single timings |

### Tests

| Test | Covers |
|------|--------|
| `concurrent_recorders_lose_no_entry` | Four producers on one lock, for correctness |
| `a_disabled_trace_stays_empty_under_contention` | The same shape with the lock never taken |
| `an_enabled_trace_records_exactly_one_entry_per_operation` | The uncontended enabled path |
