# pattern

Two patterns carry this crate, one at run time and one at compile time, and both
are unusually well argued. Five methods reach the log through a single private
accessor so no two can disagree about a poisoned lock. One `match` is written
exhaustively rather than derived so a new operation cannot be added without
someone naming it.

Both are also incomplete in the same direction. The runtime pattern is applied to
the lock and not to the flag, whose invariant is the crate's headline claim. The
compile-time pattern is stated twice — correctly in the source, falsely in the
test — in a family that has already established, in two other crates, how a
compile-time claim is supposed to be discharged.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_one_accessor_for_five_lock_sites.md) | One Accessor for Five Lock Sites | The consolidation, the history behind it, and the decision it was not applied to |
| [002](002_exhaustive_match_as_a_tripwire.md) | Exhaustive Match as a Tripwire | The family's three treatments of compile-time claims, and the wildcards that undo them |

## A Pattern Whose Evidence Is in the Plan

`entries_guard` is the crate's one private inherent helper, and all five methods
that touch the log go through it. Its doc gives the pattern — "so no two of them
can disagree about what a poisoned lock means" — in the subjunctive. The plan
gives the indicative: they did disagree, in four distinct ways, one of which
"would have killed the producer thread it was added to observe while telling its
reader nothing had happened".

The same shape is not applied to `enabled`, checked inline in `record` with no
accessor around it — which is exactly the arrangement the lock had before
consolidation, and which leaves the disabled-means-empty invariant resting on a
lone `if` that the crate's own pattern was invented to eliminate.

## Three Ways to Assert What the Compiler Does

`ring_flush` states a claim and quotes the compiler's actual notes back. `ring_poll`
states a claim it refuses to make, explains that the code would fail to *resolve*
rather than fail to compile, and names the honest substitute. `ring_trace` makes
two claims: the source's is correct, and the test's is the only compile-time claim
in the family that was neither measured nor examined — and the only one that turns
out to be false.

The tripwire also depends on there being no wildcard arm, and this crate has none.
Two elsewhere in the family — over `Resolution` and over `FlushOutcome` — turn a
future variant into `Err( record )` and `0` respectively; a third, over
`FlushPolicy`, is forced by guarded arms and is not the same thing.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the runtime pattern, and the decision it skips --'
printf '    methods reaching the log through entries_guard: %s\n' \
  "$( command grep -c 'self.entries_guard()' ring_trace/src/lib.rs || true )"
printf '    accessors wrapping the enabled flag: %s   inline checks: %s\n' \
  "$( command grep -c 'fn if_enabled\|fn when_enabled' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'if !self.enabled' ring_trace/src/lib.rs || true )"
echo '  -- the compile-time claims in the family --'
command grep -rn 'fails to compile' --include=*.rs ring_*/src/ ring_*/tests/ \
  | command grep -o '^[a-z_]*/[a-z]*/[a-z_]*\.rs:[0-9]*' | sed 's/^/    /'
echo '  -- and the wildcard arms that would undo the idiom --'
command grep -n '_ =>' ring_core/src/lib.rs ring_flush/src/lib.rs | sed 's/^/    /'
printf '    wildcard arms in ring_trace: %s\n' "$( command grep -c '_ =>' ring_trace/src/lib.rs || true )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR37 | `ring_trace` | n/a — doc gap | `entries_guard` is the crate's one private inherent helper and all five log-touching methods go through it, with its doc stating the pattern exactly — "The single access point every method below goes through, so no two of them can disagree about what a poisoned lock means" — but not stating that they did: a prior design record documents five sites and four distinct behaviours before consolidation, `record` panicking, `len` and `count_of` silently reporting zero, `entries` returning empty, `clear` doing nothing, one of which "would have killed the producer thread it was added to observe while telling its reader nothing had happened"; that is the strongest argument the pattern has and it is concrete rather than hypothetical, yet the crate's own version is subjunctive — no two methods *can* disagree — which reads as a precaution taken in advance, so a maintainer weighing whether to inline the accessor back into two call sites meets the precaution and not the history, and two clauses fix it: that the five sites did disagree in four ways, and that inlining reintroduces the same failure mode |
| TR38 | `ring_trace` | n/a — unadopted | The crate has two decisions every method must respect — what a poisoned lock means, and whether the trace is enabled — and routes the first through one accessor deliberately and with a stated reason while checking the second inline, once, in `record`, with no accessor and no other consulting site; that is correct today because one site is all there is, and it is also exactly the arrangement the lock had before consolidation, leaving the headline invariant resting on a lone `if` that nothing prevents a future writer from skipping, made easier by `entries_guard` handing any such writer a `MutexGuard< Vec< TraceEntry > >` with the whole `Vec` API and no memory of the flag — so the easy way to add a second insertion is also the way that bypasses the gate; the crate already owns the fixing shape twelve lines away, whether as a private `push_if_enabled` or merely a sentence on `entries_guard` saying inserting callers must check `self.enabled` first, and the observation is that it solved this class of problem once, wrote down why, and did not carry the solution to the neighbouring decision where the consequence is the crate's central claim |
| TR39 | `ring_trace` | n/a — inconsistency | Four places in the family assert something about what will or will not compile, and three handle it in a way worth copying: `ring_flush` states a claim and discharges it by measurement, "the second was **measured** rather than assumed: adding `fn assert_sync< T : Sync >(){}` here and calling it with this type fails to compile, and the compiler names the reason", followed by the compiler's actual notes quoted; `ring_poll` states a claim it refuses to make, "**No test that a parking call fails to compile.** It would not fail to compile; it would fail to *resolve*, because the crate is not a dependency", then names a manifest scan as the honest substitute; `ring_trace` makes two, of which the source's is correct and the test's is shown false by a compiled counterexample in `item/002` — the only compile-time claim in the family neither measured nor examined, and the only one that turned out to be wrong, in a family whose standard is already written down twice in two forms, quote the compiler or explain why you cannot, either of which would have caught it because attempting to produce the error is exactly what does not happen when the error does not exist |
| TR40 | `ring_trace` | n/a — observation | The tripwire idiom is only as strong as the absence of wildcard arms, and this crate has none anywhere, while two remain elsewhere in the family and are not equivalent to each other: `ring_flush:226` matches a `FlushOutcome` for its count and reads `_ => 0`, so a new outcome that moved records silently reports moving none, over a family-local enum, turning a would-be compile error into a plausible-looking default and carrying no comment saying the default is deliberate; `ring_flush:576` is different and should not be counted with it, its arms being guarded (`FlushPolicy::OnFull if self.buffer.is_full()` and two more) and guarded arms never contributing to exhaustiveness, so its wildcard is required by the language rather than chosen and removing it would not compile — the useful distinction being that the forced one needs nothing while the chosen one needs either an enumerated arm per variant or one comment naming which future variants the default is correct for, and the third the census originally found, `ring_core:394`, was corrected under that crate's CO1 |
