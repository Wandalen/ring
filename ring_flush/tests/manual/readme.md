# Manual test plan for `ring_flush`

Readings and measurements the automated suite cannot make. Six stages, each
with a command, a prediction written **before** running it, and the observed
result.

The prediction-first order is the point. A stage written after seeing the output
records what happened; a stage written before it can be *wrong*, and a wrong
prediction is the only thing here that teaches anything. **This round produced
two, and one of them changed the implementation.**

## Why these six are manual

| Stage | Why a test cannot do it |
|---|---|
| F1 | It requires editing `src/lib.rs` to introduce a violation. A test cannot degrade the crate under test |
| F2 | A size claim three instances assert as fact. Confirming it needs a measurement |
| F3 | It asks whether a test's stated premise is true. Nothing fails if the premise is false. The test still passes, forever |
| F4 | It reads the behaviour of a *dependency* to check whether this crate's design rests on something real |
| F5 | The suite runs under one feature configuration. Whether the acceptance criterion survives another is a question about the build, not about the code |
| F6 | It asks whether a type is *not* `Sync`. A negative trait bound is not expressible on stable Rust, so the reading is a compile failure, which a passing test suite by definition cannot contain |

---

## F1. Does the suite go red when `OnBarrier` degenerates?

The failure this crate exists to make visible is an `OnBarrier` policy that
quietly starts firing on every drive. Twenty-four passing tests is also an
accurate description of twenty-four tests that could never fail. The only way to
tell the two apart is to commit the violation.

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
mutation on the same code: **six**. The conclusion below survives intact, and
gets stronger. The not-firing assertion really is the detector, and two more
tests hold that line than this round could see. Found by `g12_mutation.sh`,
which replays this probe as a gate and therefore had to ask what its number
meant; it now runs `--no-fail-fast` so the figure it prints is the suite's
rather than the scheduler's.

**The two extra tell you what the real detector is, and it is not the test named
after the job.** Every one of the four contains an assertion that some drive
returns `NotTriggered`. That is the shape that catches this, and it turns up
incidentally in tests written for other purposes.

So the module documentation's claim that this is the failure "no positive test
can see" was **wrong as stated**. The distinction that matters is not positive
versus negative tests; it is assertions about *firing* versus assertions about
*not firing*. A suite made entirely of "after `drive_at_barrier`, `Flushed`"
would miss it. Four of ours do not, because four of ours happen to pin a
non-firing call along the way. Corrected in `tests/flush_test.rs`, in both the
module header and on `on_barrier_never_fires_without_an_announcement`, and in
`docs/non_functional_requirement/001`'s M2. That test is still the only one that
catches this *deliberately*.

---

## F2. Is `FlushPolicy` really sixteen bytes?

The crate's own module documentation, `docs/data_structure/001`, and
`docs/api/001`'s guarantee 2 all assert sixteen bytes. It is a policy consulted
by value on a path constrained to allocate nothing, so the design relies on the
figure rather than just quoting it, and it had never been measured.

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

`ring_handle`'s equivalent stage found its "one pointer" claim wrong by three
times. This one was right, which is the outcome that produces no documentation
change and is still worth the two minutes. The alternative was three instances
asserting a number nobody had looked at.

The assertion in `the_policy_is_a_value` is written as
`2 * size_of::< usize >()` rather than a literal `16`, for the same reason
`ring_handle`'s was rewritten as an equality: a literal re-pinned to whatever was
measured tests nothing.

`FlushEntry` at 40 bytes was not predicted because nothing depended on it. It is
allocated once per flush on the cold path, and 40 bytes there is noise against a
claim, a copy and a publish.

---

## F3. Is `on_batch_ignores_fullness_and_barriers` testing anything?

The test asserted that an `OnBatch` policy does not also fire when the buffer
fills. That is trigger exclusivity's V2, the case
`docs/data_structure/002` says the log's `cause` field exists to detect. Its
comment claimed the batch size was chosen "so the buffer reaches capacity with
the counter still short of `n`."

That claim is reasoning, not observation. If it is wrong, the test asserts an
absence no implementation could produce, and it is green forever while measuring
nothing.

**Command.** Inject exactly the defect the test names, and run.

```rust
      FlushPolicy::OnBatch( n ) if self.since_flush >= n || self.buffer.is_full()
        => Some( FlushCause::Batch ),  // PROBE ONLY
```

**Prediction.** `on_batch_ignores_fullness_and_barriers` fails. It is the only
test where the buffer reaches capacity under an `OnBatch` policy.

**Result (2026-08-28): WRONG. All 23 tests passed with the defect present.**

```
Summary [0.023s] 23 tests run: 23 passed, 0 skipped
```

### Why, and what it cost

The buffer holds records staged since the last flush. The counter counted
records staged since the last flush. **They were the same number**, on every
path: this driver owns the buffer privately, only `append` adds to it, only a
successful flush empties it, and a rejected flush leaves both untouched
together.

Confirmed rather than argued, with a second probe asserting the equality at every
trigger evaluation across the whole suite:

```rust
assert_eq!( self.since_flush,
  if matches!( self.policy, FlushPolicy::OnBatch( _ ) ) { self.buffer.len() } else { 0 } );
```

```
Summary [0.041s] 23 tests run: 23 passed, 0 skipped
```

Two consequences, and neither is a test fix.

**1. The defect is unreachable, so the test could not have caught it.**
Validation caps `n` at capacity, so `len >= n` holds no later than `len ==
capacity`; and once it holds, the flush empties the buffer. An `OnBatch` policy
never sees a full buffer. **The V2 case the `cause` field is said to exist for
cannot occur for this policy.** The field earns its place for other reasons
(telling `Shutdown` from a policy firing; attributing entries in a mixed log),
not the one the instance gives.

**2. The counter was redundant state on the one path this crate may not make
expensive.** It was deleted: `append` is now a single `push`, `trigger` reads
`buffer.len()`, and the two resets in `run` are gone. All tests still pass, which
is the behavioural evidence that the two quantities never diverged.

The test was rewritten to assert what is true and reachable, that a barrier does
not fire `OnBatch`. A new test,
`the_batch_trigger_arrives_no_later_than_the_buffer_fills`, asserts the property
that makes the fullness case unreachable, across every legal batch size, instead
of leaving it as prose.

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

**Prediction.** `accepted` is 2. The iterator retains two records, 4 and 5, and
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

Recorded as a finding about `ring_core` rather than a defect, because `try_push_batch`
is documented as reporting partial acceptance, and a caller that wants the
remainder must not hand it a `Drain`.

---

## F5. Does the acceptance criterion hold without default features?

Feature 176's criterion is checked by the flush log. Every option the
pre-implementation instance weighed for the log's compilation boundary makes the
answer depend on how the crate is built: a `flush-log` cargo feature,
`cfg(debug_assertions)`, `cfg(test)`. A criterion that holds only under a
non-default feature is a gate that silently tests nothing when the feature is off.

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

**This is the stage that discharges the pending decision rather than deferring
it.** The four options in `docs/data_structure/002`'s table each trade something:
`cfg(test)` is invisible to integration tests, `debug_assertions` ties
observability to a profile, a cargo feature splits the criterion in two, and a
caller-supplied sink puts a generic parameter on an exported type. An opt-in
field pays none of them, and a release build that never calls `with_log`
allocates nothing. That is what all four were trying to buy.

**What it does not buy.** A caller *can* opt in on a hot production path and get
the unbounded growth the instance warns about. Nothing prevents it. The
difference from an always-on log is that it is now a visible call at a known
site rather than a property of the build.

---

## F6. Is a `Flusher` shareable between threads?

`docs/pitfall/001`'s F4 says a barrier signal arriving on a thread other than
the buffer's owner would force a cross-thread seal. If `Flusher` is `Sync`, that
row is a live hazard, because an `&Flusher` could be handed to the announcing thread. If
it is `Send + !Sync`, the hazard is unrepresentable and the row is closed.

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
since `Producer` is `&mut`-only in its own API. It would silently reopen F4, and every
test in `ring_flush` would stay green. Recorded in `docs/pitfall/001` as the
third instance in this crate of a property it depends on and does not own,
beside E3's export boundary and O6's no-parking constraint.

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

## What this round changed

- `src/lib.rs`: the `since_flush` field, its increment in `append`, and its two
  resets in `run` all deleted. `append` is now one `push`.
- `tests/flush_test.rs`: `on_batch_ignores_fullness_and_barriers` rewritten as
  `on_batch_ignores_barriers` with the false premise removed;
  `the_batch_trigger_arrives_no_later_than_the_buffer_fills` added to assert the
  property that makes the removed half unreachable.
- `docs/data_structure/002`: the pending compilation-boundary decision settled,
  and the `cause` field's stated justification corrected.
- `docs/algorithm/001`: step 3, the counter update, deleted as redundant.
- `docs/non_functional_requirement/001`: M2's claim about what only a negative
  test can detect, corrected by F1.

## What the second round changed

- `tests/flush_test.rs`: `a_driver_is_movable_between_threads_and_never_shared`
  added (F6); `the_batch_counter_restarts_after_each_flush` renamed to
  `batches_are_consecutive_not_cumulative`, since the counter it was named for
  no longer exists and the old name would have survived its deletion unnoticed.
- `tests/append_cost_test.rs`: added. C2's safe proxy plus the append path's
  behavioural cost claims, which need no allocator.
- `src/lib.rs`: `Flusher::buffer_capacity` added, so a caller can see a refusal
  coming rather than discovering it.
- `docs/algorithm/002`, `docs/lifecycle/003`, `docs/lifecycle/001`: all
  three described a sealed-and-unreset buffer left by a rejected claim. Checking
  the ring's capacity *before* touching the buffer removes that state entirely;
  all three reconciled, and `lifecycle/003`'s B5 recorded as unreachable.
- `docs/pitfall/001`: F4 closed by F6's measurement, with the inheritance noted.
- `docs/integration/001`, `docs/integration/002`: the `ring_tls` interface narrowed
  to what is called (`drain`, not `flush_into`), and the export-surface
  bypass recorded as impossible for a bound buffer.
- `docs/decisions/readme.md`: Pending 6's stale sentence paraphrased, because
  gate G2 cannot tell a quotation from a live self-description.
