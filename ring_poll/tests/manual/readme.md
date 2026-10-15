# `ring_poll` manual test plan

Readings and measurements the automated suite cannot make. Each stage has a
command, a prediction written **before** running it, and the observed result.

The prediction-first order is the point. A stage written after seeing the output
records what happened; a stage written before it can be *wrong*, and a wrong
prediction is the only thing here that teaches anything.

## Why these are manual

| Stage | Why a test cannot do it |
|---|---|
| P1 | The property is that a name **does not resolve**. A test cannot name a crate it cannot see, so the only way to observe the error is to add the import and fail to build |
| P2 | A transitive dependency-graph fact. `cargo tree` is the instrument; no test links against the metadata |
| P3 | The suite asserts a bound. What the bound is *worth*, meaning the ratio between the measured cost and the parking cost it must discriminate from, is a reading, not an assertion |
| P4 | A check for a known coverage trap. Nothing fails if the trap is present; the cost shows up later as a coverage gap |

---

## P1: reaching for a parking operation is a resolution failure

The crate claims the parking operations are not reachable from it. The claim is
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

That distinction is the finding. The roster check in `tests/poll_test.rs`,
`the_tick_path_cannot_reach_a_parking_operation`, reads the whole manifest,
dev-dependencies included, so a `ring_wait` *dev*-dependency would fail the suite
even though nothing on the tick path could reach it. That is the conservative
direction to be wrong in.

**Keep that scan over the whole manifest.** A prediction of two errors would have
been wrong, and the natural repair would have been to narrow the scan to
`[dependencies]` only. That would weaken a correct check to satisfy a wrong
reading of it. When a check passes for a reason nobody predicted, the reason
belongs in writing, or the next reader tightens the check and removes a guard
nobody knew was there.

---

## P2: nothing reaches `ring_wait`, at any depth

The manifest scan is a **proxy** that tests a name in one file, while the claim
is about *reachability* through the whole graph. A crate that got parking
transitively, say through `ring_core`, would pass the proxy and violate the
claim.

**Command.**

```sh
cargo tree -p ring_poll | grep -c ring_wait
```

**Prediction.** Zero. Nothing in `ring_core`'s closure waits.

**Result (2026-08-28): `0`.** The full closure is 17 family crates and 0 external
ones, and `ring_wait` is in neither set.

The two checks agree today, which is what makes the proxy safe to keep as the
fast form (it runs in the suite; this does not). This stage is kept because
agreement now is not agreement later, and because a proxy whose target has never
been run is the shape that passes whether or not the claim holds.

---

## P3: what the 500 ms bound is worth

`a_large_budget_spins_rather_than_sleeping` bounds 20 000 attempts at 500 ms.
The number comes from arithmetic. A `Park` pause sleeps 50 µs per attempt, so a
parking implementation would cost about 1.0 s. The arithmetic is only worth the
measured side of it.

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
is not; it is a diagnostic value chosen to make the two costs separable. A
caller who used it in earnest would never park and would still miss the frame.

---

## P4: no bare `loop` line costs coverage

`llvm-cov` never attributes a hit to a bare `loop` keyword on its own line, so
each such line costs one uncovered line. This stage checks the crate has none.

**Command.**

```sh
grep -cE '^\s*loop\s*$' ring_poll/src/lib.rs
cargo tarpaulin -p ring_poll --all-features --skip-clean --out Stdout \
  | grep -oE '^\|\| ring_poll/src/[^:]+: [0-9/]+'
```

**Prediction.** Zero bare `loop` lines, and 100% line coverage.

**Result (2026-08-28): `0` bare loops, and `85/85` on the first run.**

**Caveat.** This stage cannot fail *usefully*. A non-zero grep has a mechanical
fix, and a coverage gap names its line. It is a checklist item kept as a stage
because no checklist exists for it. A bare-`loop` warning in
`bench_harness/gate/g1_coverage.sh` would replace it.

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
