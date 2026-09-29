# non_functional_requirement

A wait costs memory traffic and it costs time, and this crate owns neither. The
memory traffic belongs to the caller's predicate — `ring_wait` names no ordering
and performs no atomic operation anywhere in its compiled surface. The time
belongs to `pause`, and spans roughly 1700× between its cheapest and most
expensive arm.

Both instances are measurements rather than requirements in the aspirational
sense: the numbers came from probes run against the real crate, and the
regenerate commands are here so the next reader can disagree with them.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Two Atomic Loads for Every Look](001_two_atomic_loads_for_every_look.md) | WT16 — the crate issues no loads at all, both named predicates issue two `Acquire` loads each, and a default-budget `for_data` costs up to 2048 |
| 002 | [What Each Strategy Costs Per Attempt](002_what_each_strategy_costs_per_attempt.md) | WT6 (a 1700× span across one enum argument) and WT7 (`Park` delivers 2.2–2.4× its requested sleep, and two other crates do arithmetic on the requested figure) |

### The Two Cost Axes

| | Memory (001) | Time (002) |
|--|--------------|------------|
| Owned by | the caller's predicate | `pause` |
| This crate's contribution | **zero** | all of it |
| Per look / attempt | 2 `Acquire` loads, for both named predicates | 55 ns to 119 µs, by strategy |
| At `DEFAULT_SPINS` | up to 2048 loads | 80 ns to 122 ms |
| Bounded by the crate | no — the closure is opaque | no — the budget is a count, not a deadline |
| Measured by | reading `ring_cursor` | a release-build probe, 3 runs |

Neither axis is bounded by this crate, and both are bounded by the caller's
arguments. That is the same trade twice: the crate stays generic and hands the
cost model to whoever picked the predicate and the strategy.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# this crate performs no atomic operation — expected: no output
grep -vE "^[[:space:]]*(//|///|//!)" ring_wait/src/lib.rs \
  | grep -E "Ordering|atomic|load|store|fence"

# where the loads actually are, and at what ordering
grep "pub const GATING" ring_cursor/src/lib.rs
grep "load( GATING )" ring_cursor/src/lib.rs

# every site spelling the sleep length — three reason about it, three collide
command grep -r "from_micros( 50 )\|50µs" */src/*.rs */tests/*.rs 
```

Live output:

```
pub const GATING : Ordering = Ordering::Acquire;
  cursors.iter().map( | c | c.load( GATING ) ).min()
    ring_seqno::free_slots( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
    ring_seqno::pending( self.producer.load( GATING ), self.consumer.load( GATING ) )
    ring_seqno::may_claim( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
ring_wait/src/lib.rs:      std::thread::sleep( std::time::Duration::from_micros( 50 ) );
input_timeline/tests/timeline_test.rs:    "event 1 is stamped 50µs, before its predecessor at 100µs",
ring_handle/tests/handle_test.rs:    "10 000 empty drains took {elapsed:?}; a 50µs park each would be ~500ms"
ring_poll/tests/poll_test.rs:/// The arithmetic is the assertion. `ring_wait`'s `Park` variant sleeps 50µs
demo_live_capture/tests/capture_test.rs:  std::thread::sleep( Duration::from_micros( 50 ) );
demo_live_capture/tests/capture_test.rs:  std::thread::sleep( Duration::from_micros( 50 ) );
```

| | Count |
|--|------:|
| Atomic operations in `ring_wait` | 0 |
| Memory orderings named in `ring_wait` | 0 |
| `load( GATING )` occurrences in `ring_cursor` | 7, across 4 lines |
| Of those, reachable through this crate's wrappers | 4 — `pending` and `may_claim`, two each |
| Sites reasoning about the 50 µs sleep | 3 — one value, two arithmetics |
| Lines the same pattern also matches | 3 — two unrelated sleeps, one timestamp string |
| Tests asserting the 50 µs value | **0** |

The three collisions are a pattern artefact, not a fourth reader: `input_timeline`
spells `50µs` inside an assertion message about event stamps, and
`demo_live_capture` sleeps 50 µs twice to advance a clock, in a crate whose
manifest names no `ring_*` dependency at all. The pattern is left wide on
purpose — a narrower one would stop finding the two arithmetics, which is the
half of this census that matters.

### Measured Costs

Release build, three runs, per attempt, against a predicate that never succeeds:

| Strategy | Per attempt | × `DEFAULT_SPINS` | Relative to `Spin` |
|----------|------------:|------------------:|-------------------:|
| `None` | — (one look) | 80–160 ns whole call | — |
| `Spin` | 55–77 ns | ≈ 56–79 µs | 1× |
| `Yield` | 519–526 ns | ≈ 0.53 ms | ≈ 8× |
| `Park` | 112–119 µs | ≈ 115–122 ms | ≈ 1700× |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT6 | `ring_wait` | **measured cost** | Measured: the same call with the same budget spans roughly 1700× on one enum argument — 55 ns to 119 µs per attempt — and the default is `Spin`, the arm that holds a core |
| WT7 | family | **measured cost** | `Park` delivers 112–119 µs against a requested 50 µs, and two tests in two other crates compute bounds from the requested figure without being able to see the delivered one |
| WT16 | `ring_cursor` | **measured cost** | Both named predicates issue exactly two `Acquire` loads per look, because neither question can be answered from one cursor, so a `for_data` at the default budget costs up to 2048 gated loads on two lines another thread is writing |
| WT43 | family | n/a — unenforced | No crate in the family has a `benches/` directory or a benchmark framework in its manifest, so every ns and µs figure in this corpus is a transcription from a scratch crate — the structural claims are re-run by the gate and the timing claims, including WT7's basis for saying two crates budget from the wrong number, are not |
