# decisions

Two design choices carry this crate, and both are argued for in the module
documentation rather than left to be inferred — which puts it ahead of most of
the family. The flag is frozen at construction because a trace switchable
mid-run would leave a hole that reads like an idle run. The log is behind a
`Mutex` because a lock-free alternative would make the crate that exists to
debug a concurrency problem into a second concurrency problem.

Both arguments are correct. What both lack is the same thing: the fact that makes
them exceptional. The frozen flag is one of a pair of doors and only this one was
locked; the lock is the only one in a family of thirty-three whose subject is
lock-free write paths, and the "affordable" it rests on is thirty to forty times
the cost it is being compared against.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_flag_is_fixed_at_construction.md) | The Flag Is Fixed at Construction | The immutability, its argument, and the `static` the constructors permit |
| [002](002_a_mutex_over_a_lock_free_log.md) | A Mutex Over a Lock-Free Log | The family's only lock-without-atomic crate, and what "affordable" measures |

## A Decision the Type Enforces

`enabled` is a private `bool` written in two places, both constructors. Zero
setters, zero `&mut self` methods, no interior mutability over the flag. The
decision does not depend on anyone remembering it, and the sentence justifying it
does three jobs at once — it explains the frozen flag, the disabled `Default`,
and why `clear` is documented as keeping the enabled state.

Its side effect is unclaimed. Both constructors are `const fn`, because
`Mutex::new` and `Vec::new` both are, so `static TRACE : Trace =
Trace::disabled();` compiles and works. No `const` or `static` of this type
exists anywhere in the family, and for a diagnostic that wants to be reachable
without threading a parameter through every signature, that is the obvious
deployment.

## The Exception Nobody Flagged

Eight of the thirty-three crates use atomics and no lock. Twenty-three use
neither. One uses both. Exactly one uses a lock with no atomic in it at all, and
it is this one — in a family about lock-free concurrency. The argument for the
exception is present and sound; the fact that it *is* the exception is not
stated, so a reader arriving from any other crate meets a `Mutex` with no signal
that they have crossed a boundary.

The cost the argument turns on is likewise unquantified. Measured: 20.2 ns
enabled against 2.75 ns disabled uncontended, medians of nine agreeing across two
runs; 80–118 ns per call for four producers sharing one trace against 9–18 ns for
four holding their own, single timings whose direction is trustworthy and whose
magnitude is not. Thirty to forty times is a number a reader can decide on.
"Expensive" is not.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the flag and its two writers, with no third --'
command grep -n 'enabled : ' ring_trace/src/lib.rs
printf '    setters: %s   &mut self methods: %s\n' \
  "$( command grep -c 'fn set_enabled\|fn enable(\|fn disable(' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'fn [a-z_]*( *&mut self' ring_trace/src/lib.rs || true )"
echo '  -- exclusion mechanism across the family --'
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
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR13 | `ring_trace` | n/a — observation | The enabled flag is a private `bool` written in exactly two places, both constructors, with zero setters and zero `&mut self` methods anywhere in the crate, so the decision is enforced by the type rather than by convention; the argument given for it is the strongest piece of reasoning in the crate — a trace switchable mid-run "would produce a log with a silent hole at the front, which reads exactly like a run where nothing happened early — the one misreading a diagnostic tool must not invite" — reasoning on how a diagnostic gets misread rather than how it gets used, and doing three jobs at once by also explaining the disabled `Default` and why `clear` keeps the enabled state; its only shortfall is that it is phrased as a fact about the flag, so nothing carries it to the adjacent operation that produces the identical hole |
| TR14 | `ring_trace` | n/a — unadopted | `enabled()` and `disabled()` are `const fn`, possible only because `Mutex::new` and `Vec::new` are both const-constructible, which makes `static TRACE : Trace = Trace::disabled();` legal — a process-wide trace with no lazy initialisation, no `OnceLock` and no first-use atomic guard, compiled and exercised in a probe that records through the enabled form; nothing declares such a binding anywhere, the census finding no `const` or `static` of this type across all 33 crates while all nineteen tests construct a fresh `Trace`, and of the crate's five `pub const fn` the only const-position use in the whole family is `TraceOp::ALL`, which is an item rather than a call — for a diagnostic that is off by default and wants reaching without threading a parameter through every signature, the process-wide `static` is the obvious deployment and one line on the constructors would name it |
| TR15 | `ring_trace` | n/a — doc gap | Of the 33 `ring_*` crates, eight use atomics and no lock, twenty-three use neither, `ring_bench` uses both, and exactly one uses a lock with no atomic anywhere in it — this crate, in a family whose entire subject is a lock-free concurrency write path; the argument for the exception is explicit and good, that a lock-free log "would make the disabled path no cheaper … while making the enabled path a second concurrency problem inside the crate that exists to debug the first", and it is correctly conditional, "affordable precisely because the feature is off whenever the measurement matters" — but the crate never says it is the family's sole exception, so a reader arriving from any of the other thirty-two meets a `Mutex` with no signal they have crossed a boundary and reads a stated design note where a stated boundary belongs |
| TR16 | `ring_trace` | **measured cost** | The lock's affordability is asserted and never quantified: measured, an enabled `record` costs 20.2 ns per call against the disabled path's 2.75, a factor of about seven, with medians of nine agreeing to two decimal places across two independent runs; under four producers sharing one trace the per-call cost is 80–118 ns against 9–18 ns for four producers each holding their own, single timings whose spread is wide enough that only the direction should be trusted — so the enabled shared path runs roughly thirty to forty times the disabled one with contention scaling badly at four threads, which is exactly what the crate predicts and never states, leaving "expensive" and "affordable" load-bearing in an argument aimed at a reader deciding whether to switch the trace on mid-run, where a ratio is decidable and an adjective is not |
