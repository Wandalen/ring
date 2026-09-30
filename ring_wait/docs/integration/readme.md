# integration

Two dependencies, two dependents, three call sites — and one edge that is
forbidden by a test living in a crate that does not depend on this one. Both
instances are about that graph: the first maps it, the second reads the one
dependent that had to write the loop itself.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Two Dependencies, Two Dependents, and a Roster](001_two_dependencies_two_dependents_and_a_roster.md) | Every edge, `ring_poll::PARKING_CRATES`, WT13 — the crate is what the tick path must not contain — and WT12, four retry loops of which three are copies |
| 002 | [The Wrapper That Had to Be Rewritten](002_the_wrapper_that_had_to_be_rewritten.md) | `wait_for_close` and `for_space_or_close`, and the three things `for_space` could not give the second one |

### The Graph

```sh
cd "$(git rev-parse --show-toplevel)"

# every manifest naming this crate — three files, one of them its own
grep -rl "ring_wait" */Cargo.toml

# every call into it — three lines, all `wait_until`
grep -r --include=*.rs "ring_wait::" . \
  | grep -vE ':\s*(///|//!|//)' | grep -v '^ring_wait/'
```

Live output:

```
ring_barrier/Cargo.toml
ring_shutdown/Cargo.toml
ring_wait/Cargo.toml
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
```

| | Count | |
|--|------:|--|
| Out-edges | 2 | `ring_types`, `ring_cursor` |
| Out-edges reached by production code | **1** | `ring_cursor` only serves the two unused wrappers |
| In-edges | 2 | `ring_barrier`, `ring_shutdown` |
| Call sites in those two | 3 | all `wait_until` |
| Crates forbidden from depending on this one | 3 | `ring_poll`, `ring_handle`, `ring_core` |
| Tests in other crates that name this crate | 3 | 2 in `ring_poll`, 1 in `ring_handle` |
| Open-coded copies of this crate's loop shape | 3 | `ring_poll:311`, `:380`, `:423` |

### Regenerate the Guards

```sh
cd "$(git rev-parse --show-toplevel)"

# the roster `ring_poll` asserts against the manifests on disk
grep "PARKING_CRATES" ring_poll/src/lib.rs

# `ring_handle`'s seven forbidden names, and how many are in this crate's code
code=$( sed 's|//.*||' ring_wait/src/lib.rs )
for name in 'thread::sleep' 'yield_now' '::park' 'park(' 'Condvar' 'Duration' 'Waker'; do
  printf '%s  %s\n' "$( printf '%s' "$code" | grep -cF -- "$name" )" "$name"
done

# every spin-hint retry loop in the family
grep -r "spin_loop" */src/*.rs
```

Live output:

```
//!    [`PARKING_CRATES`] and `docs/invariant/001`.
/// assert!(ring_poll::PARKING_CRATES.contains(&"ring_wait"));
/// assert!(!ring_poll::PARKING_CRATES.contains(&"ring_handle"));
pub const PARKING_CRATES: [&str; 3] = ["ring_barrier", "ring_shutdown", "ring_wait"];
1  thread::sleep
1  yield_now
0  ::park
0  park(
0  Condvar
1  Duration
0  Waker
ring_poll/src/lib.rs:                    core::hint::spin_loop();
ring_poll/src/lib.rs:            core::hint::spin_loop();
ring_poll/src/lib.rs:            core::hint::spin_loop();
ring_publish/src/lib.rs://! [`Publisher::publish`] loops on a `spin_loop` hint with no wait strategy and
ring_publish/src/lib.rs:            core::hint::spin_loop();
ring_wait/src/lib.rs:                core::hint::spin_loop();
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT12 | family | n/a — duplication | Four bounded-retry-with-a-pause-hint loops exist in the family and only one is this crate's; `ring_poll` open-codes it three times because the crate is the only granularity a manifest can enforce at build time |
| WT13 | family | n/a — observation | `ring_handle` forbids seven parking-shaped names in its own source; three of the seven are in `ring_wait`'s code and a fourth is in the comment W4 requires to exist, so the two guards are one rule enforced from opposite ends |
| WT23 | family | n/a — inconsistency | The family *does* have a per-variant tick-safety predicate (`WaitKind::is_non_blocking`, surfaced as `RingConfig::is_tick_safe`), documented as "whether this strategy can be used on the tick path" and true for `None` alone — while `ring_poll`, the crate the tick path belongs to, spins on it three times |
| WT32 | family | n/a — unadopted | `ring_claim:34` tells the caller to compose `ring_wait::for_space` then `Claimer::claim`; `ring_claim` correctly does not depend on `ring_wait`, and `for_space` has no call site in any crate, so the prescribed composition exists in one sentence and nowhere in the tree |
