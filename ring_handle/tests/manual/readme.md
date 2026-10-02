# Manual test plan for `ring_handle`

Readings and measurements the automated suite cannot make. Five stages, each
with a command, a prediction written **before** running it, and the observed
result.

The prediction-first order is the point. A stage written after seeing the output
records what happened; a stage written before it can be *wrong*, and a wrong
prediction is the only thing here that teaches anything.

## Why these five are manual

| Stage | Why a test cannot do it |
|---|---|
| H1 | The suite reports pass/fail. Whether each case fails *for the intended reason* is a reading of every `.stderr` file. A case rejected for a typo passes exactly as loudly as one rejected for the right reason |
| H2 | It requires editing `src/lib.rs` to introduce a violation. A test cannot add a method to the crate under test |
| H3 | It asks whether a case is right for the reason its author believed. Nothing fails if the reason is wrong; the case still passes |
| H4 | A size claim that reads as obvious. Confirming it needs a measurement, and the measurement is what showed it was wrong |
| H5 | A reading of *why* a guard fired. The suite reported red; whether that red was the guard working or the guard misreading its own documentation is a judgement about the match, not about the result |

---

## H1. Every case is rejected, and for the stated reason

`tests/ui_test.rs` reports one pass for all its programs. That single bit says
nothing about *why* each was rejected. A compile-fail case that fails for an
unrelated reason, such as a typo'd method, a missing import or a moved value, is
the standard way such a suite rots.

**Command.**

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle
for f in tests/ui/*.stderr; do printf '%s: ' "$f"; head -1 "$f"; done
```

**Prediction.** The absence cases report `E0599: no method named <X> found`,
each naming the method the case is *about*. The `!Sync` case reports a
trait-bound error mentioning `Sync`, not `E0599`. That is a different kind of
rejection, and a case reporting `E0599` there would mean the program was
rejected before it ever got to the auto-trait.

**Result (2026-08-28): holds.**

```
tests/ui/consumer_clones.stderr: error[E0599]: no method named `clone` found for struct `ring_handle::Consumer<'a, T>` in the current scope
tests/ui/consumer_publishes.stderr: error[E0599]: no method named `try_push` found for struct `ring_handle::Consumer<'a, T>` in the current scope
tests/ui/producer_clones.stderr: error[E0599]: no method named `clone` found for struct `ring_handle::Producer<'a, T>` in the current scope
tests/ui/producer_drains.stderr: error[E0599]: no method named `try_recv` found for struct `ring_handle::Producer<'a, T>` in the current scope
tests/ui/producer_shared_across_threads.stderr: error[E0277]: `Cell<()>` cannot be shared between threads safely
```

Each `E0599` names the method its own case is about: `try_recv` on the
producer, `try_push` on the consumer, `clone` on each. The `!Sync` case is
`E0277` on `Cell<()>`, as predicted.

The listing above predates two cases. `producer_try_clones` reports
`E0599: no method named try_clone`, and `ring_used_after_split` reports
`E0382: borrow of moved value: ring`. The second is the only case here rejected
by the *borrow checker* rather than by name resolution.

**The `!Sync` line is the one worth reading twice.** It does not mention
`ring_handle` at all in its first line. The `!Sync` property is inherited from
`ring_spsc`'s `PhantomData< Cell< () > >`, two crates down, and the pinned
stderr records the whole chain (`Cell<()>` → `ring_spsc::Producer` →
`ring_core::ProducerInner` → `ring_handle::Producer`). **A reader who asks "why
can't I share this" gets both the answer and the owning crate from a test
artifact**, which is more than the assertion itself was asked to provide.

---

## H2. The suite goes red when the violation is real

A set of programs that do not compile is also what a set of programs containing
typos looks like. H1 rules out the typo; it does not establish that the
mechanism *responds* to the thing it exists to detect. The only way to establish
that is to commit the violation.

**Command.** Add the forbidden drain to `Producer` in `src/lib.rs`, run the
suite, remove it, run again.

```rust
  /// PROBE ONLY — the forbidden drain, added to prove the case goes red.
  pub fn try_recv( &mut self ) -> Option< T >
  {
    None
  }
```

```sh
cargo test -p ring_handle --test ui_test
```

**Prediction.** `producer_drains.rs` fails, with trybuild reporting that it
compiled when it was expected not to. The other cases still pass, since none of
them mentions `try_recv`. After removing the method the suite is green again.

**Result (2026-08-28): holds.**

```
test tests/ui/producer_drains.rs ... error
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

and after reverting:

```
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**This is the stage that separates measurement from decoration**, and it is the
one a compile-fail suite most often skips. A suite that could never fail still
satisfies every step that describes what the cases *are*: they exist, they are
pinned, they compile against the real API. Only an injected violation
distinguishes the two, and it costs one edit and forty seconds.

---

## H3. Is `producer_clones.rs` right for the reason its author believed?

The case binds `producer` by value and calls `.clone()`. The comment in the file
claims the case depends on the by-value binding. With a `&Producer` binding,
`.clone()` would resolve to `<&Producer as Clone>::clone` and **compile**,
copying the reference rather than the handle.

That claim is reasoning, not observation. If it is wrong, and `.clone()` fails
on a reference too, then the case is still correct but for a reason nobody
checked, and the file's warning misleads the next reader.

**Command.** A throwaway test binding a reference and cloning it twice. Cloning
twice is the discriminator: a `Producer` is not `Copy`, so a second use would be
a use-after-move; a `&Producer` is `Copy`, so it compiles.

```rust
let shared = &producer;
let second = shared.clone();
let _third = second;
let _fourth = second;   // compiles only if `second` is a reference
```

**Prediction.** It compiles. `second` is `&Producer`, and the double use proves
it, because there is no `E0382: use of moved value`.

**Result (2026-08-28): holds.** The probe compiled and ran; `second` was used
twice without a move error.

**So the case is right for exactly the reason claimed**, and the warning in the
file is worth its space. The case depended on the author's reasoning, and the
reasoning turned out to be sound. That outcome
produces no documentation change and is still worth the two minutes, because an
unchecked "obviously" is how a case quietly becomes decorative.

**One residual, stated because the probe does not cover it:** a future editor
who rewrites the case to use a reference does not create a silent hole. The file
would compile, and trybuild fails a `compile_fail` case that compiles. That
degradation is loud. The one this case cannot see is the one H1 covers.

---

## H4. What do the handles weigh?

A handle looks like "one pointer", 8 bytes. This stage measures it.

**Command.**

```rust
println!( "producer={} consumer={} split={} drain={}",
  size_of::< ring_handle::Producer< '_, u32 > >(),
  size_of::< ring_handle::Consumer< '_, u32 > >(),
  size_of::< ring_handle::Split< u32 > >(),
  size_of::< ring_handle::Drain< '_, '_, u32 > >() );
```

**Prediction.** `Producer` and `Consumer` are 8 bytes each. `Split` is the size
of a `Ring`. `Drain` is 16: a reference and a `usize`.

**Result (2026-08-28): the prediction is wrong on the two that mattered.**

| Type | Predicted | Measured | Why |
|---|---:|---:|---|
| `Producer< '_, u32 >` | 8 | **24** | An enum discriminant selecting the backend, the reference, and an `OverflowPolicy` |
| `Consumer< '_, u32 >` | 8 | **16** | Discriminant and reference; no policy, because refusal is the producer's concern |
| `Split< u32 >` | ring-sized | 384 | It owns the whole `Ring`, which is the point of taking it by value |
| `Drain< '_, '_, u32 >` | 16 | 16 | Held |

**Where the wrong number came from is the finding.** "One pointer" describes a
hand-rolled handle over a single backend. The real handle wraps a *dispatching*
handle. `ring_core` picks among its backends at runtime, so a discriminant is
unavoidable, and the producer also carries the policy that decides whether a
full ring refuses or drops.

**The fix is not to correct 8 to 24.** An absolute figure is wrong again the
moment `ring_core` adds a backend. `tests/handle_test.rs`'s
`the_wrapper_costs_nothing` asserts **equality with the wrapped handle**
instead. That equality is the property "one pointer" was reaching for: this
crate's newtype adds no field. It also stays true across layout changes one
crate down.

---

## H5. Why did the source scan go red?

`handle_test.rs::no_parking_shaped_name_appears_in_the_source` closes a gap the
dependency check leaves. `ring_poll::PARKING_CRATES` watches the dependency
graph, so a `std::thread::sleep` written inline adds no manifest edge and is
invisible to it. The first version scanned `src/lib.rs` for substrings, one of
them the bare word `park`.

**Command.** Run the suite with the guard in place.

```sh
cargo nextest run -p ring_handle --all-features
```

**Prediction.** It passes. The source contains no parking call. `ring_wait` is
not a dependency, and nothing in `src/lib.rs` blocks.

**Result (2026-08-28): red, and the prediction about the *source* was right.**

```
FAIL [0.019s] ring_handle::handle_test no_parking_shaped_name_appears_in_the_source
parking-shaped names in ring_handle's source: ["park"] — see docs/invariant/002
```

The match is in the module documentation: *"would put a **park**ing operation
within reach of the tick path"*. **The guard fired on the prose explaining what
the guard is for.**

**This is a known defect shape, a grep counting its own prose.** Knowing the
shape did not prevent it. What it bought was the time to diagnosis. The failure
was read correctly on sight rather than investigated as a real violation, which
is a smaller saving than not making the mistake and a real one.

**Fix.** Strip `//`-to-end-of-line before scanning, and narrow `park` to
`::park` and `park(`. Both, not either:

- Stripping comments alone leaves `park` matching a future identifier like
  `parked_count`, which is not a parking call.
- Narrowing alone leaves `thread::park` in a doc comment matching `::park`.

The scan now reads code and not documentation, which is what the invariant was
always about.

**What this cost, stated plainly.** The guard is weaker than the first version
looked. It cannot see a busy loop, a `Duration` behind a type alias, or a
blocking call reached through some dependency other than `ring_wait`. A guard
that also fires on its own documentation is only louder than one that does
not, and the noise is what makes people delete it.

---

## Run Record

| Date | Stages | Result |
|------|--------|--------|
| 2026-08-28 | H1–H5 | 5/5 run. Three predictions held; H4's was wrong in the two entries that mattered, and H5's held about the source while the *check* was wrong. Between them they changed five doc instances, the shape of one assertion, and the scan in `no_parking_shaped_name_appears_in_the_source` |

### Per-stage

| Stage | Ran | Prediction | Verdict |
|---|---|---|---|
| H1 | 2026-08-28 | Three `E0599` naming their own method, one trait-bound error on `Sync` | Held. The fifth case's stderr also records the two-crate inheritance chain, which is more than was asked of it |
| H2 | 2026-08-28 | The matching case fails when the method is added; green again when removed | Held. Recorded verbatim in `invariant/001` as the measurement behind "redder, not greener" |
| H3 | 2026-08-28 | `(&producer).clone()` compiles and yields a `Copy` reference | Held. The warning in `producer_clones.rs` matters and is correct |
| H4 | 2026-08-28 | Both handles are one pointer | **Wrong.** 24 and 16. Four instances corrected; the assertion rewritten as an equality rather than an absolute |
| H5 | 2026-08-28 | The source scan passes | **Red, for the right reason and the wrong match.** The scan hit its own documentation; comment-stripping and two narrower needles fixed it |

### What this round changed

- `docs/data_structure/001`: the shape table's handle-size column, and a measured table replacing it
- `docs/type/001` U6, `docs/type/002` C6: "one pointer" replaced with the size-equality property
- `docs/lifecycle/003`: its Tests row, which named the wrong assertion
- `docs/invariant/001`: H2's output recorded as the evidence for the redder-not-greener claim
- `docs/non_functional_requirement/001`: a sixth measurement step, because "the suite has never been shown to fail" was not on the list
- `tests/handle_test.rs`: the source scan rewritten to read code rather than comments, with H5's account in its doc comment so the next person to widen the needle list knows what it costs
