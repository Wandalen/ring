# Manual testing for ring_testkit

Two stages matter most. **M1** checks the claim the crate is shaped around.
**M2** is a negative control on the loom model. Without it, a model that
explored nothing and a model that explored everything would look identical from
the outside.

Run from the repository root.

---

## M1: which reading of a run sees a dropped record?

`ring_core::try_push` reports `Ok` on a full ring under
`OverflowPolicy::DropNewest` and discards the record. One script runs against
two rings differing only in that policy. It pushes eight records into a
four-slot ring, then drains everything.
`neither_the_count_nor_the_delivered_records_sees_a_drop_alone` in
`tests/testkit_test.rs` pins the result.

The obvious guess is that the two rings produce identical counts and different
delivered records. **Expected:** the inverse.

```
Fail:       accepted=4 refused_full=4 received=[0, 1, 2, 3] vanished=0 audit=Ok(())
DropNewest: accepted=8 refused_full=0 received=[0, 1, 2, 3] vanished=4 audit=Ok(())
```

The counts **differ**. The delivered records are **identical**. The inversion
matters, because the reading that fails is the one a careful person reaches for
first. `received` is the ground truth of what a ring delivered. Both rings
delivered `[0,1,2,3]`, so it says they are the same. The counts do separate
them, into `accepted=4` and `accepted=8`, which reads as *one ring took twice as
much work* rather than as *one ring destroyed half of it*.

`Outcome::vanished`, which is accepted, less delivered, less still held, is the
only reading that names the destruction, and it exists because of this run.

**Also expected:** a stage-then-flush of 8 records through a 6-slot buffer into
a 4-slot ring gives `refused_staging=2, accepted=4, refused_full=2,
staged_at_end=0`. The fixture counts the two refusal kinds apart, and the flush
empties the buffer either way.

---

## M2: does the loom model explore, or does it trivially pass?

```bash
RUSTFLAGS="--cfg loom" cargo test -p ring_testkit --test exhaustive_test
```

The models pass fast enough to be suspicious. A `loom::model` that explored one
interleaving and a `loom::model` that explored two hundred look identical from
the outside, and the failure mode of a broken model is that it passes. Every
other stage checks something the automated suite also touches; this one checks
that the suite's *loom half* is doing anything at all, which no test inside it
can establish about itself.

So break the model deliberately. Invert the drain assertion in
`no_interleaving_delivers_a_record_that_was_not_published` to
`received.len() >= 1`, which claims "the drain always sees the record". That is
true only in the interleavings where the producer runs first.

**Expected:** it fails. If loom is exploring, there is an execution in which the
consumer looks before the producer publishes.

```
thread 'no_interleaving_delivers_a_record_that_was_not_published' panicked at
  ring_testkit/tests/exhaustive_test.rs:LINE:COL:
NEGATIVE CONTROL: asserts the drain always sees the record
```

Restore the file and re-run; every model passes. **The model explores.** Without
this stage a fast pass would be evidence of nothing.

---

## M3: is the loom model reachable at all from a `ring_core::Ring`?

`loom::thread::spawn` takes `'static` closures; `Ring::ends()` returns an `Ends`
that `split` then borrows.

**Expected:** one leak is not enough. `leak` gives the *ring* a `'static`
lifetime, but the `Ends` is still a local, so the `Producer` is bounded by it.
The error message points at the local, not at the missing leak. `leak_ends`
performs both and is why the pair exists rather than one function with a
caveat.

Confirmed under an ordinary build too, by
`leak_ends_produces_ends_that_can_be_moved_onto_spawned_threads`, which moves
both ends onto `std::thread::spawn`. The lifetime is the property under test and
it is the same lifetime either runtime demands. That matters because
`exhaustive_test.rs` compiles to nothing without the cfg, so a helper only it
used would rot unnoticed.

---

## M4: are all declared dependencies used?

```bash
cargo +nightly udeps -p ring_testkit --all-targets --all-features
```

**Expected:** `All deps seem to have been used.` `ring_core` is the ring,
`ring_tls` is the staging buffer, and `ring_shutdown` is the guard.
`ring_config` and `ring_types` are dev-dependencies, both used by the tests.

---

## M5: is the crate fully covered by an ordinary run?

```bash
cargo tarpaulin -p ring_testkit --all-features --out Stdout 2>&1 \
  | grep 'ring_testkit/src'
```

**Expected:** 100% line coverage. `leak_ends` is exercised by a doc test and by
the loom model, and tarpaulin counts neither, so two ordinary tests carry the
lines that would otherwise be uncovered:

| Line | Covered by |
|---|---|
| `leak_ends` | `leak_ends_produces_ends_that_can_be_moved_onto_spawned_threads` |
| `Flush`'s closed-refusal arm | `a_flush_into_a_closed_ring_is_refused_as_closed` |

The second is the better test of the two. A flush into a *closed* ring with
room to spare distinguishes the two refusal reasons, which is the distinction
`refused_full` and `refused_closed` exist to keep apart.

---

## M6: is the crate clippy-clean under `-D warnings`, including under the loom cfg?

```bash
cargo clippy -p ring_testkit --all-targets --all-features        -- -D warnings
cargo clippy -p ring_testkit --all-targets --no-default-features -- -D warnings
RUSTFLAGS="--cfg loom" cargo clippy -p ring_testkit --all-targets --all-features -- -D warnings
```

**Expected:** clean in all three. The third is worth running because nothing
else in this crate's routine checks compiles the loom module at all. A lint
failure there would be invisible until someone set the cfg.

`Script::run` carries an `allow` for `clippy::too_many_lines`, and its `reason`
states where the length comes from.

---

## Run Record

| Stage | Question | 2026-08-28 |
|---|---|---|
| M1 | Which reading sees a dropped record | ❌ **prediction wrong, both halves**: counts differ, records identical |
| M2 | Does the loom model explore | ✅ negative control fails, restored run passes |
| M3 | Is one leak enough for loom | ✅ no. Two leaks, hence `leak_ends` |
| M4 | No unused dependencies | ✅ clean; all three declared edges real |
| M5 | Fully covered on an ordinary run | ✅ predicted 79/83 first, then 83/83 |
| M6 | Clippy-clean, including under `--cfg loom` | ✅ clean in all three configurations |

**Environment:** `2026-08-28`, Linux 6.8.0, `cargo nextest`, `cargo tarpaulin`,
`cargo +nightly udeps`, `cargo clippy`, `loom 0.7`. 27 tests + 4 doc tests + 3
loom models, all passing.
