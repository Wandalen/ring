# invariant

Two properties, and both are stated partly as absences. One says what the
reported frontier can never be ahead of; the other says what can never enter the
computation at all. Neither is expressible to the compiler, so each is held by a
test — and in one case the test that used to hold it no longer can.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Frontier Never Exceeds a Dependency](001_the_frontier_never_exceeds_a_dependency.md) | The safety property, the non-atomic read that weakens its statement, and the direction staleness runs |
| 002 | [Capacity Never Enters the Arithmetic](002_capacity_never_enters_the_arithmetic.md) | The defining absence, the 4-versus-1,000 assertion that makes it falsifiable, and the grep that no longer can |

### The Two, and What Holds Each

| | 001 | 002 |
|--|-----|-----|
| Property | `frontier <= every dependency` | `Capacity` appears nowhere |
| Enforced by | `Acquire`, `.min()`, `Option`, no cache — all in other crates | Nothing structural; the type simply has no capacity |
| Held by | `a_barrier_never_reports_a_frontier_a_dependency_has_not_reached` | `available_ignores_capacity_entirely` |
| Instrument's limit | Cannot falsify the ordering on x86; needs the spawn to discriminate at all | B1's grep passes because capacity has no way in, not because it was kept out |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs | grep -cE "capacity|Capacity"   # 0
grep -c 'thread::scope' ring_barrier/tests/barrier_test.rs                          # 2
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR15 | `ring_barrier` | n/a — observation | `a_barrier_never_reports_a_frontier_a_dependency_has_not_reached` asserts equality with `Seq::ZERO` 20,000 times and **can** fail — unlike `ring_gating`'s counterpart (G12), which is a tautology |
| BR35 | family | n/a — unenforced | `Seq::distance_to` justifies its saturation by describing two kinds of caller, and every one of the family's eleven call sites is the second kind — not one compares the two sequences first, so the justification rests entirely on the first category being empty |
| BR36 | `ring_barrier` | n/a — observation | A consumer past its frontier and one exactly at it both get `available == 0`; the doctest asserts the two side by side and labels the second a property rather than a state that must never occur, and the saturation erases the evidence before `available` can return |
| BR37 | `ring_barrier` | n/a — coverage | The crate whose invariant is about two threads racing on shared cursors is compiled out of every loom run, contributing twenty-two ordinary tests and no interleaving model; its one real-race test asserts progress rather than ordering |
