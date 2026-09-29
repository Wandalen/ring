# invariant

The crate has two properties worth calling invariants, and they behave very
differently. A disabled trace's log is empty for the life of the process — that
one holds unconditionally, is written down, and is asserted three ways. The
entries come back in the order they were recorded — that one holds exactly, and
"recorded" is not "happened", which the accessor's wording gets right and the
module doc's statement of purpose does not.

Between them they describe the crate's honesty profile: it is careful about the
property it is graded on and looser about the property it is *sold* on. The
suite is more careful than either, asserting under contention exactly the
multiset-preserving invariant that survives interleaving — in a `sort_unstable()`
call with no comment beside it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_disabled_means_zero_forever.md) | Disabled Means Zero Forever | The one gate holding it, and what an empty log cannot tell you |
| [002](002_the_order_is_the_thing_a_counter_lacks.md) | The Order Is the Thing a Counter Lacks | Lock order against sequence order, measured under four producers |

## An Invariant With One Enforcement Point

Five methods reach the log, all through the private `entries_guard`. One of them
inserts. One `if !self.enabled` exists in the crate, at that same site. So the
headline claim rests on a single early return, and `entries_guard` hands every
future caller a `MutexGuard< Vec< TraceEntry > >` with the whole `Vec` API on it
and no memory of the flag.

All three disabled tests drive the log through `record`, so they assert the gate
rather than the invariant — an ungated second writer added later would compile,
pass, and break the claim silently. A test that exercises every public method on
a disabled trace and asserts the log is still empty would be a statement about
the type instead of about one function.

## Two Orders That Coincide for One Producer

The module doc sells the crate on order: a counter "tells you nothing about which
sequences or in what order; a trace answers 'which operations, in what order'".
The log's order is the order producers won the lock. With one producer that is
the operation order; with four, measured against a shared atomic issuing
sequences the way a claim path would, 3,580 and 4,828 of 80,000 adjacent pairs
run backwards across two runs, stepping back as much as 4,654 sequences.

`entries()` already says it correctly — "every entry, in the order recorded" —
and `concurrent_recorders_lose_no_entry` sorts before comparing, because without
the sort it would fail on nearly every run. The knowledge is in the crate twice,
once as a careful phrase and once as a method call, and never as a sentence a
reader interpreting a log would meet.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the invariant with one enforcement point --'
printf '    log accesses: %s   of those, insertions: %s   enabled gates in the crate: %s\n' \
  "$( command grep -c 'entries_guard()' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'entries_guard().push' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'if !self.enabled' ring_trace/src/lib.rs || true )"
echo '  -- the two statements of the ordering property --'
command grep -m1 -F '//! order; a trace answers "which operations, in what order" and grows without' ring_trace/src/lib.rs
command grep -m1 -F '  /// Every entry, in the order recorded.' ring_trace/src/lib.rs
echo '  -- and the line where the suite concedes the difference --'
command grep -m1 -F '  seqs.sort_unstable();' ring_trace/tests/trace_test.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR21 | `ring_trace` | n/a — unenforced | Five methods reach the log and all go through the private `entries_guard`, exactly one of them inserts, and exactly one `if !self.enabled` exists in the crate — the same site — so "a disabled trace's log is empty forever" is a property of one early return rather than of the type, holding because `Vec::push` appears once in the file; `entries_guard` is what makes a second writer easy, handing any future caller a `MutexGuard< Vec< TraceEntry > >` with the whole `Vec` API and no memory of the flag, so a `record_batch` written the obvious way would compile, pass and break the headline claim silently, and all three disabled tests drive the log through `record` — 50 publishes, one of each of the five kinds, four threads contending — so they assert the gate rather than the invariant and would all still pass; either a comment on `entries_guard` saying inserting callers must check `self.enabled` and `record` is the only one today, or a test exercising every public method on a disabled trace and asserting the log is still empty, would make it structural |
| TR22 | `ring_trace` | n/a — doc gap | `Trace::disabled()` and `Trace::enabled()` on an idle run are indistinguishable from outside — `len()` 0, `is_empty()` true, `entries()` an empty `Vec` for both — and no method returning log content returns the flag beside it, while the crate has already reasoned about exactly this misreading better than most of the corpus, arguing that a mid-run switchable trace "would produce a log with a silent hole at the front, which reads exactly like a run where nothing happened early — the one misreading a diagnostic tool must not invite"; a whole-run hole is the same shape and is reachable by simply not calling `Trace::enabled()`, which the disabled `Default` makes the outcome of every unthinking construction, and although `is_enabled()` answers the question nothing pairs it with the log — `TraceOp` and `TraceEntry` both implement `Display` but `Trace` does not, so any dump is caller code that has to remember the flag on its own; a sentence at `len` or `entries` naming both readings, plus either a `Display for Trace` leading with the flag or a note that whole-trace formatting is the caller's job and must carry it, closes a corollary of the crate's own strongest argument that the crate never draws |
| TR23 | `ring_trace` | **misleading doc** | The module doc's contrast with `ring_stats` is the crate's statement of purpose — a counter "tells you nothing about which sequences or in what order; a trace answers 'which operations, in what order'" — from which a reader takes that the entries reconstruct the order operations happened in, but they reconstruct the order producers won the lock, and those coincide only for a single producer: measured against a shared atomic issuing sequences the way a real claim path would, four producers put 3,580 and 4,828 of 80,000 adjacent pairs out of sequence order across two runs, roughly one pair in twenty, with the log stepping backwards by as much as 4,654 sequences at the worst point against zero inversions for one producer; the cause is the unclosable gap between claiming a sequence and recording it, since `record` takes the sequence as an argument, and nothing here is a bug — but `entries()` already carries the exact version, "every entry, in the order recorded", load-bearing precisely because recorded is not happened, so the module doc's sentence is the overreaching one and it is the sentence a reader chooses the crate on |
| TR24 | `ring_trace` | n/a — doc gap | `entries_come_back_in_the_order_recorded` spawns no threads, records four operations from one and asserts they come back in that order with the comment "order is the only thing a trace has that a counter does not" — the strongest statement of the property, made in the only setting where it is unconditionally true — while `concurrent_recorders_lose_no_entry` runs four threads at 2,000 records each and asserts the count is 8,000 and that the sequence column, **sorted first**, is exactly `0..8000`; that `sort_unstable()` at line 305 is the crate's only acknowledgement anywhere that recorded order is not sequence order, since without it the assertion would fail on almost every run, so the author knew and the knowledge went into a method call rather than a sentence — and what the sorted assertion states is the invariant that does hold under contention, every claimed sequence appearing exactly once with none lost or duplicated regardless of interleaving, which deserves a comment above the sort and a sentence at `entries()` where a caller about to interpret a log will meet it |
