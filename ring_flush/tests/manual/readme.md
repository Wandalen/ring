# Manual test plan for `ring_flush`

Readings and measurements the automated suite cannot make. Six stages, each
with a command, a prediction written **before** running it, and the observed
result.

The prediction-first order is the point. A stage written after seeing the output
records what happened; a stage written before it can be *wrong*, and a wrong
prediction is the only thing here that teaches anything.

## Why these six are manual

| Stage | Why a test cannot do it |
|---|---|
| F1 | It requires editing `src/lib.rs` to introduce a violation. A test cannot degrade the crate under test |
| F2 | A size claim the design relies on. Confirming it needs a measurement |
| F3 | It asks whether a test's stated premise is true. Nothing fails if the premise is false. The test still passes, forever |
| F4 | It reads the behaviour of a *dependency* to check whether this crate's design rests on something real |
| F5 | The suite runs under one feature configuration. Whether the acceptance criterion survives another is a question about the build, not about the code |
| F6 | It asks whether a type is *not* `Sync`. A negative trait bound is not expressible on stable Rust, so the reading is a compile failure, which a passing test suite by definition cannot contain |

---

## F1. Does the suite go red when `OnBarrier` degenerates?

The failure this crate exists to make visible is an `OnBarrier` policy that
quietly starts firing on every drive. A green suite is also what a suite that
could never fail looks like. The only way to tell the two apart is to commit the
violation.

**Command.** Drop the `at_barrier` guard, run, revert.

```rust
      FlushPolicy::OnBarrier => Some( FlushCause::Barrier ),  // PROBE ONLY
```

```sh
cargo nextest run -p ring_flush --all-features
```

**Prediction.** Two failures: `on_barrier_never_fires_without_an_announcement`,
the dedicated negative, and `on_barrier_fires_only_when_a_barrier_is_announced`,
which asserts a plain drive returns `NotTriggered` before announcing.

**Result (2026-08-28): red, and by more than predicted, with four failures.**

```
FAIL  a_call_that_did_not_fire_is_not_recorded
FAIL  on_barrier_never_fires_without_an_announcement
FAIL  an_empty_trigger_is_recorded_and_is_not_a_non_trigger
FAIL  on_barrier_fires_only_when_a_barrier_is_announced
Summary  16/23 tests run: 12 passed, 4 failed
```

**Correction (2026-08-29): the count above is wrong, and the reading built on
it was undercounted.** `16/23 tests run` is nextest stopping at the first
failures, which is its default. So *four* is how many had run when it gave up,
not how many catch this. Re-run with `--no-fail-fast` against the same
mutation on the same code: **six**. Add `--no-fail-fast` whenever the number of
failures is the reading.

**The extra failures tell you what the real detector is, and it is not the test
named after the job.** Every failing test contains an assertion that some drive
returns `NotTriggered`. That is the shape that catches this, and it turns up
incidentally in tests written for other purposes.

So the distinction that matters is not positive versus negative tests; it is
assertions about *firing* versus assertions about *not firing*. A suite made
entirely of "after `drive_at_barrier`, `Flushed`" would miss it. Ours does not,
because several tests happen to pin a non-firing call along the way.
`on_barrier_never_fires_without_an_announcement` is still the only one that
catches this *deliberately*.

---

## F2. Is `FlushPolicy` really sixteen bytes?

`FlushPolicy` is a policy consulted by value on a path constrained to allocate
nothing, so the design relies on it staying small: a discriminant plus one
`usize`, sixteen bytes on a 64-bit target. This stage measures it.

**Command.** A throwaway `tests/zz_size_probe.rs` printing `size_of` for every
exported type. (The name matters, because `cargo test --test` parses a file named
with a leading hyphen as a flag.)

**Prediction.** `FlushPolicy` is 16: a discriminant plus one `usize` payload,
padded to the payload's alignment. `FlushOutcome` likewise 16. `FlushCause` 1.

**Result (2026-08-28): holds.**

| Type | Predicted | Measured |
|---|---|---|
| `FlushPolicy` | 16 | **16** |
| `FlushOutcome` | 16 | **16** |
| `FlushCause` | 1 | **1** |
| `FlushEntry` | not predicted | 40 |
| `usize` | 8 | 8 |

The assertion in `the_policy_is_a_value` is written as
`2 * size_of::< usize >()` rather than a literal `16`, because a literal
re-pinned to whatever was measured tests nothing.

`FlushEntry` at 40 bytes was not predicted because nothing depended on it. It is
allocated once per flush on the cold path, and 40 bytes there is noise against a
claim, a copy and a publish.

---

## F3. Can an `OnBatch` policy ever see a full buffer?

An `OnBatch` policy that also fired when the buffer fills would break trigger
exclusivity, and a test asserting it does not looks like the obvious guard. That
test is only worth having if the defect is reachable. If it is not, the test
asserts an absence no implementation could produce, and it is green forever
while measuring nothing.

**Command.** Inject exactly that defect into the `OnBatch` arm of `trigger`, and
run.

```rust
        if self.buffer.len() >= n || self.buffer.is_full() {  // PROBE ONLY
```

**Prediction.** The test where the buffer reaches capacity under an `OnBatch`
policy fails.

**Result (2026-08-28): WRONG. Every test passed with the defect present.**

```
Summary [0.023s] 23 tests run: 23 passed, 0 skipped
```

### Why

**The defect is unreachable, so no test can catch it.** Validation caps `n` at
capacity, so `len >= n` holds no later than `len == capacity`; and once it
holds, the flush empties the buffer. An `OnBatch` policy never sees a full
buffer. The log's `cause` field earns its place for other reasons: telling
`Shutdown` from a policy firing, and attributing entries in a mixed log.

This probe also exposed a batch counter the driver then kept as redundant. A
second probe asserted at every trigger evaluation that the counter equalled
`buffer.len()`, and every test passed. This driver owns the
buffer privately, only `append` adds to it, only a successful flush empties it,
and a rejected flush leaves both untouched together. So the counter was deleted,
and `append` is a single `push`.

`on_batch_ignores_barriers` asserts what is true and reachable, that a barrier
does not fire `OnBatch`. `the_batch_trigger_arrives_no_later_than_the_buffer_fills`
asserts the property that makes the fullness case unreachable, across every
legal batch size, instead of leaving it as prose.

**A test whose premise is false is worse than a missing test**, because it
occupies the slot where a real one would go and reports success while doing it.
This stage costs one injection and forty seconds, and it is the only thing that
distinguishes the two.

---

## F4. Does `try_push_batch` really destroy the record it refuses?

`run` checks the ring's free capacity **before** touching the buffer. The whole
rejected-flush guarantee, that records stay staged and the call is safe to retry,
rests on that check being necessary rather than belt-and-braces. It is
necessary only if the alternative loses data.

**Command.** Push five records through `ring_core::Producer::try_push_batch`
into a two-slot `Fail` ring, then collect what the iterator still holds.

**Prediction.** `accepted` is 2. The iterator retains two records, the fourth and fifth, and
one record is unaccounted for, because `try_push_batch` calls `try_push`, which returns
the refused record in an `Err`, and the loop discards it on `break`.

**Result (2026-08-28): holds.**

```
accepted  2
leftover  [13, 14]
lost      1
```

Record `12` is gone. It was consumed from the iterator, refused by the ring, and
dropped with the `Err` that carried it.

**So the pre-check is the guarantee, not a courtesy.** Draining first and pushing
optimistically would silently destroy one record per rejection, and report
`Flushed` while doing it. This also settles why `run` carries no
`debug_assert!` on the accepted count. By the time a shortfall is observable the
record is already gone, and an assertion that only fires in debug builds would
promise a guarantee that evaporates exactly where the race is likely.

This is not a defect in `ring_core`. `try_push_batch` is documented as reporting
partial acceptance, and a caller that wants the remainder must not hand it a
`Drain`.

---

## F5. Does the acceptance criterion hold without default features?

The acceptance criterion, that each policy fires at exactly its trigger and at
no other point, is checked by the flush log. Every obvious compilation boundary
for the log makes the answer depend on how the crate is built: a `flush-log`
cargo feature, `cfg(debug_assertions)`, `cfg(test)`. A criterion that holds only
under a non-default feature is a gate that silently tests nothing when the
feature is off.

**Command.**

```sh
cargo nextest run -p ring_flush --no-default-features
```

**Prediction.** Identical result to the `--all-features` run. The log is an
`Option` on the driver, opted into by `with_log`, with no `cfg` anywhere, so
there is no configuration in which the criterion is checked differently.

**Result (2026-08-28): holds.**

```
Summary [0.030s] 24 tests run: 24 passed, 0 skipped
```

**This stage is the evidence for the opt-in log.** The alternatives each trade
something: `cfg(test)` is invisible to integration tests, `debug_assertions` ties
observability to a profile, a cargo feature splits the criterion in two, and a
caller-supplied sink puts a generic parameter on an exported type. An opt-in
field pays none of them, and a release build that never calls `with_log`
allocates nothing. That is what all four were trying to buy.

**What it does not buy.** A caller *can* opt in on a hot production path and get
unbounded growth. Nothing prevents it. The
difference from an always-on log is that it is now a visible call at a known
site rather than a property of the build.

---

## F6. Is a `Flusher` shareable between threads?

A barrier signal arriving on a thread other than the buffer's owner would force
a cross-thread seal. If `Flusher` is `Sync`, that is a live hazard, because an
`&Flusher` could be handed to the announcing thread. If it is `Send + !Sync`,
the hazard is unrepresentable.

**Command.** Add the assertion the answer would have to satisfy, and compile.

```rust
  fn assert_sync< T : Sync >() {}
  assert_sync::< ring_flush::Flusher< '_, u32 > >();   // PROBE ONLY
```

```sh
cargo build -p ring_flush --tests
```

**Prediction.** It fails to compile, and the reason is `ring_tls`'s `TlsBuffer`
holding a `Vec` behind `&mut`.

**Result (2026-08-28): fails to compile, and the prediction named the wrong
cause.** The compiler points at `ring_core`, not `ring_tls`:

```text
note: required because it appears within the type `Producer<'_, u32>`
note: required because it appears within the type `Flusher<'_, u32>`
```

`ring_core::ProducerInner` is not `Sync`, so nothing holding one is. A `Vec` is
`Sync` when `T` is, so the buffer would not have blocked it at all.

**Why the wrong cause matters.** The property is **inherited from a crate this
one does not control**, and it is inherited from the dependency I did not
suspect. `ring_core` making `Producer` `Sync` would look like an ordinary change,
since `Producer` is `&mut`-only in its own API. It would silently reopen the
cross-thread seal hazard, and every test in `ring_flush` would stay green.

The suite keeps the half it can hold: `a_driver_is_movable_between_threads_and_never_shared`
asserts `Send` and records the `!Sync` measurement, with the caveat that the
assertion would survive the regression.

---

## Run Record

| Date | Stages | Result |
|---|---|---|
| 2026-08-28 | F1–F5 | 5/5 run. Three predictions held; F1's was too weak and F3's was wrong. F3 removed a field from the driver, deleted an increment from the append path, replaced one test and added another |
| 2026-08-28 | F6 | 1/1 run. The prediction's verdict held and its stated cause was wrong. The `!Sync` comes from `ring_core`, not `ring_tls` |

| Stage | Date | Prediction | Outcome |
|---|---|---|---|
| F1 | 2026-08-28 | Two tests catch the `OnBarrier` degeneration | **Four did.** The detector is any assertion that a drive returns `NotTriggered`, not the test named for the job. Module doc corrected |
| F2 | 2026-08-28 | `FlushPolicy` is 16 bytes | Holds at 16. Assertion written as an equality with `2 × usize`, not a literal |
| F3 | 2026-08-28 | The V2 injection fails one test | **Wrong. All 23 passed.** The counter was redundant with buffer occupancy; the defect is unreachable; the field was deleted |
| F4 | 2026-08-28 | `try_push_batch` loses exactly one record per refusal | Holds: 5 in, 2 accepted, 2 retained, 1 gone |
| F5 | 2026-08-28 | The criterion holds without default features | Holds at 24/24, same as `--all-features` |
| F6 | 2026-08-28 | `Flusher` is not `Sync`, because of `TlsBuffer` | **Verdict right, cause wrong.** Not `Sync` because of `ring_core::Producer`; a `Vec` would not have blocked it |
