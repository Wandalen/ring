# Integration: The Read Path, and the Edge That Was Assigned and Removed

### Scope

**Purpose:** Record who reads these counters, in what condition they read them, and
the two edges the workspace deliberately does not have.

**Responsibility:** `ring_bench`'s embedded `RingStats` and its accessor,
`ring_factory`'s recorded removal, and this crate's single dependency.

**In Scope:** `ring_bench/src/lib.rs:620-630`, `:735-741`, `:923-961`;
`ring_factory/Cargo.toml:27-31`; `ring_stats/Cargo.toml:9`;
`ring_stats/src/lib.rs:49`.

**Out of Scope:** Who writes, and the two named callers that do not exist, is
[`integration/001`](001_the_write_path_and_two_callers_that_are_not_there.md). The
absence of a way to read several counters at once is
[`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md).

---

## The One Real Consumer, and the Edge Somebody Deleted on Purpose

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the only crate that holds a RingStats and hands it out --'
awk '/^  write_nanos : u128,$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 1 { print } /^    self\.write_nanos$/{ n2 = NR } n2 && NR >= n2 + 1 && NR <= n2 + 9 { print }' ring_bench/src/lib.rs
echo '  -- and how it fills one --'
command grep -m1 -A10 -F '  // Every counter is the *drained* count, never the reported one. A record that' ring_bench/src/lib.rs
echo '  -- the edge that was assigned and removed --'
command grep -m1 -A4 -F '# Message 960 also assigned `ring_stats` and `ring_tls`; both are removed.' ring_factory/Cargo.toml
echo '  -- what ring_stats itself depends on, and what it uses it for --'
command grep '^ring_types' ring_stats/Cargo.toml
command grep '^use ring_types' ring_stats/src/lib.rs
```

Live output:

```
  -- the only crate that holds a RingStats and hands it out --
  write_nanos : u128,
  stats : RingStats,
  }

  /// Feature 185's counters for this run, written once from totals after the
  /// clock stopped.
  #[ must_use ]
  pub const fn stats( &self ) -> &RingStats
  {
    &self.stats
  }
  -- and how it fills one --
  // Every counter is the *drained* count, never the reported one. A record that
  // came back out claimed exactly one slot and published it; a record that did
  // not claimed none — a `DropNewest` discard never reaches a slot at all. That
  // mapping is what keeps `in_flight` at zero here, so a nonzero reading would
  // mean what `ring_stats` says it means (a slot taken and abandoned) rather
  // than "the workload offered more than the ring could hold", which is not a
  // leak and is already reported as `dropped`.
  //
  // Fix(BN11): that zero is *structural*, not measured. `in_flight` is
  // `claimed - published` and both are this same expression, so no run of this
  // harness can make it nonzero — the two assertions on it in the suite pin the
  -- the edge that was assigned and removed --
# Message 960 also assigned `ring_stats` and `ring_tls`; both are removed.
# `RingConfig` has no field that would configure staging or counters, so there
# was nothing for a build to do with either, and inventing surface to justify a
# manifest line is backwards. Closes `docs/decisions` Pending 7 by the second of
# its two branches. Restore them alongside the config fields, not before.
  -- what ring_stats itself depends on, and what it uses it for --
ring_types = { path = "../ring_types" }
use ring_types::OverflowPolicy;
```

---

### ST19 — The Family's Only Live Use Is a Summary Written After the Clock Stopped

`ring_bench::Outcome` embeds a `RingStats` by value and exposes it through a
`const fn` returning `&RingStats`. Its own doc says what it is: "Feature 185's
counters for this run, written once from totals after the clock stopped."

That is exactly what the four `record_*` calls do. `received`, a single number for
the whole run, is fed to `record_claim`, `record_publish` and `record_consume`; the
difference between offered and received is fed to `record_drop` under the run's one
configured policy. Nine lines of comment above them explain the mapping with care,
because getting it wrong is what produced the 240-leaked-slots-on-a-16-slot-ring
reading the family already fixed
([`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) § ST4).

**Finding.** Every property that makes these counters hazardous is switched off in
their only real use. Nothing is concurrent — the writes happen on one thread after
every worker has stopped. Nothing tears — each counter is written once. `in_flight`
cannot under-report, because `claimed` and `published` are the same number and
neither is moving. `dropped_total` cannot conflate causes, because a run has one
policy.

That is why the mapping comment is nine lines and the concurrency is nowhere
discussed: in this consumer there is none. The gap is that this is the only consumer,
so every reader of the family who wants to know how `RingStats` behaves in practice
finds a worked example in which none of its actual behaviour is exercised. The
accessor even propagates the shape one level out — a `&RingStats` handed to a caller
who still has no way to read two counters together
([`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md)), which is
harmless here for the same reason and would not be anywhere else.

---

### ST20 — Two Edges Refused, One With Its Reasoning Written Into the Manifest

`ring_factory` was assigned `ring_stats` as a dependency and does not have one. The
manifest says why, in five lines of comment: `RingConfig` has no field that would
configure counters, "so there was nothing for a build to do with either, and
inventing surface to justify a manifest line is backwards" — closing a numbered
pending decision by its second branch, and stating the condition for reversal
("Restore them alongside the config fields, not before").

The other refused edge points the other way. `ring_stats` declares exactly one
dependency, `ring_types`, and imports exactly one item from it: `OverflowPolicy`. It
does not declare `ring_align`, whose `CacheAligned` would address the 2.6×–3.3× its
layout costs
([`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md)).

**Finding.** These two absences are the same shape and have opposite documentation.
One is a considered refusal with its reasoning, its resolved decision reference, and
its reversal condition recorded in the file where a future reader would look for
them. The other is a dependency that was, as far as any record shows, never
considered — no comment, no decision entry, nothing in `ring_stats`' own manifest or
module doc.

The `ring_factory` comment is worth naming as the standard the rest of the family's
manifests are not held to. It is also the reason `ring_atomic`'s two undeclared-but-
unused dependencies read as omissions rather than choices: when a manifest in this
workspace has been thought about, it says so.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/001`](001_the_write_path_and_two_callers_that_are_not_there.md) | Who writes, and the callers the docs name that do not exist |
| [`api/002`](../api/002_seven_readers_and_no_way_to_read_the_set.md) | The shape `Outcome::stats()` hands on to its own callers |
| [`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md) | The dependency that would address the measured cost |
| [`pattern/001`](../pattern/001_the_record_and_read_pair.md) | Recording and reading as a shape, and where the family puts each half |

### Sources

| Fact | Where |
|------|-------|
| `Outcome` embeds a `RingStats` | `ring_bench/src/lib.rs:628` |
| "written once from totals after the clock stopped" | `ring_bench/src/lib.rs:735-736` |
| The four totals-based recorder calls | `ring_bench/src/lib.rs:944-946,961` |
| The mapping comment and what it prevents | `ring_bench/src/lib.rs:923-929` |
| `ring_factory`'s recorded removal | `ring_factory/Cargo.toml:27-31` |
| One dependency, one imported item | `ring_stats/Cargo.toml:9`; `ring_stats/src/lib.rs:49` |

### Tests

| Test | Covers |
|------|--------|
| `ring_bench`'s `the_counters_are_the_runs_own_totals` | `ring_bench`'s mapping, at rest, after the fix |
| `a_drop_is_counted_not_absorbed` | `ring_overflow`'s single-drop recording |
| `counts_are_exact_under_contention` | The only concurrent use anywhere, and it is a test |
| *(to create)* | No consumer exercises these counters while a ring is running |
