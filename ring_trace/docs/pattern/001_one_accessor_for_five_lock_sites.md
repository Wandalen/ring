# Pattern: One Accessor for Five Lock Sites

### Scope

**Purpose:** Record the crate's single-access-point pattern, establish that the
history justifying it lives outside the crate, and note that the same shape is
not applied to the crate's other shared decision — the one whose invariant has no
structural guard.

**Responsibility:** `entries_guard` and its five call sites, the plan's record of
what the five sites did before consolidation, the private-helper census across
the family, and how the enabled flag is checked by comparison.

**In Scope:** `ring_trace/src/lib.rs:265-280`; every `ring_*` `src/lib.rs`.

**Out of Scope:** Whether the recovery path can be reached is
[`pitfall/002`](../pitfall/002_a_recovery_no_caller_can_reach.md). The flag
invariant this pattern would guard is
[`invariant/001`](../invariant/001_disabled_means_zero_forever.md). The other
compile-time pattern is
[`pattern/002`](002_exhaustive_match_as_a_tripwire.md).

---

## The Accessor and What It Replaced

The plan quoted below, `docs/plan/008_ring_write_path_staged.md`, was reachable
only through the pre-extraction monorepo's `docs/` symlink and no longer exists
in a standalone `ring` checkout — that one line of the recipe cannot be re-run.
The Live output is kept as the historical record of what it said.

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the accessor, and what it says it is for --'
command grep -m1 -A9 -F '  /// The log, with poisoning recovered rather than propagated.' ring_trace/src/lib.rs
printf '    public methods that go through it: %s\n' \
  "$( command grep -c 'self.entries_guard()' ring_trace/src/lib.rs || true )"
echo '  -- what the plan records the five sites used to do --'
command grep 'poisoning was inconsistent' docs/plan/008_ring_write_path_staged.md \
  | fold -w 96 -s | sed 's/ *$//' | sed 's/^/    /'
echo '  -- private helpers, excluding trait impl methods, across the family --'
tot=0; with=0
for c in ring_*/; do
  k=$( command grep '^  fn [a-z_]*(' "$c"src/lib.rs 2>/dev/null \
       | command grep -vc 'fn fmt(\|fn default(\|fn clone(\|fn drop(\|fn next(\|fn eq(' || true )
  tot=$(( tot + k ))
  if [ "$k" != 0 ]; then with=$(( with + 1 )); fi
done
printf '    ring_trace: %s   crates with any: %s of 33   total: %s\n' \
  "$( command grep '^  fn [a-z_]*(' ring_trace/src/lib.rs | command grep -vc 'fn fmt(\|fn default(' || true )" \
  "$with" "$tot"
echo '  -- and whether the same shape guards the other shared decision --'
printf '    accessors wrapping the enabled flag: %s   inline checks of it: %s\n' \
  "$( command grep -c 'fn [a-z_]*_gate\|fn if_enabled\|fn when_enabled' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'if !self.enabled' ring_trace/src/lib.rs || true )"
```

Live output:

```
  -- the accessor, and what it says it is for --
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
    public methods that go through it: 5
  -- what the plan records the five sites used to do --
    - **`ring_trace` poisoning was inconsistent across five lock sites** — `record` panicked,
    `len`/`count_of` silently reported zero, `entries` returned empty, `clear` did nothing. A
    poisoned trace would have killed the producer thread it was added to observe while telling its
    reader nothing had happened. Consolidated into one recovering accessor. No test can reach the
    condition, which is exactly why reading the five sites side by side was the only instrument
    that could find it.
  -- private helpers, excluding trait impl methods, across the family --
    ring_trace: 1   crates with any: 13 of 33   total: 47
  -- and whether the same shape guards the other shared decision --
    accessors wrapping the enabled flag: 0   inline checks of it: 1
```

---

### TR37 — The Pattern's Justification Lives in the Plan, Not the Crate

`entries_guard` is the crate's one private inherent helper and every one of the
five methods that touch the log goes through it. Its doc states the pattern
exactly: "The single access point every method below goes through, so no two of
them can disagree about what a poisoned lock means."

What it does not say is that they did. The plan records the state before
consolidation, in detail: `record` panicked, `len` and `count_of` silently
reported zero, `entries` returned empty, `clear` did nothing — five sites, four
distinct behaviours, one of which would have "killed the producer thread it was
added to observe while telling its reader nothing had happened". That is the
strongest argument the pattern has, and it is a concrete, already-happened
argument rather than a hypothetical one.

It is also the only place that argument exists. The crate's version is written in
the subjunctive — no two methods *can* disagree — which reads as a design
precaution taken in advance. A maintainer weighing whether to inline the accessor
back into two call sites for readability meets the precaution and not the history.

**Finding.** Recorded as a good pattern whose evidence sits one document away, in
an external plan a reader of this crate has no reason to open. Two clauses
close it: that the five sites did in fact disagree, in four different ways, and
that inlining this accessor reintroduces the same failure mode. That converts a
stated intention into a stated constraint, which is what stops a future
simplification from undoing the fix.

---

### TR38 — The Same Pattern Is Not Applied to the Flag

The crate has two decisions that every method has to respect: what a poisoned
lock means, and whether the trace is enabled. It routes the first through one
accessor, deliberately and with a stated reason. The second is checked inline,
once, in `record` — there is no accessor wrapping it and no other site consults
it before touching the log.

That is correct today, because one site is all there is. It is also exactly the
arrangement the lock had before consolidation, and it leaves the crate's headline
invariant — a disabled trace's log is empty forever — resting on a lone `if` that
nothing prevents a future writer from skipping. `entries_guard` hands any such
writer a `MutexGuard< Vec< TraceEntry > >` with the whole `Vec` API and no memory
of the flag, so the easy way to add a second insertion is also the way that
bypasses the gate.

The crate already owns the shape that fixes it, in the same file, twelve lines
away. A private `fn push_if_enabled( &self, entry : TraceEntry )` — or simply a
sentence on `entries_guard` saying any caller that inserts must check
`self.enabled` first — makes the flag a single access point the way the lock is.

**Finding.** Recorded as an unadopted application of the crate's own pattern.
Nothing is broken; the observation is that the crate solved this exact class of
problem once, wrote down why, and did not carry the solution to the neighbouring
decision — where the same argument holds and the consequence
([`invariant/001`](../invariant/001_disabled_means_zero_forever.md)) is the
crate's central claim rather than a diagnostic detail.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](../invariant/001_disabled_means_zero_forever.md) | The invariant this pattern would guard |
| [`pitfall/002`](../pitfall/002_a_recovery_no_caller_can_reach.md) | Whether the recovered path can be reached at all |
| [`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md) | The lock the accessor wraps |
| [`api/001`](../api/001_three_types_nine_attributes_and_one_private_item.md) | The privacy that makes the pattern hold |

### Sources

| Fact | Where |
|------|-------|
| The accessor and its stated purpose | `ring_trace/src/lib.rs:265-280` |
| Five methods going through it | Census above |
| One private inherent helper in the crate | Census above |
| One inline flag check, no accessor | Census above |

### Tests

| Test | Covers |
|------|--------|
| `concurrent_recorders_lose_no_entry` | The lock the accessor takes, under contention |
| `clearing_empties_the_log_without_switching_it_off` | One of the five sites, still agreeing |
| `a_disabled_trace_records_zero` | The flag check that has no accessor |
