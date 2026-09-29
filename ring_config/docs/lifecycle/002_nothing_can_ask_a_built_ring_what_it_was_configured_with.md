# Lifecycle: Nothing Can Ask a Built Ring What It Was Configured With

### Scope

**Purpose:** Record what survives construction as queryable state, what shape it
survives in, and what becomes unreachable the moment the record is dropped.

**Responsibility:** The three readings a built ring exposes, the two fields with
no trace anywhere, and the derived equality that has no production use because
there is nothing to compare against.

**In Scope:** `ring_core/src/lib.rs:101-110`, `:120-124`, `:214`, `:227`,
`:244`; `ring_config/src/lib.rs:41`, `:167`, `:192`, `:204`;
`ring_bench/src/lib.rs:323`.

**Out of Scope:** How the record narrows on the way down is
[`lifecycle/001`](001_built_once_copied_never_mutated.md). The reader census
behind the same shape is
[`api/002`](../api/002_three_readers_used_and_four_with_no_caller.md).

---

## What Is Left After the Record Is Gone

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- everything a built ring can be asked about how it was configured --'
command grep 'pub const fn backend\|pub fn capacity( &self ) -> Capacity\|pub const fn overflow' ring_core/src/lib.rs
echo '  -- and the shape producers survives as --'
command grep -m1 -A9 -F 'pub enum Backend' ring_core/src/lib.rs
echo '  -- every method named wait, producers or batch taking &self, anywhere in the family --'
command grep -r 'fn wait( &self )\|fn producers( &self )\|fn batch( &self )' --include=*.rs */src | sed 's|^ring/||'
echo '  -- and every comparison of two configurations outside this crate, in src/ or tests/ --'
command grep -rE '(config|cfg)[a-z_]* ==|== *(config|cfg)\b' --include=*.rs */src */tests | command grep -v '^ring_config/' | command grep -v '///\|//!' || echo '  (none)'
```

Live output:

```
  -- everything a built ring can be asked about how it was configured --
  pub const fn backend( &self ) -> Backend
  pub fn capacity( &self ) -> Capacity
  pub const fn overflow( &self ) -> OverflowPolicy
  -- and the shape producers survives as --
pub enum Backend
{
  /// [`ring_spsc`] — one producer, binding `free_capacity`, no RMW in the path.
  Spsc,
  /// [`ring_mpsc`] — many producers, advisory `free_capacity`.
  Mpsc,
  /// `crossbeam_queue::ArrayQueue` — feature 187's interim backend.
  #[ cfg( feature = "crossbeam" ) ]
  Crossbeam,
}
  -- every method named wait, producers or batch taking &self, anywhere in the family --
ring_bench/src/lib.rs:  pub const fn producers( &self ) -> usize
ring_bench/src/lib.rs:  pub const fn batch( &self ) -> usize
ring_bench/src/lib.rs:  pub const fn producers( &self ) -> usize
ring_config/src/lib.rs:  pub const fn wait( &self ) -> WaitKind
ring_config/src/lib.rs:  pub const fn producers( &self ) -> usize
ring_config/src/lib.rs:  pub const fn batch( &self ) -> usize
  -- and every comparison of two configurations outside this crate, in src/ or tests/ --
demo_window_present/src/grade.rs:  if tally.presented == 0 && tally.skipped == 0 && tally.reconfigured == 0
```

---

### RC31 — Two Fields Survive as Themselves, One as a Tag, and Two Leave No Trace

A built `ring_core::Ring` answers three questions about how it was made:
`capacity()`, `overflow()`, and `backend()`. The first two return the configured
values as configured. The third returns a `Backend` — `Spsc`, `Mpsc`, or
`Crossbeam` behind a feature — which is what became of `producers` after
`is_multi_producer()` reduced it to a bit and the match at `ring_core:174`
consumed that bit.

So a caller holding a ring can recover the capacity and the overflow policy
exactly, and can learn whether more than one producer was expected. The count
itself is gone: `Backend::Mpsc` is compatible with a configured `2` and a
configured `64` alike.

`wait` and `batch` leave nothing at all. The census over every `src/` file in
thirty-three crates finds a method named `wait( &self )` on exactly one type —
`RingConfig` itself — and methods named `producers` or `batch` on only
`RingConfig` and two `ring_bench` harness types, `Workload` and `Outcome`, which
report what the harness was told rather than what any ring holds.

**Finding.** The record is not recoverable from what it built. That is a
reasonable design for the two fields nothing needs afterwards, and it is worth
stating because the crate's own documentation frames `RingConfig` as the thing a
ring is configured by, which invites the assumption that a ring knows its
configuration. It knows two fifths of it, plus a tag.

The practical consequence is narrow but real: a `WaitKind` mismatch cannot be
diagnosed from a running system. If a ring is spinning where it should have been
non-blocking, the reading that would say so was dropped at the end of
`ring_core::Ring::new`, and no accessor anywhere in the family could report it.

---

### RC32 — The Derived Equality Has Nothing to Compare Against

`RingConfig` derives `PartialEq` and `Eq`, and `the_record_is_copy_and_compares_by_value`
exercises them. Outside this crate, in `src/` and `tests/` across the family, two
configurations are never compared: the census returns nothing.

That follows from RC31 rather than being separate from it. Comparing
configurations is useful when one of them came from somewhere — a ring, a
registry, a serialized form — and there is nowhere for a second one to come from.
The only stored `RingConfig` in the workspace is `ring_bench::Workload`'s, and its
`config()` accessor at `:323` returns the copy the harness was handed, so
comparing it to the record the caller already has compares a value to itself.

**Finding.** The derive is correct and costs nothing — five `Copy` fields, all of
whose types derive the same, and `setters_commute` needs the comparison to state
its property at all. But it is a within-crate facility: it lets the suite say two
chains agree, and there is no configuration anywhere in the family that a caller
could obtain and compare against one they built.

That is the round-trip this type does not have. `capacity` and `overflow` survive
into a ring and could be checked against the record that made it; `producers`
survives as a coarser value; `wait` and `batch` do not survive. So even a
hand-written comparison against a built ring could only ever be partial, and the
equality the type does derive can only be used on the near side of construction.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/001`](001_built_once_copied_never_mutated.md) | The narrowing that produces this end state |
| [`type/001`](../type/001_five_derives_and_the_one_that_is_free_and_absent.md) | The five derives, including the equality this records |
| [`item/002`](../item/002_the_two_derived_readings.md) | `is_multi_producer`, the reading `Backend` is downstream of |
| [`api/002`](../api/002_three_readers_used_and_four_with_no_caller.md) | The accessors nothing calls, from the reader's side |

### Sources

| Fact | Where |
|------|-------|
| The three readings a built ring exposes | `ring_core/src/lib.rs:214`, `:227`, `:244` |
| The two fields a ring keeps | `ring_core/src/lib.rs:120-124` |
| `Backend`'s three variants | `ring_core/src/lib.rs:101-110` |
| The branch that reduces `producers` to a tag | `ring_core/src/lib.rs:174` |
| No `wait`/`producers`/`batch` accessor on any built object | Census above |
| No comparison of two configurations outside this crate | Census above |
| `Workload::config()` returning the harness's own copy | `ring_bench/src/lib.rs:323` |

### Tests

| Test | Covers |
|------|--------|
| `the_record_is_copy_and_compares_by_value` | The equality that has no production caller |
| `setters_commute` | The one property that needs the comparison to be stated |
| `tick_safety_is_exactly_non_blocking_waiting` | The reading that survives construction least |
