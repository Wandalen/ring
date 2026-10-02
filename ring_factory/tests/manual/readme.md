# ring_factory manual testing

Each stage states a **prediction before it is run**, so a wrong prediction is
recorded as a finding rather than quietly corrected into agreement with the
result.

Two stages matter most, and both predictions were wrong. F1 found that a
guarantee the test file appeared to assert cannot be measured. F2 found that the
window in which the suite's main helper discriminates anything is bounded on
**both** sides.

Run from the repository root.

---

## F1. Where does the refused ring die?

A `NameTaken` build must not have constructed a ring, or must have dropped it
before returning, and it must leave the ring already registered untouched.
`a_refusal_drops_nothing_that_was_already_registered` counts drops around the
refused call. Probed directly (`tests/zz_probe_drop.rs`, run and deleted):

```rust
Factory.build_named( cfg, "events", &mut registry ).expect( "first" );
// … fill the registered ring with 4 drop-counting records …
let before = DROPS.load( SeqCst );
let again = Factory.build_named( cfg, "events", &mut registry );  // refused
let after = DROPS.load( SeqCst );
```

**Prediction:** the refused ring's destruction shows up as a nonzero delta.

**Result (2026-08-28): prediction wrong. The delta is zero, and it is zero for a
reason no amount of counting can fix.**

```
after filling the first ring:   drops=0
refusal returned:               Err(NameTaken)
drops across the refused call:  0
registry len after refusal:     1
records drained from survivor:  4
```

**A freshly built ring is empty.** It holds no records, so its destruction drops
nothing a record counter can see. A *leaked* ring would produce the same
zero. The counter cannot distinguish the two states it was installed to
distinguish.

The zero *does* prove the other clause. The refusal did not touch the ring
already registered, whose four records are still alive and still drain in
order. That clause can fail. A registry that swapped on collision, or dropped the
incumbent before refusing, breaks it while passing every `len()` check.

So the test is named for the clause it checks, and its doc comment says which
clause is checked and which is structural. `_refused` is an ordinary binding
that goes out of scope with no `mem::forget` on the path, so ownership
guarantees the destruction, not this suite.

**The name matters.** A name is what a reader sees in a failure report and in a
coverage summary. A test that asserts less than its name says is a
documentation defect that no test run can catch.

---

## F2. How narrow is the window where `observable_profile` discriminates?

`only_two_of_five_config_fields_are_observable_through_the_factory` compares
profiles built from configs differing in one field, from a 4-slot base. The
helper refuses any ring of 8 slots or more with a `capacity < 8` guard, because
its *negative* assertions pass vacuously once the ring stops overflowing.

Probed across three capacities (`tests/zz_probe_vacuity.rs`, run and deleted),
offering 8 records each time:

**Prediction:** the vacuity is one-sided. Below 8 slots the ring overflows and
the profile discriminates; at 8 and above it saturates and stops discriminating.

**Result (2026-08-28): prediction wrong. It is bounded on both sides, and the
two bounds fail differently.**

```
  4 slots  fail=(4, 4, 4)   dropnewest=(4, 8, 4)   p1=(4, 8, 4)   p4=(4, 8, 4)
  8 slots  fail=(8, 8, 8)   dropnewest=(8, 8, 8)   p1=(8, 8, 8)   p4=(8, 8, 8)
 16 slots  fail=(16, 8, 8)  dropnewest=(16, 8, 8)  p1=(16, 8, 8)  p4=(16, 8, 8)
```

| Capacity | Overflow assertion | Capacity assertion | Why |
|---|---|---|---|
| 4 | ✅ discriminates: `(4,4,4)` vs `(4,8,4)` | ✅ | the ring overflows |
| 8 | ❌ vacuous | ❌ vacuous | exactly saturates: every profile is `(8,8,8)` |
| 16 | ❌ vacuous | ✅ works | never fills: 8 offered into 16 slots |

**At 8 slots every one of the six profiles is byte-identical**, so all four
assertions in the test pass whatever the factory does. At 16 the capacity
assertion recovers and the overflow one goes vacuous instead. That is the same
defect arriving from the opposite direction, and it would *not* have announced
itself, because no positive assertion fails at 16.

**The guard covers both walls.** `capacity < 8` excludes both cases, and its
message, "a profile of a N-slot ring never overflows, so it discriminates
nothing", is accurate for both. The test's own comment records the measurement
so that anyone loosening the bound knows there are two walls, not one.

**In general, a helper that returns a tuple of counts is only as sharp as the
ratio between what it offers and what the ring holds**, and that ratio is
invisible at both call sites. The helper takes a config, and the caller passes a
capacity. Nothing in either signature says the two must be related.

---

## F3. Do the two doors diverge exactly where the policy says?

`build` refuses `OverflowPolicy::DropOldest` and `build_crossbeam` accepts it,
because `ArrayQueue::force_push` evicts. Probed under `--all-features`:

**Prediction:** `build` returns `Unsupported( PolicyUnsupported )`;
`build_crossbeam` accepts all 8 records into a 4-slot ring and the survivors are
the **last** four, `[4, 5, 6, 7]`, since the oldest are the ones evicted.

**Result (2026-08-28): exactly as predicted.**

```
build( DropOldest )           = Some(Unsupported(PolicyUnsupported))
build_crossbeam accepted      = 8 of 8
records that survived         = [4, 5, 6, 7]
```

The two doors diverge on the overflow policy and agree everywhere else.
It was worth measuring rather than reasoning. The claim "the crossbeam backend
accepts `DropOldest`" is about acceptance, and eviction *order* is a separate
claim that nothing in the type system connects to it. A backend that accepted
the policy and then evicted the newest would satisfy every assertion in the
automated suite.

---

## F4. Are all declared dependencies used?

Each manifest edge is there because a signature names one of its types or
`build` calls it, so an edge with neither is dead.

```bash
cargo +nightly udeps -p ring_factory --all-targets --all-features
```

**Prediction:** clean. All four normal dependencies appear in a public
signature: `RingConfig` and `Registry` as re-exports, `Split` as `build`'s
return type, `RingError` as `BuildError::Unsupported`'s payload. `ring_core` is
called directly.

**Result (2026-08-28): as predicted.** `All deps seem to have been used.`

---

## F5. Is the crate fully covered?

```bash
cargo tarpaulin -p ring_factory --all-features --out Stdout
```

**Prediction:** 100%. The crate's functions are a few lines each, and every
error arm has a test.

**Result (2026-08-28): as predicted, 100% on first measurement.** It needed no
coverage-chasing pass, because nearly all of its difficulty is in what it
*refuses* and what it *cannot express*, and both live in tests and documentation
rather than in branches.

---

## F6. Clippy-clean under `-D warnings`, in every feature configuration?

The `crossbeam` feature gates a whole function, so a lint could hide in either
configuration.

```bash
cargo clippy -p ring_factory --all-targets --all-features    -- -D warnings
cargo clippy -p ring_factory --all-targets                   -- -D warnings
cargo clippy -p ring_factory --all-targets --no-default-features -- -D warnings
```

**Prediction:** clean in all three. `BuildError` is `Copy` and two words wide, so
`result_large_err` cannot fire here.

**Result (2026-08-28): as predicted, all three clean.** The `Copy` bound that
forced `BuildError::NameTaken` to discard `RegistryError`'s payload is the same
property that keeps this stage quiet. One decision paid for twice.

---

## Run Record

| Stage | Question | 2026-08-28 |
|---|---|---|
| F1 | Where does the refused ring die | ❌ **prediction wrong.** Unmeasurable; test renamed and its doc comment split into checked vs structural |
| F2 | How narrow is the discriminating window | ❌ **prediction wrong.** Bounded on both sides, not one; 16 slots vacuous for the opposite reason to 8 |
| F3 | Do the two doors diverge where the ADR says | ✅ `Unsupported`, then 8 of 8 accepted, survivors `[4,5,6,7]` |
| F4 | No unused dependencies | ✅ clean, after two removals and two additions |
| F5 | Fully covered | ✅ 15/15 on first measurement |
| F6 | Clippy-clean in three configurations | ✅ clean in all three |

**Predictions wrong: 2 of 6**, and both are about the automated suite rather
than about the crate. That is the pattern of this stage. The implementation was
short and correct almost immediately, and the effort went into establishing which
of its claimed guarantees a test can see. Two could not: V2's first clause and
three of feature 180's five fields. In both cases the honest outcome was to say
so in the test rather than to write an assertion that passes for the wrong
reason.

**F1 and F2 share a shape.** Both are assertions that pass whether or not the
code is correct. F1's counter reads zero for a dropped ring and for a leaked
one. F2's comparison reads equal for a factory that honours a field and for one
that ignores it. Running the suite detects neither, since a green test is green
either way. Both were found only by deliberately probing the measurement
instead of the thing measured. **A suite cannot check that its own
assertions can fail**, which is the argument for this stage existing at all.

**Environment:** `2026-08-28`, Linux 6.8.0, `cargo test`, `cargo tarpaulin`,
`cargo +nightly udeps`, `cargo clippy`. 19 tests + 3 doc tests, all passing.
