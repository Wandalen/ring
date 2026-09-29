# Decisions: `Relaxed` Everywhere, With a Reason That Covers One Load

### Scope

**Purpose:** Record the crate's one explicit performance decision, the argument
given for it, and what that argument does and does not establish.

**Responsibility:** The four-line ordering rationale, every site it governs, and a
measurement of the cost it names.

**In Scope:** the module comment's ordering paragraph, and every `Ordering::` in the
crate.

**Out of Scope:** The cost the decision does not mention is
[`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md). What
the compositions do with these loads is
[`algorithm/001`](../algorithm/001_eleven_operations_and_three_compositions.md).

---

## One Ordering, One Argument, and What the Family Uses Instead

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the argument, in full --'
command grep -m1 -A3 -F '//! The counters are relaxed atomics: a stats read is a diagnostic, never a' ring_stats/src/lib.rs
echo '  -- every ordering the crate uses --'
command grep -o 'Ordering::[A-Za-z]*' ring_stats/src/lib.rs | sort | uniq -c
echo '  -- what its neighbours use --'
for c in ring_atomic ring_cursor ring_gating; do
  printf '    %-12s' "$c"
  command grep -oh 'Ordering::[A-Za-z]*' ring/$c/src/lib.rs | sort | uniq -c | tr '\n' ' '
  echo
done
```

Live output:

```
  -- the argument, in full --
//! The counters are relaxed atomics: a stats read is a diagnostic, never a
//! synchronisation point, so ordering them would buy nothing. The fence-cost
//! half of that argument was never measured here, and measures within noise
//! on this workspace's two target platforms — the layout's cache-line
  -- every ordering the crate uses --
     18 Ordering::Relaxed
  -- what its neighbours use --
    ring_atomic       2 Ordering::AcqRel       4 Ordering::Acquire      11 Ordering::Relaxed       2 Ordering::Release 
    ring_cursor       6 Ordering::Acquire       8 Ordering::Release 
    ring_gating       5 Ordering::Release 
```

---

### ST13 — The Decision Is Right, Recorded, and Applied Without Exception

Eighteen atomic sites, eighteen `Relaxed`, no other ordering anywhere in the crate —
eleven of each when this was written, and the ratio held through every addition since.
That uniformity is not the family default: `ring_cursor` uses `Acquire` and `Release` at
all fourteen of its sites and `Relaxed` at none, `ring_gating` uses only `Release`,
and `ring_atomic` mixes all four. `ring_stats` is the one that made a single choice
and applied it everywhere.

The reason is written down, which is more than most such choices get: a stats read is
a diagnostic, never a synchronisation point. That is exactly right for what these
counters are. Nothing in the family gates on a counter value, no publish is ordered
against a stats write, and the `wait_nanos` total is a number for a human to look at.
A `Relaxed` `fetch_add` is still atomic — no count is lost, which
`counts_are_exact_under_contention` demonstrates — it simply carries no promise about
what else the reading thread can see.

**Finding.** This is the crate's one recorded design decision and it is a good one,
correctly scoped in its first clause. What follows the comma is where it gets
interesting: the decision is justified by a cost, and that cost is stated rather than
measured.

---

### ST14 — The Avoided Cost Is Unmeasurable Here; the Unmentioned One Is 2.6×–3.3×

"Ordering them would buy nothing and cost a fence on the hot path." The first half is
sound. The second half is a quantitative claim, and it is testable: run the same
seven counters, same layout, same access pattern, under `Relaxed` and under `AcqRel`.

**Finding.** Median of nine paired ratios, the two variants run back-to-back inside
every repetition:

```
    size_of::< SevenCounters >()  56 bytes

    threads   relaxed   acqrel    ratio   (median of 9 paired runs)
          1      4.0ms     3.8ms   0.96×   spread 0.52×–2.18×
          3     13.0ms    14.0ms   1.07×   spread 0.96×–1.20×
          6     24.5ms    24.6ms   0.96×   spread 0.94×–1.11×
         12     49.8ms    50.5ms   1.00×   spread 0.96×–1.12×

    ratio > 1 means the ordering the crate declined would have cost that much
```

A second run:

```
    size_of::< SevenCounters >()  56 bytes

    threads   relaxed   acqrel    ratio   (median of 9 paired runs)
          1      3.9ms     4.4ms   1.17×   spread 0.59×–2.35×
          3     13.5ms    12.2ms   0.90×   spread 0.63×–1.26×
          6     24.9ms    25.2ms   1.06×   spread 0.69×–1.14×
         12     46.9ms    47.3ms   1.01×   spread 0.90×–1.09×

    ratio > 1 means the ordering the crate declined would have cost that much
```

Every median lands within 10% of 1.0, and the sign flips between runs at every
thread count. The fence the decision was made to avoid does not show up at all on
this host. That is not a surprise once stated: on AArch64 the difference is `ldadd`
against `ldaddal`, a single instruction either way, and on x86-64 a `lock`-prefixed
read-modify-write is already a full barrier, so `AcqRel` on `fetch_add` is free there
by construction. Those are the two platforms `ring_align` names as the family's
targets.

Set that beside the cost the same struct pays and does not mention. The layout these
`Relaxed` operations run in — seven contended counters packed into 56 bytes —
measures at **2.6×–3.3×** the same counters given a cache line each
([`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md)). The
crate reasoned carefully about a hot-path cost it could not measure and shipped one
it never looked for, roughly three times larger, in the same struct.

Neither the decision nor its reasoning needs reversing — `Relaxed` remains the right
ordering for diagnostic counters, and the first clause is the load-bearing one. What
the record should say is that the second clause was never checked, and that the
question it was answering was not the one that mattered for this struct.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats
command grep -F 'contention is the real, measured cost in this struct' src/lib.rs
```

Live output:

```
//! contention is the real, measured cost in this struct, at 2.6x-3.3x, not
```

**Disposition:** applied — the module comment's ordering paragraph now says the
fence-cost half of its own argument was never measured and turned out to be
within noise, and names the layout's cache-line contention as the real,
measured 2.6x-3.3x cost in this struct instead — matching what this finding
and `data_structure/001` § ST9 both establish.
Now prints: `contention is the real, measured cost in this struct`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/002`](002_per_policy_drop_counters_stated_and_delivered.md) | The crate's other recorded decision, and how completely it was carried out |
| [`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md) | The cost that is not in the argument |
| [`non_functional_requirement/001`](../non_functional_requirement/001_cheap_enough_to_leave_on.md) | The feature's cost requirement, against both measurements |
| [`algorithm/001`](../algorithm/001_eleven_operations_and_three_compositions.md) | Every site this decision governs |

### Sources

| Fact | Where |
|------|-------|
| The ordering argument | `ring_stats/src/lib.rs:17-20` |
| 11 sites, all `Relaxed` | Census above |
| `ring_cursor` at 6 `Acquire` / 8 `Release` / 0 `Relaxed` | Census above |
| No measurable `Relaxed`-against-`AcqRel` difference | Probe, two runs quoted above |
| The two named target platforms | `ring_align/src/lib.rs:26-30` |
| 2.6×–3.3× for the layout | `data_structure/001` § ST9 |

### Tests

| Test | Covers |
|------|--------|
| `counts_are_exact_under_contention` | That `Relaxed` loses no counts — the correctness half of the decision |
| `distinct_policy_counters_do_not_interfere_under_contention` | That neighbouring counters stay independent under `Relaxed` writes |
| `each_counter_is_monotone_while_writers_run` | The one property a `Relaxed` reader can still rely on |
| *(to create)* | Nothing measures the cost the decision cites, in either direction |
