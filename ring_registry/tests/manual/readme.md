# Manual testing for ring_registry

Six stages. Each states a **prediction before it is run**, so a wrong
prediction is recorded as a finding rather than quietly corrected into agreement
with the result.

Two stages carry the weight. M1 measures the claim that
[`docs/pitfall/001`](../../docs/pitfall/001_insert_would_have_replaced_silently.md)
is built on, rather than leaving it as reasoning. M6 is where the prediction was
wrong. It put a measured byte count on the crate's one argued-over signature
decision. The rest cover things the automated suite structurally cannot.

Run from the repository root.

---

## M1: would an `insert`-based registry really lose the ring?

The whole crate is shaped around the claim that `HashMap::insert` with a
discarded return value silently destroys the ring already registered. This stage
tests it with a throwaway probe (`tests/zz_probe_insert.rs`, run and deleted)
rather than arguing it:

```rust
let mut map : HashMap< String, Split< Counted > > = HashMap::new();
map.insert( "events".to_string(), first );        // 4 records inside
map.insert( "events".to_string(), ring( 16 ) );   // return value discarded
```

**Prediction:** 4 drops, `len() == 1`, `contains( "events" ) == true`. Every
observable correct, four records gone.

**Result (2026-08-28): exactly as predicted.**

```
after registering 4 records: drops=0 len=1
after a second insert:       drops=4 len=1
contains(events)=true
```

The pitfall is measured, not inferred. Note what a test would have to look at to
notice: not `len`, not `contains`, not any retrieval, only the drop counter.

---

## M2: is a `Registry< T >` movable between threads?

```rust
fn assert_send< T : Send >() {}
assert_send::< ring_registry::Registry< u32 > >();
```

**Prediction:** yes, for `T : Send`. `HashMap` is `Send` when its value is, and
`Split< T >` is `Send` when `Ring< T >` is.

**Result (2026-08-28):** as predicted, it compiles.

---

## M3: what happens with a record type that is not `Send`?

```rust
let r : Registry< Rc< u32 > > = Registry::new();
```

**Prediction:** it compiles. `Registry< T >` carries no bound, since the registry
never touches `T`, so the type exists. But `ring_core::Ring::new` requires
`T : Send`, so nothing can ever be registered in it.

**Result (2026-08-28):** as predicted, constructed with `len() == 0`.

**The finding is a deliberate choice rather than an oversight.** A
`Registry< Rc< u32 > >` is a type that compiles and can never hold anything.
Adding `T : Send` to the struct would make it a compile error instead, which is
arguably friendlier. But it would put a bound on a type that does not need one,
only to mirror a requirement that belongs to `ring_core`. If `ring_core` ever
gained a non-`Send` ring, the bound would then be wrong and would have to be
removed. The struct stays unbounded, and this stage records that so the
emptiness is not mistaken for a bug.

---

## M4: are all declared dependencies used?

```bash
cargo +nightly udeps -p ring_registry --all-targets --all-features
```

**Prediction:** clean. `ring_core` and `ring_types` were moved out of
`[dependencies]` when the implementation showed the record type is opaque here
(→ [`docs/integration/001`](../../docs/integration/001_one_declared_edge_of_three.md));
`ring_core` and `ring_config` remain as dev-dependencies, both used by the tests.

**Result (2026-08-28):** `All deps seem to have been used.`

---

## M5: is the crate fully covered?

```bash
cargo tarpaulin -p ring_registry --all-features --out Stdout 2>&1 \
  | grep 'ring_registry/src'
```

**Prediction:** 100%. Every operation has a named test and the only branch is the
`Entry` match, both arms of which are exercised.

**Result (2026-08-28):** `28/28`. Reached on the first measurement, without a
coverage-driven test added afterwards.

---

## M6: is the crate clippy-clean under `-D warnings`?

```bash
cargo clippy -p ring_registry --all-targets --all-features    -- -D warnings
cargo clippy -p ring_registry --all-targets --no-default-features -- -D warnings
```

**Prediction:** clean both ways. The implementation is a `HashMap` with one
`Entry` match; there is nothing in it for a lint to object to.

**Result (2026-08-28): wrong. The build failed, both ways, on the same lint.**

```
error: the `Err`-variant returned from this function is very large
   --> ring_registry/src/lib.rs:128:6
128 |   -> Result< (), ( RegistryError, Split< T > ) >
    |      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the `Err`-variant is at least 448 bytes
```

**This is the most useful stage in the plan despite being a lint.** The
prediction was wrong because it was made about the *implementation*, and the lint
is about the *signature*. The signature is the one part of this crate that was
argued at length before it was written
(→ [`decisions/readme.md`](../../docs/decisions/readme.md) Closed 2). The lint
did not find a mistake; it put a number on a cost the decision had accepted
without one. 448 bytes, on the `Ok` path as well as the `Err` path.

Two remedies were available and both were refused, because each undoes Closed 2:
dropping the payload destroys the caller's ring, and boxing it allocates on the
failure path to narrow a `Result` whose bytes `register` already takes by value
on the way in. The suppression is on `register` alone, with a `reason =`, so the
next oversized `Result` still fails the build.

The same run had a secondary finding, unrelated and fixed rather than
suppressed. Four `producer.try_push( .. ).ok().expect( .. )` calls in the test
file tripped `clippy::ok_expect`. They were written that way to dodge a `Debug`
bound and are now `assert!( .. .is_ok(), "within capacity" )`, which dodges it
without the lint.

**After the fix:** clean both ways.

---

## Run Record

| Stage | Question | 2026-08-28 |
|---|---|---|
| M1 | `insert` loses the ring | ✅ 4 drops, every observable still correct |
| M2 | `Registry< T >` is `Send` | ✅ compiles |
| M3 | Non-`Send` `T` | ✅ compiles, unusable by design |
| M4 | No unused dependencies | ✅ clean |
| M5 | Fully covered | ✅ 28/28 |
| M6 | Clippy-clean under `-D warnings` | ❌ **prediction wrong**: `result_large_err`, 448 bytes; allowed with a reason, plus 4 `ok_expect` fixed |

**Predictions wrong: 1 of 6**, and it is M6. The five that were right cover a
`HashMap` with one refusal, which is what a crate this small should produce. The
two that earned their run are M1, which turns the pitfall from an argument into a
measurement, and M6, which turns the crate's one deliberate signature decision
from a qualitative trade into a number.

**Environment:** `2026-08-28`, Linux 6.8.0, `cargo nextest`, `cargo tarpaulin`,
`cargo +nightly udeps`, `cargo clippy`. 14 tests + 1 doc test, all passing.
