# decisions

`ring_stats` records two design decisions in its own source, and both are stated in
the module comment rather than left implicit: every counter is a relaxed atomic, and
drops are counted per policy instead of in one bucket. For a crate this size that is
an unusually good ratio of recorded rationale to code.

The two instances here take one decision each, and they fail in opposite ways. The
ordering decision is correct, applied without a single exception, and justified by a
cost that turns out not to exist on either of the family's target platforms — while
the same struct pays a cost three times larger that the argument never considers.
The per-policy decision is correct, implemented past the letter of the requirement,
and then partly undone by a method the requirement never asked for.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_relaxed_with_a_reason_that_covers_one_load.md) | `Relaxed` Everywhere, With a Reason That Covers One Load | The ordering argument, its uniform application, and a measurement of the cost it names |
| [002](002_per_policy_drop_counters_stated_and_delivered.md) | Per-Policy Drops Delivered, and Five Methods Nobody Asked For | What this crate was originally asked to count, what the crate ships, and where the findings landed |

## A Cost Reasoned About and a Cost Not Looked For

Eighteen atomic sites, eighteen `Relaxed`, no exceptions — against `ring_cursor`'s
fourteen sites at `Acquire`/`Release` and no `Relaxed` at all. The reason is written
down and the first half of it is exactly right: a stats read is a diagnostic, never a
synchronisation point.

The second half — that ordering would "cost a fence on the hot path" — is a
quantitative claim, and measured against the same counters under `AcqRel` it does not
appear: every median within 10% of 1.0, the sign flipping between runs. Meanwhile the
layout those operations run in costs 2.6×–3.3×. The crate reasoned carefully about the
smaller of the two and never looked at the larger.

## A Requirement Delivered and a Surface That Partly Undoes It

This crate's own originating requirement asks for four counters, and one clause of it — "and under which policy" —
is the only part that constrains storage. The crate implements it fully: three fields,
an exhaustive `match` in both directions, and the rationale restated in the
implementer's own words.

Then five public methods appear whose subject the requirement never names, and the
findings in this corpus that name a single method name three of those five. The four
requested counters have produced no method-specific finding at all. What separates
them is not care but specification: the requested ones arrived with something to be
checked against.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two recorded decisions --'
command grep -m1 -A8 -F '//! The counters are relaxed atomics: a stats read is a diagnostic, never a' ring_stats/src/lib.rs
echo '  -- the first, applied without exception --'
command grep -o 'Ordering::[A-Za-z]*' ring_stats/src/lib.rs | sort | uniq -c
echo '  -- the second, and what the feature asked for --'
command grep -m1 -F 'Per-ring counters for what actually happened: items claimed, items published, items dropped and under which policy, and nanoseconds spent waiting. They are cheap enough to leave on and are the only thing that distinguishes a ring under mild pressure from one that is quietly discarding traffic.' docs/feature/185_ring_stats.md
echo '  -- and the methods it never named --'
command grep -o '  pub fn \(record_consume\|consumed\|dropped_total\|in_flight\|reset\)' ring_stats/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| ST13 | `ring_stats` | n/a — observation | Eighteen atomic sites, all `Relaxed`, no exceptions — eleven of them when this was written, and every site added since took the same ordering — against `ring_cursor` at fourteen `Acquire`/`Release` sites and no `Relaxed`, and `ring_atomic` mixing all four, making this the one crate in the group that made a single ordering choice, applied it everywhere, and wrote down why |
| ST14 | `ring_stats` | **measured cost** | The fence the ordering decision was made to avoid does not measure: `Relaxed` against `AcqRel` on the same counters gives medians within 10% of 1.0 at 1, 3, 6 and 12 threads with the sign flipping between runs — on AArch64 the difference is `ldadd` against `ldaddal`, and on x86-64 a `lock`-prefixed RMW is already a full barrier — while the layout those same operations run in costs 2.6×–3.3× |
| ST15 | `ring_stats` | n/a — observation | "And under which policy" is the only clause in this crate's own originating requirement that constrains storage rather than naming a number, was not present in the design record it descends from, and is implemented past the letter — three fields, exhaustive `match` in both directions, rationale restated in the implementer's own words |
| ST16 | `ring_stats` | n/a — coverage | Seven fields for four requested counters and five public methods on subjects this crate's own originating requirement never names — and every method-specific finding in this corpus lands on three of those five (`in_flight`, `reset`, `dropped_total`), while the four requested counters have produced none, each being one atomic operation that does what its name says |
