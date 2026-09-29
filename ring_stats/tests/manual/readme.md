# ring_stats — manual testing plan

`tests/stats_test.rs` asserts the counters count. This plan covers the two
claims the crate makes that no assertion can reach: that the counters are cheap
enough to leave on permanently, and that `Relaxed` is a deliberate choice with a
stated consequence rather than the default someone reached for.

Run from the workspace root.

## M1 — every counter is `Relaxed`, with the reason stated

A statistics counter must not introduce ordering the algorithm did not already
need. If one counter quietly uses `SeqCst`, it puts a fence on the hot path of
every publish, and the cost is invisible until someone profiles.

```bash
grep -n "Ordering::" ring_stats/src/lib.rs | sort | uniq -c
```

**Expected:** `Ordering::Relaxed` and nothing else. Then check the reason is
written down:

```bash
grep -n -i "relaxed\|ordering\|diagnostic" ring_stats/src/lib.rs | head
```

**Expected:** the module doc says a stats read is a diagnostic, never a
synchronisation point.

## M2 — the tests claim only what `Relaxed` actually guarantees

This is the check most likely to find something. `Relaxed` guarantees each
counter's own atomicity and monotonicity; it does **not** guarantee a coherent
snapshot across two counters mid-run. A test asserting `published == consumed`
while writers run would be asserting something `Relaxed` does not provide, and
would pass on x86 and fail on ARM.

List the concurrent tests, then read each one — this is a judgement about where
an assertion sits relative to a scope boundary, and no grep decides it. A range
expression like `awk '/thread::scope/,/^}/'` looks like it would, but its end
pattern matches the first closing brace at column 0 and then restarts, so it
silently mixes in-scope and post-scope assertions and reports them as if they
were all inside.

```bash
grep -n "thread::scope" ring_stats/tests/stats_test.rs
```

Then read each named test in full.

**Expected:** every assertion made *while writers are running* concerns one
counter only. Cross-counter assertions (`in_flight`, `dropped_total`) appear
only after `thread::scope` returns, where the join has established
happens-before for every write.

## M3 — the per-policy breakdown cannot be collapsed by accident

The module doc's argument is that a single `dropped` counter cannot distinguish
a ring dropping newest from one evicting oldest. That argument only holds if the
breakdown is structurally enforced.

```bash
grep -nE -A 10 "pub fn record_drop" ring_stats/src/lib.rs
```

**Expected:** the policy selects the counter by `match`, exhaustively — so
adding a fourth policy is a compile error here rather than a silently
uncounted drop.

## M4 — `dropped_total` and the breakdown cannot drift

```bash
grep -nE -A 6 "pub fn dropped_total" ring_stats/src/lib.rs
```

**Expected:** the total is *derived* by summing over `OverflowPolicy::ALL`, not
maintained as a separate counter. A separate counter is one more thing to forget
to increment.

## M5 — the doc examples are the API's first reader

```bash
cd ring_stats && cargo test --doc
```

**Expected:** every example passes and reads as an explanation on its own.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | 11 `Ordering::` occurrences, all `Relaxed`, none else. Module doc lines 13–14 state the reason: "a stats read is a diagnostic, never a synchronisation point". |
| 2026-08-28 | M2 | ✅ | Three tests use `thread::scope`. The only assertion *inside* a scope is `now >= last` in `each_counter_is_monotone_while_writers_run` — single-counter monotonicity, exactly what `Relaxed` provides. Every cross-counter read (`in_flight`, `dropped_total`) sits after the scope returns. The check's original `awk` range was unsound and is replaced above with "list, then read" — it had reported post-scope assertions as if they were inside. |
| 2026-08-28 | M3 | ✅ | `record_drop` selects by exhaustive `match` on `OverflowPolicy`; a fourth policy would be a compile error here, not an uncounted drop. |
| 2026-08-28 | M4 | ✅ | `dropped_total` sums over `OverflowPolicy::ALL` — derived, not a fourth stored counter. |
| 2026-08-28 | M5 | ✅ | 10 doc tests pass. |
