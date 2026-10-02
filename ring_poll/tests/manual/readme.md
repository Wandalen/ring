# `ring_poll` manual test plan

Readings and measurements the automated suite cannot make. Four stages, each
with a command, a prediction written **before** running it, and the observed
result.

The prediction-first order is the point. A stage written after seeing the output
records what happened; a stage written before it can be *wrong*, and a wrong
prediction is the only thing here that teaches anything. This round produced one,
in P1.

## Why these four are manual

| Stage | Why a test cannot do it |
|---|---|
| P1 | The property is that a name **does not resolve**. A test cannot name a crate it cannot see, so the only way to observe the error is to add the import and fail to build |
| P2 | A transitive dependency-graph fact. `cargo tree` is the instrument; no test links against the metadata |
| P3 | The suite asserts a bound. What the bound is *worth*, meaning the ratio between the measured cost and the parking cost it must discriminate from, is a reading, not an assertion |
| P4 | A check that an inherited lesson was applied. Nothing fails if it was not; the cost shows up in a coverage number three steps later |

---

## P1: reaching for a parking operation is a resolution failure

[`invariant/001`](../../docs/invariant/001_no_parking_operation_on_the_tick_path.md)
claims the parking operations are not reachable from this crate. The claim is
worth exactly the compiler error behind it, so this stage produces the error.

**Command.** Write `ring_poll/tests/p1_probe.rs` containing the two
imports below, run `cargo build --tests -p ring_poll`, then delete it.

```rust
use ring_wait::pause;
use ring_types::WaitKind;

#[ test ]
fn probe() { let _ = pause( WaitKind::Park, 0 ); }
```

**Prediction.** The build fails with `E0432: unresolved import` on `ring_wait`.
That is an *unresolved name*, not a type error, because the crate is not in the
dependency graph at all.

**Result (2026-08-28): the prediction holds, and the stage produced a wrong
sub-prediction worth more than the right one.**

```
error[E0432]: unresolved import `ring_wait`
 --> ring_poll/tests/p1_probe.rs:1:5
error: could not compile `ring_poll` (test "p1_probe") due to 1 previous error
```

**One error, not two.** The probe imports two crates this crate's *runtime*
manifest does not name, and the natural expectation is two unresolved imports.
`ring_types` resolved fine, because it is a **dev-dependency**. It is present for
the test suite and absent from the public API.

That distinction is the finding. `PARKING_CRATES` and the manifest scan in
`tests/poll_test.rs` both read the whole manifest, dev-dependencies included, so
a `ring_wait` *dev*-dependency would fail the suite even though nothing on the
tick path could reach it. That is the conservative direction to be wrong in.
[`integration/001`](../../docs/integration/001_family_dependency_seam.md)'s edge
table now states it and distinguishes the runtime edge from the two dev edges,
so the behaviour is no longer an accident of how the check was written.

**Recorded as a near miss.** Had the prediction included the count, it would have
been wrong, and the natural repair would have been to loosen the manifest scan to
`[dependencies]` only. That would weaken a correct check to satisfy a wrong
reading of it. This is the family's defect shape 5, *a check that is right for
the wrong reason*, arriving from the other side. Here the check is right for a
reason its author had not noticed.

---

## P2: nothing reaches `ring_wait`, at any depth

[`pattern/001`](../../docs/pattern/001_enforcement_by_dependency_graph.md) names
its own second failure mode. The manifest scan is a **proxy** that tests a name
in one file, while the claim is about *reachability* through the whole graph. A
crate that got parking transitively, say through `ring_core`, would pass the
proxy and violate the claim.

**Command.**

```sh
cargo tree -p ring_poll | grep -c ring_wait
```

**Prediction.** Zero. `ring_core`'s own closure is most of tier 1 and tier 2 and
none of it waits.

**Result (2026-08-28): `0`.** The full closure is 17 family crates and 0 external
ones, and `ring_wait` is in neither set.

The two checks agree today, which is what makes the proxy safe to keep as the
fast form (it runs in the suite; this does not). This stage is recorded because
agreement now is not agreement later
(→ `pln_staged.rulebook.md § Measurement : Target Over Proxy`), and
because a proxy whose target has never been run is the shape that passes whether
or not the claim holds.

---

## P3: what the 500 ms bound is worth

`a_large_budget_spins_rather_than_sleeping` bounds 20 000 attempts at 500 ms,
and [`invariant/002`](../../docs/invariant/002_a_budget_bounds_attempts_not_time.md)
justifies the number by arithmetic: a `Park` pause sleeps 50 µs per attempt, so
a parking implementation would cost about 1.0 s. The arithmetic is only worth
the measured side of it.

**Command.**

```sh
cargo nextest run -p ring_poll a_large_budget_spins
```

**Prediction.** Well under 10 ms. Spinning costs nanoseconds per attempt, so
20 000 of them plus a `try_push` each should not reach a hundredth of a second.

**Result (2026-08-28): 0.008 s, including test setup.**

| | Cost of 20 000 attempts | Ratio to the 500 ms bound |
|---|---|---|
| Measured (spin) | **≤ 8 ms** | 62× under |
| Arithmetic (`Park`, 50 µs each) | ~1 000 ms | 2× over |

So the bound sits between the two with roughly two orders of magnitude of margin
on the side that matters. The assertion is not a tight-rope. A slow machine
would have to be **60× slower** before it flaked, while a parking implementation
fails it on any machine.

**What this does not establish.** That 20 000 attempts is a sensible budget. It
is not; it is a diagnostic value chosen to make the two costs separable.
[`pitfall/001`](../../docs/pitfall/001_non_parking_is_not_bounded_latency.md)
is about exactly the caller who would use it in earnest.

---

## P4: the `loop`-coverage lesson was applied, not rediscovered

`ring_shutdown` measured that `llvm-cov` never attributes a hit to a bare `loop`
keyword, costing one uncovered line. That finding is only worth anything if the
next crate written does not pay for it again.

**Command.**

```sh
grep -cE '^\s*loop\s*$' ring_poll/src/lib.rs
cargo tarpaulin -p ring_poll --all-features --skip-clean --out Stdout \
  | grep -oE '^\|\| ring_poll/src/[^:]+: [0-9/]+'
```

**Prediction.** Zero bare `loop` lines, and 100% on the first coverage run. There
is no before-and-after, because there is no "before".

**Result (2026-08-28): `0` bare loops, and `85/85` on the first run.**

This is the cheapest stage here and the one most worth keeping. A finding
recorded in a sibling crate's docs is only a finding if it changes what the next
crate does; a grep that costs nothing is what tells the difference between a
lesson learned and a lesson written down.

**Caveat.** This stage cannot fail *usefully*. If the grep returned
non-zero the fix would be mechanical, and if coverage were 84/85 the tool would
name the line. It is a checklist item promoted to a stage because the checklist
it would otherwise live on does not exist. That absence is itself an argument
for `gate/g1_coverage.sh` growing a bare-`loop` warning, filed as deferred
blocker (x) rather than done here.

---

## Run Record

| Date | Stages | Result |
|------|--------|--------|
| 2026-08-28 | P1–P4 | 4/4 run. Three predictions held outright; P1's held on its stated claim and produced a wrong sub-prediction (one error, not two) that turned into a documentation fix |

### Per-stage

| Stage | Ran | Prediction | Verdict |
|-------|-----|------------|---------|
| P1 | 2026-08-28 | `E0432` unresolved import on `ring_wait` | ✅ on the claim; ⚠️ one error not two, because dev-dependencies resolve, and the scan covers them anyway |
| P2 | 2026-08-28 | Zero transitive hits | ✅ `0`, closure 17 family / 0 external |
| P3 | 2026-08-28 | Under 10 ms | ✅ 8 ms, 62× under the bound, 125× under the parking cost |
| P4 | 2026-08-28 | No bare `loop`, 100% first try | ✅ `0` and `85/85` |

### What this round adds to the family's defect-shape list

Nothing new. P1 is defect shape 5, *a check that is right for the wrong reason*,
arriving from its other side. Instead of a check whose correctness depends on
knowledge it does not carry, it is a check that is **more correct than its
author realised**, for a reason he had not noticed.

The mitigation is the same and the direction is the opposite: when a check passes
for a reason you did not predict, write down the real reason. Otherwise the next
person to read it sees a scan that covers dev-dependencies, assumes that is
sloppiness, tightens it, and removes a guard nobody knew was there.
