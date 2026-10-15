# Manual test plan for `ring_shutdown`

Readings and measurements the automated suite cannot make. Each stage has a
command, a prediction written **before** running it, and the observed result.

The prediction-first order is the point. A stage written after seeing the
output records what happened; a stage written before it can be *wrong*, and a
wrong prediction is the only thing here that teaches anything.

## Why these are manual

| Stage | Why a test cannot do it |
|---|---|
| D1 | The property is that code **does not compile**. A passing suite is silent about code that was never written |
| D2 | A measurement about the coverage tool, not about the crate. The suite cannot observe its own instrumentation |
| D3 | An assertion about the *absence* of a field in every other crate, most of which this one does not depend on |
| D4 | A dependency-closure fact. `cargo tree` is the instrument; no test links against the metadata |

---

## D1. The two `Stopped` properties are compile errors, not runtime ones

The crate claims two things cannot be spelled. A claim of that shape is worth
exactly the compiler error behind it, so this stage produces the errors.

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

`E0599` and `E0382` are two distinct mechanisms. One is "the method is not
there"; the other is "the proof was consumed". That is what makes them two
properties rather than one stated twice.

**What this stage does not establish.** That a caller *cannot* drain an open
ring. They can. `split()` hands out a `Consumer` and `try_recv_batch` is public
on it. The token only forecloses reaching *this crate's* drain without a close.
That is a narrower claim than "an open ring cannot be drained", and the crate
makes only the narrower one. Case B is narrower still. A token from an earlier
`close` outlives `reopen`, as
[ADR 001](../../docs/decisions/001_stopped_tokens_and_into_inner_wait_for_a_real_caller.md)
records.

---

## D2. `llvm-cov` does not attribute a hit to a bare `loop` line

The comment in `drain_all` explains its unnatural `while` spelling by a
coverage artifact. That explanation is only worth the measurement behind it.

**Command.**

```sh
cargo tarpaulin -p ring_shutdown --all-features --skip-clean --out Stdout --engine llvm \
  | grep -oE '^\|\| ring_shutdown/src/[^:]+: [0-9/]+'
```

run once with `drain_all` written as a `loop` with an inner `return`, and once
as it stands. This stage pins `--engine llvm` rather than leaving tarpaulin's
default `Auto`, because the engine changes the reported coverage denominator
for identical source. For the same reproducibility reason, the result below
records the tool version that produced it.

**Prediction.** The `loop` spelling reports one line short. The uncovered line
is the `loop` keyword itself.

**Result (2026-09-03): both halves hold.** Measured with
`cargo-tarpaulin-tarpaulin 0.35.1`, `--engine llvm`.

| Spelling | Coverage | Uncovered line |
|---|---|---|
| `loop` + inner `return` | 80/81 | 224, the bare `loop` |
| `while` + duplicated first read | **81/81** | none |

Tarpaulin names the uncovered line explicitly (`|| ring_shutdown/src/lib.rs: 224`),
so this did not require inference.

**The finding is about the gate, not only the crate.** The coverage gate,
`bench_harness/gate/g1_coverage.sh`, demands 100% and cannot distinguish an
instrumentation artifact from a real gap, so an artifact of this kind forces a
code change to satisfy a measurement. That is a cost of the 100% threshold. The
threshold is right, because it is what makes the number mean anything, but it
does occasionally pay for a measurement with a slightly worse spelling.
`ring_core`'s `cfg`-removed lines are another such case.

---

## D3. There is exactly one liveness flag in the family

The crate makes a negative claim about every other crate in the family. No test
in any of them can assert it.

**Command.**

```sh
grep -rln 'AtomicBool' ring_*/src
grep -rn 'fn is_closed' ring_*/src
```

**Prediction.** First: one file, `ring_shutdown/src/lib.rs`. Second: also
`ring_shutdown` only, with **two hits**, because `Refusal::is_closed` shares the
name and reads no atomic.

**Result (2026-08-28): both hold, including the two-hit refinement.**

```
ring_shutdown/src/lib.rs
ring_shutdown/src/lib.rs:80:  pub fn is_closed( &self ) -> bool      ← Shutdown, loads the flag
ring_shutdown/src/lib.rs:268: pub const fn is_closed( &self ) -> bool ← Refusal, reads an enum arm
```

**Expect two hits, and know why.** "One flag, so one hit" is what the claim's
wording suggests, and only reading the crate shows that a second method shares
the name. A prediction of one hit would fail against a healthy invariant, and
the natural response to that failure, renaming `Refusal::is_closed`, would
change correct code to satisfy a wrong check.

---

## D4. The closure contains no external crate

The crate claims its whole dependency closure is in-house.

**Command.**

```sh
cargo tree -p ring_shutdown -e normal | grep -cv 'ring_'
cargo tree -p ring_shutdown -e normal | grep -oE 'ring_[a-z_]+ v' | sort -u | wc -l
```

**Prediction.** Zero non-family lines. The second command counts the family
crates in the closure, most of which arrive through `ring_core`.

**Result (2026-08-28): 0 external, 18 family. Both hold.**

**The first command is a proxy, and what it tests matters.**
`grep -cv 'ring_'` counts lines lacking the string `ring_`. That works for the
crates that exist, because an external dependency at any depth prints as
`└── serde v1.0.x` with no `ring_` anywhere. But it tests a *name*, while the
claim is about *origin*. An external crate named `ring_something`, or one
vendored under a path containing `ring_`, would pass it.

So the target form filters on the path cargo prints for a local crate:

```sh
cargo tree -p ring_shutdown -e normal | grep -v '/ring_' | grep -c .
```

**Run: `0`, and the unfiltered listing is empty.** The two commands agree
today, which is what makes the proxy safe to keep as the quick form. Both stay
because agreement now is not agreement later, and because a proxy whose target
has never been run is the kind of check that passes whether or not the claim
holds.

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
