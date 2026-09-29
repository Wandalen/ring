# Decisions: Per-Policy Drops Delivered, and Five Methods Nobody Asked For

### Scope

**Purpose:** Record what this crate was originally asked to count, what the crate
counts instead, and where the difference shows up.

**Responsibility:** The feature's four named counters, the seven fields, and the
five public methods whose subject the feature never names.

**In Scope:** This crate's own originating requirement and design record;
the `RingStats` fields and the five methods that requirement never names —
`record_consume`, `consumed`, `dropped_total`, `in_flight` and `reset` — in
`ring_stats/src/lib.rs`.

**Out of Scope:** The ordering decision is
[`decisions/001`](001_relaxed_with_a_reason_that_covers_one_load.md). The feature's
lifecycle state — implemented and still marked planned — is
[`lifecycle/002`](../lifecycle/002_implemented_tested_and_still_planned.md).

---

## Four Counters Requested, Seven Stored, Five Methods Unnamed

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the feature asks for --'
command grep -m1 -F 'Per-ring counters for what actually happened: items claimed, items published, items dropped and under which policy, and nanoseconds spent waiting. They are cheap enough to leave on and are the only thing that distinguishes a ring under mild pressure from one that is quietly discarding traffic.' docs/feature/185_ring_stats.md
echo '  -- and its first occurrence, in the design record --'
command grep -m1 -F '12. Stats (claimed, published, dropped, wait_ns)' docs/initial_design_message/958_okay_great_let_work_out/message.md
echo '  -- what the crate stores --'
command grep -o '^  [a-z_]* : AtomicU64' ring_stats/src/lib.rs | sed 's/ : AtomicU64//'
echo '  -- the public methods whose names the feature never mentions --'
command grep -o '  pub fn \(record_consume\|consumed\|dropped_total\|in_flight\|reset\)' ring_stats/src/lib.rs
```

Live output:

```
  -- what the feature asks for --
Per-ring counters for what actually happened: items claimed, items published, items dropped and under which policy, and nanoseconds spent waiting. They are cheap enough to leave on and are the only thing that distinguishes a ring under mild pressure from one that is quietly discarding traffic.
  -- and its first occurrence, in the design record --
12. Stats (claimed, published, dropped, wait_ns)
  -- what the crate stores --
  claimed
  published
  consumed
  dropped_newest
  dropped_oldest
  failed
  wait_nanos
  -- the public methods whose names the feature never mentions --
  pub fn record_consume
  pub fn consumed
  pub fn dropped_total
  pub fn in_flight
  pub fn reset
```

---

### ST15 — "And Under Which Policy" Is the One Requirement With a Design Consequence, and It Was Carried Out

The feature's phrasing is worth reading closely: "items dropped **and under which
policy**". The design record it descends from is plainer still — `Stats (claimed,
published, dropped, wait_ns)`, a single `dropped`. Somewhere between the original design record and
the feature instance, one clause was added, and it is the only clause in the
requirement that constrains storage rather than merely naming a number.

The crate carried it out completely. Three separate fields, indexed by
`OverflowPolicy` rather than by a shared bucket, with an exhaustive `match` in both
directions so a fourth variant breaks the build rather than silently merging
([`data_structure/002`](../data_structure/002_three_drop_counters_behind_one_enum.md)).
The module comment argues the case in its own words — a hundred dropped newest and a
hundred evicted oldest "are in completely different trouble" — and the storage
matches the argument exactly.

**Finding.** This is the crate's second recorded decision and, at the storage layer,
its most faithfully executed one. It is worth recording as such: a requirement whose
one hard clause was implemented past the letter, with the rationale restated in the
implementer's own terms rather than copied.

What happened above the storage is a separate matter — `dropped_total` puts the three
back into the single bucket the same paragraph argues against, and folds in a
counter that records refusals rather than losses. The decision was right and the
delivery at storage level was right; the surface undoes part of it.

---

### ST16 — Every Method-Specific Finding in This Corpus Names Something the Feature Never Asked For

Seven fields for four requested counters — the extra two are the drop split, which
the feature did ask for, and `consumed`, which it did not. Five public methods
concern subjects the requirement never names: `record_consume` and `consumed`,
`dropped_total`, `in_flight`, and `reset`.

Each addition is defensible on its own. A consumer-side counter completes the
lifecycle. A total is convenient. `reset` exists because `ring_shutdown` recycles
rings and needed it. None of them is a mistake.

**Finding.** They are, however, where everything is. The findings in this corpus
that name one method rather than the crate's shape as a whole are on `in_flight`
(the seam that can only under-report, and the direction the family's one caught bug
could not have been), `reset` (seven stores under a shared reference, with an
observable window), and `dropped_total` (the fold that erases the distinction the
storage was built to preserve). Three methods, none requested. The four counters the
feature actually named — claimed, published, dropped-per-policy, wait nanos — have
produced no method-specific finding at all: each is one `fetch_add` or one `load`,
does exactly what its name says, and is covered by a test that passes for the right
reason.

The pattern is not that the additions were wrong to add. It is that the requested
counters arrived with a written requirement to check them against, and the added ones
arrived with nothing — no line in this crate's own feature record, no entry in the design record, no
statement of what `in_flight` is for or what a zero from it means. The two whose
contracts turned out to need the most care are the two that were never specified.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/001`](001_relaxed_with_a_reason_that_covers_one_load.md) | The crate's other recorded decision, and the cost it did not measure |
| [`data_structure/002`](../data_structure/002_three_drop_counters_behind_one_enum.md) | The per-policy split as storage, and the total that undoes it |
| [`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) | The unrequested method that carries the crate's main hazard |
| [`lifecycle/002`](../lifecycle/002_implemented_tested_and_still_planned.md) | The feature instance itself, and the status it still carries |

### Sources

| Fact | Where |
|------|-------|
| The four requested counters | Census above |
| The plainer original, without the policy clause | Census above |
| Seven stored fields | Census above |
| Five methods on unnamed subjects | Census above |
| `reset`'s stated caller | `RingStats::reset`'s doc comment |
| The per-policy argument in the implementer's words | The module comment's per-policy paragraph |

### Tests

| Test | Covers |
|------|--------|
| `a_drop_lands_under_its_own_policy_only` | The requirement's one hard clause, at storage level |
| `the_drop_total_is_the_sum_over_every_policy` | The unrequested fold, asserted as correct |
| `consuming_does_not_affect_in_flight` | Two unrequested counters against each other |
| `reset_returns_every_counter_to_the_fresh_state` | The unrequested reset, on one thread |
| *(to create)* | Nothing checks the crate against this crate's own originating requirement — no test names a requirement |
