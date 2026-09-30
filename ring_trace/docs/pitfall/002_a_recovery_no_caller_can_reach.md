# Pitfall: A Recovery No Caller Can Reach

### Scope

**Purpose:** Establish that the crate's poisoning-recovery branch cannot be
reached through the public API, identify the structural property that makes it
unreachable, and place the crate's stated rationale beside the three other lock
sites in the family, which state a different one.

**Responsibility:** `entries_guard`'s recovery, what the API does and does not
hand out, the module doc's argument for recovery over propagation, and the
family's four lock sites.

**In Scope:** `ring_trace/src/lib.rs:43-51`, `:265-280`;
`ring_bench/src/lib.rs:1066`; `ring_mpsc/tests/mpsc_test.rs:156`,
`:202`.

**Out of Scope:** The single-access-point pattern itself is
[`pattern/001`](../pattern/001_one_accessor_for_five_lock_sites.md). Why there is
a lock at all is
[`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md).

---

## What the API Hands Out, and What Everyone Else Does

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the recovery, and the one place it is reachable from --'
command grep -m1 -A16 -F '  /// The log, with poisoning recovered rather than propagated.' ring_trace/src/lib.rs
printf '    public methods that return a guard: %s   private ones: %s\n' \
  "$( command grep -c 'pub fn [a-z_]*(.*MutexGuard\|pub const fn [a-z_]*(.*MutexGuard' ring_trace/src/lib.rs || true )" \
  "$( command grep -c '^  fn [a-z_]*(.*MutexGuard' ring_trace/src/lib.rs || true )"
printf '    callbacks or generic parameters record could invoke under the lock: %s\n' \
  "$( command grep -c 'FnOnce\|FnMut\|Fn(\|impl Fn' ring_trace/src/lib.rs || true )"
echo '  -- and how the family treats a poisoned lock everywhere else --'
command grep -r '\.lock()' --include=*.rs ring_*/src/ ring_*/tests/ | sed 's|ring/||' | sed 's/^/    /'
```

Live output:

```
  -- the recovery, and the one place it is reachable from --
  /// The log, with poisoning recovered rather than propagated.
  ///
  /// The single access point every method below goes through, so no two of
  /// them can disagree about what a poisoned lock means — see the module
  /// documentation for why recovery is the right answer here.
  ///
  /// Safe only because no caller-supplied code ever runs while this guard is
  /// held: every method below creates and drops it inside a few plain
  /// statements, so the poisoning this recovers from cannot actually occur.
  /// A future method that hands out the guard itself, or takes a callback to
  /// invoke under the lock, would change that and needs its own reachability
  /// argument before it can rely on the same recovery. See `pitfall/002` TR43.
  fn entries_guard( &self ) -> std::sync::MutexGuard< '_, Vec< TraceEntry > >
  {
    self.entries.lock().unwrap_or_else( std::sync::PoisonError::into_inner )
  }

    public methods that return a guard: 0   private ones: 1
    callbacks or generic parameters record could invoke under the lock: 0
  -- and how the family treats a poisoned lock everywhere else --
    ring_bench/src/lib.rs:  let mut guard = queue.lock().unwrap_or_else( std::sync::PoisonError::into_inner );
    ring_trace/src/lib.rs:    self.entries.lock().unwrap_or_else( std::sync::PoisonError::into_inner )
    ring_bench/tests/bench_test.rs:/// every later `.lock()` from *any* thread fails, not just a retry from the
    ring_bench/tests/bench_test.rs:  queue.lock().unwrap().push_back( 1 );
    ring_bench/tests/bench_test.rs:      let _guard = queue_ref.lock().unwrap();
    ring_bench/tests/bench_test.rs:    drop( queue.lock().expect( "no producer panics while holding the lock" ) );
    ring_bench/tests/bench_test.rs:  let recovered = queue.lock().unwrap_or_else( std::sync::PoisonError::into_inner );
    ring_mpsc/tests/mpsc_test.rs:          granted.lock().unwrap_or_else( std::sync::PoisonError::into_inner ).extend( mine );
    ring_mpsc/tests/mpsc_test.rs:        received.lock().unwrap_or_else( std::sync::PoisonError::into_inner ).extend( drained );
    ring_mpsc/tests/mpsc_test.rs:          granted.lock().unwrap_or_else( std::sync::PoisonError::into_inner ).extend( mine );
    ring_mpsc/tests/mpsc_test.rs:        received.lock().unwrap_or_else( std::sync::PoisonError::into_inner ).extend( drained );
    ring_mpsc/tests/mpsc_test.rs:        let _guard = granted_ref.lock().unwrap();
    ring_mpsc/tests/mpsc_test.rs:      drop( granted.lock().expect( "no panic while holding the lock" ) );
    ring_mpsc/tests/mpsc_test.rs:    let recovered = granted.lock().unwrap_or_else( std::sync::PoisonError::into_inner );
```

## The Two Ways a Caller Might Expect to Poison It

*The scratch binary this probe ran as is gone, swept per this project's
convention for temporary `-tr_probe/` files, so the two poisoning attempts
below can't be re-run. Nothing has changed the behaviour they found: TR43's
disposition added a clause to `entries_guard`'s doc but left the accessor, its
five call sites and its reachability untouched, so a panicking recorder or a
panicking guard-holder still leaves the trace working exactly as recorded.
Read the output as preserved evidence, not a live rerun.*

```rust
// -tr_probe/src/bin/poison_reach.rs
// A thread records, then panics; then a caller panics while holding what
// `entries()` returned. Both `true` values below are "the panic happened",
// not "the lock was poisoned" — the lines after each say what the trace did.
let handle = scope.spawn( move ||
{
  shared.record( TraceOp::Claim, Seq( 1 ), 1 );
  shared.record( TraceOp::Publish, Seq( 1 ), 1 );
  panic!( "while using the trace" );
} );
handle.join().is_err()
```

```
  panicked inside a recording thread: true
  the trace afterwards:               len 2 entries 2
  panicked holding entries():         true
  the trace afterwards:               len 2 count_of(claim) 1
  record/clear/record still work:     len 1 enabled true
```

---

### TR43 — The Branch Is Correct, Unreachable, and Unreachable for a Reason Nothing States

`entries_guard` recovers a poisoned lock rather than propagating it, and the
recovery cannot fire. A `Mutex` is poisoned only by a panic that unwinds while
the guard is alive, and every guard this crate creates is created and dropped
inside one of five short methods that call nothing the caller supplies. The
accessor is private, no method returns a `MutexGuard`, `entries()` hands back a
clone rather than a borrow, and `record` takes three plain values — there is no
closure, no generic parameter, no trait object anywhere in the file that could
run caller code under the lock.

The probe attempts both routes a caller would try. A thread panics after
recording, and the trace afterwards reports both entries and keeps working. A
caller panics while holding what `entries()` returned, and `len` and `count_of`
are unaffected, because the lock was released before the clone was ever handed
over. Recording, clearing and recording again all still succeed. The plan reaches
the same conclusion from the other direction: "No test can reach the condition,
which is exactly why reading the five sites side by side was the only instrument
that could find it".

**Finding.** Recorded as a latent hazard rather than dead code, because the thing
that makes the branch unreachable is not the branch — it is the API shape, and
nothing says so. A future `pub fn with_entries( &self, f : impl FnOnce( &[ TraceEntry ] ) )`,
which is the obvious way to let a caller read the log without cloning it, makes
poisoning reachable in the same commit that introduces it, and the recovery that
has been sitting there correct-but-cold becomes live and untested at that moment.
One clause on `entries_guard` — that the lock is safe because no caller code runs
under it, so any method that hands out a guard or invokes a callback changes that
— turns an invisible precondition into a stated one, at no cost.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A5 -F '/// Safe only because no caller-supplied code ever runs while this guard is' ring_trace/src/lib.rs
```

Live output:

```
    /// Safe only because no caller-supplied code ever runs while this guard is
    /// held: every method below creates and drops it inside a few plain
    /// statements, so the poisoning this recovers from cannot actually occur.
    /// A future method that hands out the guard itself, or takes a callback to
    /// invoke under the lock, would change that and needs its own reachability
    /// argument before it can rely on the same recovery. See `pitfall/002` TR43.
```

**Disposition:** applied — added exactly the clause this finding names to
`entries_guard`'s doc: that the recovery is safe only because no caller-supplied
code runs while the guard is held, and that a future method handing out the
guard itself (or taking a callback to invoke under the lock) would need its own
reachability argument before relying on the same recovery. This is a doc-only
change — `entries_guard` itself, its five call sites, and its reachability are
unchanged — so no new test was added; the existing suite re-verifies the
crate still builds and passes with the doc attached. Verified via
`cargo test -p ring_trace --all-features`, 2026-09-04 — ring_trace's 20 unit
tests plus 9 doctests all pass. Now prints:
`Safe only because no caller-supplied code ever runs while this guard is`

---

### TR44 — Three of the Family's Four Lock Sites Give the Argument This One Does Not

The family takes a lock in four places. `ring_bench` writes
`.expect( "no producer panics while holding the lock" )`; `ring_mpsc`'s tests
write `.expect( "no panic while holding the lock" )` twice. All three assert that
the condition cannot arise, and all three say why in the message: nothing panics
under the lock. That is a reachability argument, stated in one clause, at the
site.

`ring_trace` is the fourth, and it argues instead from the data. Eight lines of
module documentation explain that "a `Vec<TraceEntry>` has no invariant a panic
could leave half-established — a push either landed or it did not — so there is
nothing for poisoning to protect", then rule out the two alternatives that were
"briefly in this file": panicking would "let a diagnostic kill the producer
thread it was added to observe", and reporting zero would make "nothing happened"
and "the log broke" indistinguishable. Every word of that is right, and none of
it is the reachability argument.

The crate has the stronger fact available and states the weaker one. "This `Vec`
has no broken invariant to protect" justifies recovering *if* poisoning happens.
"No caller code runs under this lock" says it cannot, which is what actually
holds and what the other three sites write down.

**Finding.** Recorded as an inconsistency in how one property is documented
across four sites rather than a defect at any of them — the behaviour differs
legitimately, since `ring_trace` is a diagnostic that must survive what it
observes while the other three are harnesses that should fail loudly. What does
not differ, and is stated at three sites out of four, is why the case never comes
up. Adding that clause here would also make the accessor's own doc self-contained,
since today it defers the entire question to the module doc — "see the module
documentation for why recovery is the right answer here" — and the module doc
answers a different question than the one a reader of `entries_guard` is asking.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/001`](../pattern/001_one_accessor_for_five_lock_sites.md) | The consolidation that produced this single site |
| [`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md) | Why there is a lock to poison |
| [`api/001`](../api/001_three_types_nine_attributes_and_one_private_item.md) | The privacy the unreachability rests on |
| [`pitfall/001`](001_a_range_that_reads_backwards.md) | The crate's other failure path, which is reachable |
| [`non_functional_requirement/002`](../non_functional_requirement/002_the_one_crate_that_genuinely_needs_std.md) | The `std` items this lock is made of |

### Sources

| Fact | Where |
|------|-------|
| The recovery and its doc | `ring_trace/src/lib.rs:265-280` |
| The module doc's rationale | `ring_trace/src/lib.rs:43-51` |
| No guard escapes, no callback under the lock | Census above |
| Both poisoning routes failing to poison | Probe above |
| The family's other three lock sites | `ring_bench/src/lib.rs:1066`, `ring_mpsc/tests/mpsc_test.rs:156`, `:202` |

### Tests

| Test | Covers |
|------|--------|
| `concurrent_recorders_lose_no_entry` | The lock under real contention |
| `a_disabled_trace_stays_empty_under_contention` | The same, with the lock never taken |
| `clearing_empties_the_log_without_switching_it_off` | One of the five sites the accessor serves |
