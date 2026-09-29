# pitfall

The crate has two failure paths and they point in opposite directions. One is
guarded carefully, argued at length in the module documentation, and cannot be
reached through the public API at all. The other is unguarded, undocumented,
untested, and reachable in three lines from a public constant the family
publishes by name.

Which of the two got the attention is the shape worth recording. The unreachable
one is the interesting engineering problem — poisoning, five inconsistent sites,
a real bug found by reading them side by side. The reachable one is a `+`.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_range_that_reads_backwards.md) | A Range That Reads Backwards | What `Display` prints at the top of the sequence range, and how far the suite went |
| [002](002_a_recovery_no_caller_can_reach.md) | A Recovery No Caller Can Reach | Why the poisoning branch cannot fire, and what the family's other three lock sites say instead |

## The One That Prints

`TraceEntry::end` is `Seq( self.seq.0 + self.count as u64 )`, and `Display` puts
its result on the right of a `..`. At the top of the range that produces
`publish 18446744073709551615..0` in release and a panic mid-format under debug
assertions — in the crate's only rendering of a log line, read by someone already
investigating a failure.

`ring_mpsc` publishes `Seq( u64::MAX )` as `UNSTAMPED`, so the value takes one
`use` rather than `2^64` publications. Nothing guards the addition: no `# Panics`
section anywhere in the file, no `debug_assert`, and across all 33 crates zero
calls to `checked_add`, `saturating_add` or `wrapping_add`.

## The One That Cannot

`entries_guard` recovers a poisoned lock rather than propagating it, and the
recovery cannot fire, because no guard leaves the crate and no caller code runs
under the lock — there is no closure, generic parameter or trait object anywhere
in the file. That is a real safety property and it comes from the API's shape
rather than from the branch, which means the next method that hands out a guard
makes the cold branch live in the same commit.

The family's other three lock sites state exactly that argument, in one clause
inside an `expect` message: nothing panics while holding the lock. This crate's
eight lines of prose argue from the data structure instead — a `Vec` has no
half-established invariant — which is true, and is the weaker of the two facts it
has available.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the reachable failure: an addition with no guard --'
sed -n '/^    Seq( self\.seq\.0 + self\.count as u64 )$/p;/^    write!( f, "{} {}\.\.{}", self\.op, self\.seq\.0, self\.end()\.0 )$/p' ring_trace/src/lib.rs
printf '    the sentinel that reaches it: %s\n' \
  "$( command grep -h 'pub const UNSTAMPED' ring_mpsc/src/lib.rs )"
printf '    checked/saturating/wrapping adds in 33 crates: %s   Panics sections in ring_trace: %s\n' \
  "$( command grep -rc 'checked_add\|saturating_add\|wrapping_add' --include=*.rs ring_*/src/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )" \
  "$( command grep -c '# Panics' ring_trace/src/lib.rs || true )"
echo '  -- the unreachable failure: a recovery with no route to it --'
printf '    public methods returning a guard: %s   callbacks runnable under the lock: %s\n' \
  "$( command grep -c 'pub fn [a-z_]*(.*MutexGuard' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'FnOnce\|FnMut\|impl Fn' ring_trace/src/lib.rs || true )"
command grep -rn '\.lock()' --include=*.rs ring_*/src/ ring_*/tests/ \
  | sed 's|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||' | command grep -o '^[a-z_]*/[a-z]*/[a-z_]*\.rs:[0-9]*' | sed 's/^/    /'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR41 | `ring_trace` | **latent hazard** | `TraceEntry::end` is a bare `+` and `Display` puts its result on the right of a `..`, so at the top of the sequence range the crate's only rendering of a log line becomes `publish 18446744073709551615..0` in release — an upper bound below its lower bound — and panics mid-format under debug assertions, which three other crates already record for the arithmetic but none of them for the output a human reads; the value is not hypothetical, `ring_mpsc` publishing `UNSTAMPED : Seq = Seq( u64::MAX )` as its "never published" sentinel, so a probe reaches it with one `use`, one `record` and one `to_string`, and `record` accepts it without complaint since it checks `self.enabled` and nothing else before pushing all three values as handed to it, leaving the entry stored and ordinary-looking until someone reads it — and the failure is exactly the one a prior design record sets the crate's standard against, a diagnostic that must not "kill the producer thread it was added to observe while telling its reader nothing had happened", where a panicking `Display` does the first and a backwards range does the second; either `saturating_add` in `end` or a `Display` that prints the count instead of a computed bound when the addition would carry keeps the log printable at the one value the family hands out by name |
| TR42 | `ring_trace` | n/a — coverage | `an_entry_reports_the_range_it_covers` asserts two cases, an ordinary `count : 3` from `Seq( 5 )` ending at `Seq( 8 )` and a deliberate degenerate one, a zero-count entry asserted to end where it starts with the reason written into the assertion message, "a zero-count operation covers nothing" — so boundary behaviour was considered, the low end picked, asserted and explained, and the other degenerate end is one line away and absent, with `entries_compare_by_value_and_print_readably` asserting only `"drop 1..2"` and `"claim 8..72"`; nothing in the crate tests a `seq` or `count` near the top of `u64` and nothing warns about one, there being no `# Panics` section in the file, no `debug_assert`, and across all 33 crates zero calls to `checked_add`, `saturating_add` or `wrapping_add`, so the family's arithmetic posture is a bare `+` throughout with two crates carrying a `debug_assert` about something else — and the missing case takes the same three lines and the same `TraceEntry` literal the test already builds, at the one end whose behaviour changes by build profile |
| TR43 | `ring_trace` | **latent hazard** | The poisoning recovery in `entries_guard` cannot fire through the public API: a `Mutex` is poisoned only by a panic unwinding while a guard is alive, and every guard this crate creates is created and dropped inside one of five short methods that call nothing the caller supplies — the accessor is private, no method returns a `MutexGuard`, `entries()` hands back a clone rather than a borrow, and there is no closure, generic parameter or trait object anywhere in the file, so a probe attempting both routes finds a thread panicking after recording leaves the trace reporting both entries and still working, and a caller panicking while holding what `entries()` returned leaves `len` and `count_of` unaffected because the lock was released before the clone changed hands; the plan reaches the same conclusion from the other side, "No test can reach the condition, which is exactly why reading the five sites side by side was the only instrument that could find it" — and this is recorded as a hazard rather than as dead code because what makes the branch unreachable is the API's shape rather than the branch, so an obvious future addition such as `pub fn with_entries( &self, f : impl FnOnce( &[ TraceEntry ] ) )` makes poisoning reachable in the same commit that introduces it and turns a cold correct branch live and untested at that moment, which one clause on `entries_guard` naming the precondition would prevent |
| TR44 | `ring_trace` | n/a — inconsistency | The family takes a lock in four places and three of them assert the condition cannot arise while saying why in the message — `ring_bench/src/lib.rs:1066` writes `.expect( "no producer panics while holding the lock" )` and `ring_mpsc/tests/mpsc_test.rs:156` and `:202` write `.expect( "no panic while holding the lock" )` — a reachability argument stated in one clause at the site; `ring_trace` is the fourth and argues from the data instead, eight lines of module documentation explaining that "a `Vec<TraceEntry>` has no invariant a panic could leave half-established — a push either landed or it did not — so there is nothing for poisoning to protect" before ruling out the two alternatives that were "briefly in this file", every word of which is right and none of which is the reachability argument; the behaviour differing is legitimate, a diagnostic having to survive what it observes where a harness should fail loudly, but the reason the case never comes up does not differ and is written down at three sites out of four, and adding it here would also make the accessor's doc self-contained, since it currently defers the whole question to a module doc that answers a different question than a reader of `entries_guard` is asking |
