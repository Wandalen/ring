# lifecycle

The value has no lifecycle worth the name — one state, no destructor, no
release. What does have one is the sequence it serves: a consumer draining
behind a producer, which is where all the crate's steps and all its cost are.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A Barrier From `over` to the End of a Borrow](001_a_barrier_from_over_to_the_end_of_a_borrow.md) | One state, the allocation ledger, and why there is nothing to release |
| 002 | [A Consumer Draining Behind a Producer](002_a_consumer_draining_behind_a_producer.md) | The drain loop step by step, and where the barrier sits in the four-operation handshake |

### The Absences, Counted

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c '&mut self' ring_barrier/src/lib.rs                          # 0
grep -c 'impl.*Drop'  ring_barrier/src/lib.rs                        # 0
grep -c 'unsafe'      ring_barrier/src/lib.rs                        # 0
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -cE '\bwhile\b|\bloop\b|\bfor\b|Ordering::'                        # 0
grep -rc 'impl.*Drop for' ring_*/src/*.rs | grep -v ':0'               # 4, none in this chain
```

No mutation, no destructor, no `unsafe`, no loop, no ordering named. The crate's
whole life is a construction that allocates nothing and reads that allocate
every time.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR19 | `ring_barrier` | n/a — coverage | The 512-item drain loop treats a spent budget as *try again* with no bound, so a dead producer makes the test hang rather than fail |
| BR40 | `ring_barrier` | n/a — observation | The type has no `Drop`, no close, no release — the borrow is the whole state and ending is the borrow checker's business, which is what makes `Copy` sound and lets `ring_consume::Consumer` store a `Barrier` by value; `GatingSet` owns its cursors and can do none of it |
| BR41 | `ring_barrier` | n/a — coverage | The wait path has one test with a real publisher on another thread, six against cursors written by hand, and zero production callers — `ring_consume` composes `available` itself and leaves waiting to its own caller |
| BR52 | `ring_barrier` | n/a — observation | The drain loop is bounded by items received rather than attempts made, which is the only form needing no second arbitrary constant — it cannot be flaky and it can hang, and the failure mode lives entirely in the test runner's timeout |
