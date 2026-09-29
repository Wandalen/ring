# Manual Test Plan — `ring_shutdown`

Readings and measurements the automated suite cannot make. Four stages, each
with a command, a prediction written **before** running it, and the observed
result.

The prediction-first order is the point. A stage written after seeing the
output records what happened; a stage written before it can be *wrong*, and a
wrong prediction is the only thing here that teaches anything.

## Why These Four Are Manual

| Stage | Why a test cannot do it |
|---|---|
| D1 | The property is that code **does not compile**. A passing suite is silent about code that was never written |
| D2 | A measurement about the coverage tool, not about the crate. The suite cannot observe its own instrumentation |
| D3 | An assertion about the *absence* of a field in 32 crates this one mostly does not depend on |
| D4 | A dependency-closure fact. `cargo tree` is the instrument; no test links against the metadata |

---

## D1 — The two `Stopped` properties are compile errors, not runtime ones

[`type/001`](../../docs/type/001_stopped_proof_token.md) claims two things
cannot be spelled. A claim of that shape is worth exactly the compiler error
behind it, so this stage produces the errors.

**Command.** Write `ring_shutdown/tests/d1_probe.rs` containing both
cases below, run `cargo build --tests -p ring_shutdown`, then delete it.

```rust
// Case A — drain without close.
let shutdown = Shutdown::new();
shutdown.drain_all( &mut consumer, &mut out );

// Case B — drain after reopen.
let stopped = shutdown.close();
stopped.reopen();
stopped.drain_all( &mut consumer, &mut out );
```

**Prediction.** Both fail. Case A because `drain_all` is not on `Shutdown`;
case B because `reopen` consumed the token.

**Result (2026-08-28): both hold, with the error codes named.**

```
error[E0599]: no method named `drain_all` found for struct `ring_shutdown::Shutdown`
  --> ring_shutdown/tests/d1_probe.rs:14:12
error[E0382]: borrow of moved value: `stopped`
    |           -------- `stopped` moved due to this method call
error: could not compile `ring_shutdown` (test "d1_probe") due to 2 previous errors
```

`E0599` and `E0382` are the two distinct mechanisms — one is "the method is not
there", the other is "the proof was consumed" — which is what makes them two
properties rather than one stated twice.

**What this stage does not establish.** That a caller *cannot* drain an open
ring. They can: `split()` hands out a `Consumer` and `try_recv_batch` is public
on it. What the token forecloses is reaching *this crate's* drain without a
close, which is a narrower claim than "an open ring cannot be drained" — and
[`type/001`](../../docs/type/001_stopped_proof_token.md) is careful to make the
narrower one.

---

## D2 — `llvm-cov` does not attribute a hit to a bare `loop` line

[`algorithm/001`](../../docs/algorithm/001_drain_to_empty.md) explains an
unnatural `while` spelling by a coverage artifact. That explanation is only
worth the measurement behind it.

**Command.**

```sh
cargo tarpaulin -p ring_shutdown --all-features --skip-clean --out Stdout --engine llvm \
  | grep -oE '^\|\| ring_shutdown/src/[^:]+: [0-9/]+'
```

run once with `drain_all` written as a `loop` with an inner `return`, and once
as it stands. `--engine llvm` is pinned rather than left at tarpaulin's
default `Auto`, because the engine changes the reported coverage denominator
for identical source — the tool version that produced a result is recorded
alongside it, below, for the same reproducibility reason.

**Prediction.** The `loop` spelling reports one line short. The uncovered line
is the `loop` keyword itself.

**Result (2026-09-03): both halves hold.** Measured with
`cargo-tarpaulin-tarpaulin 0.35.1`, `--engine llvm`.

| Spelling | Coverage | Uncovered line |
|---|---|---|
| `loop` + inner `return` | 80/81 | 224 — the bare `loop` |
| `while` + duplicated first read | **81/81** | — |

Tarpaulin names the uncovered line explicitly (`|| ring_shutdown/src/lib.rs: 224`),
so this did not require inference.

**The finding is about the gate, not only the crate.** G1 demands 100% and
cannot distinguish an instrumentation artifact from a real gap, so an artifact
of this kind forces a code change to satisfy a measurement. That is a cost of
the 100% threshold and worth stating plainly: the threshold is right — it is
what makes the number mean anything — but it does occasionally pay for a
measurement with a slightly worse spelling. This is the second such case in the
family, after `ring_core`'s `cfg`-removed lines
([`ring_core/docs/pitfall/002`](../../../ring_core/docs/pitfall/002_feature_gated_code_reads_as_uncovered.md)).

---

## D3 — There is exactly one liveness flag in the family

[`invariant/001`](../../docs/invariant/001_exactly_one_liveness_flag.md) is a
negative about 32 other crates. No test in any of them can assert it.

**Command.**

```sh
grep -rln 'AtomicBool' ring_*/src
grep -rn 'fn is_closed' ring_*/src
```

**Prediction.** First: one file, `ring_shutdown/src/lib.rs`. Second: also
`ring_shutdown` only — **two hits**, because `Refusal::is_closed` shares the
name and reads no atomic.

**Result (2026-08-28): both hold, including the two-hit refinement.**

```
ring_shutdown/src/lib.rs
ring_shutdown/src/lib.rs:80:  pub fn is_closed( &self ) -> bool      ← Shutdown, loads the flag
ring_shutdown/src/lib.rs:268: pub const fn is_closed( &self ) -> bool ← Refusal, reads an enum arm
```

The second prediction was written to expect two hits **because the crate was
read first**, and that is the only reason it is not defect shape 2 from
`ring_core`'s plan summary — a count where the identity is what matters. A
prediction of "one hit" would have failed against a perfectly healthy
invariant, and the natural response to that failure would have been to rename
`Refusal::is_closed`, changing correct code to satisfy a wrong check.

**Recorded as a near miss rather than a success.** The prediction was right by
one deliberate act of reading; the shape that produces the wrong version of it
is the default.

---

## D4 — The closure contains no external crate

[`integration/001`](../../docs/integration/001_family_dependency_seam.md)
claims the whole dependency closure is in-house.

**Command.**

```sh
cargo tree -p ring_shutdown -e normal | grep -cv 'ring_'
cargo tree -p ring_shutdown -e normal | grep -oE 'ring_[a-z_]+ v' | sort -u | wc -l
```

**Prediction.** Zero non-family lines. Family crates in the closure: fewer than
20 — this crate sits above `ring_core`, whose own closure is most of the tier-1
and tier-2 family.

**Result (2026-08-28): 0 external, 18 family. Both hold.**

**The first command is a proxy, and naming what it actually tests matters.**
`grep -cv 'ring_'` counts lines lacking the string `ring_`. That works for the
crates that exist — an external dependency at any depth prints as
`└── serde v1.0.x` with no `ring_` anywhere — but it tests a *name*, while the
claim is about *origin*. An external crate named `ring_something`, or one
vendored under a path containing `ring_`, would pass it.

So the target form filters on the path cargo prints for a local crate:

```sh
cargo tree -p ring_shutdown -e normal | grep -v '/ring_' | grep -c .
```

**Run: `0`, and the unfiltered listing is empty** — the two commands agree
today, which is what makes the proxy safe to keep as the quick form. Recorded
because agreement now is not agreement later
(→ `pln_staged.rulebook.md § Measurement : Target Over Proxy`), and
because a proxy whose target has never been run is the shape that passes
whether or not the claim holds.

---

## Run Record

| Date | Stages | Result |
|------|--------|--------|
| 2026-08-28 | D1–D4 | 4/4 run. All four predictions held. D3 held only because the crate was read before the prediction was written; D4's quick command is a proxy, and its target form was run alongside it and agreed |

### Per-stage

| Stage | Ran | Prediction | Verdict |
|-------|-----|------------|---------|
| D1 | 2026-08-28 | Both cases fail to compile | ✅ `E0599` and `E0382`, two distinct mechanisms |
| D2 | 2026-09-03 | `loop` costs one uncovered line | ✅ 80/81 → 81/81, line 224 named by the tool, `--engine llvm` pinned |
| D3 | 2026-08-28 | One flag; two `is_closed` hits | ✅ both, and the two-hit half was a near miss |
| D4 | 2026-08-28 | 0 external, <20 family | ✅ 0 and 18, by both the proxy and its target form |

### What This Round Adds to the Family's Defect-Shape List

`ring_core`'s plan summary names four shapes a manual check fails in. D3 and D4
are both instances of a **fifth**, which is not on that list:

> **A check that is right for the wrong reason.** D3's prediction is correct,
> but the reasoning that produces the correct version ("two hits, because a
> second method shares the name") is available only to someone who has read the
> file. The reasoning that produces the *incorrect* version ("one flag, so one
> hit") is what the invariant's own wording suggests. A check whose correctness
> depends on knowledge the check does not carry will be broken by the next
> person who edits it in good faith.

The mitigation is in the doc rather than in the command: `invariant/001` now
states the expected hit count *and* why it is two, so the reasoning travels
with the check instead of living in whoever wrote it.
