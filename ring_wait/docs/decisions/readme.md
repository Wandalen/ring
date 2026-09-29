# decisions

Two choices with live alternatives, recorded with the argument for each and the
condition under which each could be revisited. One is about a variant's
behaviour; the other is about where the variant lives at all.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [`Park` Sleeps Rather Than Parking](001_park_sleeps_rather_than_parking.md) | The deviation, four rejected alternatives, WT20 — the family's only manual check that requires a comment to exist — and the deletion condition |
| 002 | [The Discriminants Live in `ring_types`](002_the_discriminants_live_in_ring_types.md) | A prior ruling, WT19 — the boundary costs one byte — and WT22, the sibling enum that already has two matchers |

### The Two Decisions at a Glance

| | Decision 001 | Decision 002 |
|--|--------------|--------------|
| Question | how should `Park` block? | where do the four variants live? |
| Chosen | a 50 µs sleep | in `ring_types`, handlers here |
| Ruled by | this crate, in a code comment | a prior ruling |
| Guarded by | W4, which checks the comment survives | `tests/wait_test.rs:69-84`, checking the two crates agree |
| Reversible when | a wakeup relationship exists in a crate this one may depend on | a strategy needs state |
| Cost today | 50 µs floor per attempt, a name that lies | this crate cannot rename or extend the enum |

### Regenerate the Evidence

```sh
cd "$(git rev-parse --show-toplevel)"

# W4 — the three pause primitives, and no `thread::park` among them
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E "thread::(park|sleep|yield_now)|spin_loop"
# ...but `thread::park` must appear in the file, in the comment
grep -c "thread::park" ring_wait/src/lib.rs

# WT22 — how many crates exhaustively match each ruled enum
grep -rl 'WaitKind::Spin\s*=>' */src/*.rs
grep -rl 'OverflowPolicy::[A-Za-z]*\s*=>' */src/*.rs
```

Live output:

```
        core::hint::spin_loop();
      std::thread::yield_now();
      std::thread::sleep( std::time::Duration::from_micros( 50 ) );
1
ring_wait/src/lib.rs
ring_overflow/src/lib.rs
ring_stats/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT19 | `ring_types` | n/a — observation | `WaitKind` is one byte, `Copy`, and fieldless, which is what lets `RingConfig::with_wait` be a `const fn` and makes the discriminants/handlers split free at every call site |
| WT20 | `ring_wait` | n/a — observation | W4's second command requires `thread::park` to *appear* in the file, in a comment; it is the family's only manual check that asserts an explanation still exists rather than that code does |
| WT30 | `ring_types` | **misleading doc** | `ring_types::WaitKind::Park` used to document the variant as "Block until a publisher signals. Costs nothing while idle, pays a wakeup" — the only implementation sleeps 50 µs on a timer, no signalling primitive exists anywhere in the family, and this correction sits in a crate two-thirds of `WaitKind`'s users never depend on, so the wording was fixed at its own source and now reads "Idle between reads; no publisher wakes it. Cheapest, highest latency." |
| WT31 | family | n/a — inconsistency | The family's two bounded-retry crates ship defaults a factor of 1024 apart — `DEFAULT_SPINS` at 1024, `ring_poll::Budget::once` at 1 — encoding the tick-path distinction in two numbers that carry no reference to each other in either direction |
