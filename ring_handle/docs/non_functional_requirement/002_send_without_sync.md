# Non-Functional Requirement: Send Without Sync

### Scope

- **Purpose**: State the second half of this crate's Reached condition — that both handles are `Send` and the pair moves to two threads without a shared mutable reference — and separate the three distinct properties it bundles.
- **Responsibility**: The attribute, the statement, the measurement, and the threshold.
- **In Scope**: `Send` on both handles; the absence of a shared `&mut`; why `Sync` is not required and should not be granted casually.
- **Out of Scope**: The capability split itself (→ [Proven by Code That Must Not Compile](001_proven_by_code_that_must_not_compile.md)); thread affinity of the underlying ring, which is `ring_core`'s.

### Quality Attribute

**Concurrency without shared mutability** — the ability to drive a ring from
two threads where neither holds a reference the other could alias.

### Statement

The second clause of
[the acceptance table](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md)'s
row for this crate's handle split:

> both are `Send`, and the pair can be moved to two threads without a shared
> mutable reference.

**Three claims, not one, and they are independent:**

| # | Claim | Independent because |
|---|-------|---------------------|
| N1 | `Producer: Send` | A type can be `Send` and still be useless across threads if its methods need `&mut self` on a value someone else owns |
| N2 | `Consumer: Send` | Same, independently |
| N3 | No shared mutable reference is needed | A pair could be `Send` and still require both ends to reach one `&mut Ring` — which is exactly the alternative this design rejects |

**N3 is the claim that carries the design.**
The rejected alternative is one shared mutable reference to the whole
ring, which has to be guarded to be shared at all — so the type that exists to
avoid a lock would end up behind one. N1 and N2 are necessary and would not be
sufficient; a `Send` handle that borrows from a ring living elsewhere still
forces that ring to be shared.

**`Sync` is conspicuously not in the statement, and that is not an omission.**
`Send` means the value may *move* to another thread. `Sync` would mean two
threads may hold `&Producer` *simultaneously* — which is a second producer by
another name, and reintroduces exactly the cardinality violation
[`ring_spsc`](../../../ring_spsc/docs/invariant/001_exactly_one_producer_one_consumer.md)
cannot survive. Whether `Producer` should be `Sync` at MPSC cardinality is a
real question; whether it should be `Sync` unconditionally is not.

### Measurement Method

1. **A static assertion, not a runtime one.** `Send` is a compile-time property
   and the test asserts it as such — a `fn assert_send<T: Send>() {}` invoked at
   both types. This fails to compile if the property is lost, which is the
   correct direction for a property this crate is supposed to guarantee.

2. **A real two-thread move for N3.** The pair is created on one thread, the
   `Producer` moved into a spawned thread by value, the `Consumer` kept — and
   items flow. A test that merely asserts `Send` demonstrates N1 and N2 and says
   nothing about N3.

3. **No `Arc`, no `Mutex`, no scoped borrow in the test.** This is the whole
   measurement. If the test needs `Arc<Mutex<_>>` to compile, the crate has
   failed N3 while satisfying N1 and N2, and the test would still pass on a
   naive reading.

4. **`Sync` is asserted absent where it must be absent**, rather than left
   unstated. If `Producer` is intended non-`Sync`, a compile-fail case pins it;
   if it is intended `Sync` under some configuration, the condition is stated.
   Leaving it unexamined is how a `Sync` impl arrives later by inference from an
   auto-trait.

5. **Miri or a thread sanitizer over measurement 2**, since a two-thread move
   that compiles proves ownership discipline and not memory safety of what the
   handles do once moved.

### Acceptance Threshold

| # | Criterion | Met when |
|---|-----------|----------|
| Q1 | `Producer: Send` | `both_handles_are_send` compiles — it also covers `Split` |
| Q2 | `Consumer: Send` | Same assertion |
| Q3 | The pair drives a ring from two threads | A spawn-and-move test moves items end to end |
| Q4 | Neither `Arc` nor `Mutex` nor any guard appears in Q3's test | Read off the test source — the criterion is the *absence*, so it is checked by reading, not by asserting |
| Q5 | `Sync` status is deliberate on both types | **Absent, pinned** by `tests/ui/producer_shared_across_threads.rs` |

**Q4 is the criterion that cannot be expressed as an assertion**, and that is
worth being explicit about rather than pretending otherwise. Every other
criterion in this crate's two requirements is machine-checked; this one is a
property of the test's own source text. A reviewer reading Q3's test and finding
a `Mutex` in it has found the failure; no assertion inside that test will.

**Q5 is the criterion most likely to be skipped**, because `Send`/`Sync` are
auto traits: a type gets them by having members that have them, without anyone
deciding. `Producer` becoming `Sync` because a field changed is a silent widening
of what the crate permits, and nothing about it looks like an edit to the
concurrency contract.

**Measured, and the answer is more interesting than the criterion asked for.**
Neither handle is `Sync`, and the reason is not in this crate or in `ring_core`:
`ring_spsc`'s handles carry a `PhantomData< Cell< () > >`, a deliberate opt-out
placed in the crate that owns the one-producer-one-consumer invariant, and the
property is inherited structurally through two crates. The pinned `.stderr`
records that whole chain — `Cell<()>` → `ring_spsc::Producer` →
`ring_core::ProducerInner` → `ring_handle::Producer` — so a reader who asks
"why can't I share this" gets the answer and the owning crate from the test
output.

**This is the auto-trait exposure stated concretely rather than generically.**
The widening Q5 fears would not happen here by an edit to `ring_handle` at all;
it would happen when a backend crate drops a marker for its own reasons, with
nothing in this crate's diff to notice. That is precisely why the mechanism has
to be a compile-fail case rather than a review habit: the edit that breaks it
is not in the file anyone would be reviewing.

**And "not `Sync`" cannot be a static assertion.** Rust has no negative trait
bound, so there is no `assert_not_sync::< T >()` to write. A program that must
not compile is the only available shape — the same structural reason this
crate's own Reached criterion takes that shape (→
[Proven by Code That Must Not Compile](001_proven_by_code_that_must_not_compile.md)).

**This threshold asserts nothing about performance.** Two threads exchanging
items proves the ownership model; whether the ring is fast is
[`ring_bench`](../../../ring_bench/readme.md)'s
question over its eight measured configurations.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) | What each handle holds, which is what determines N1, N2 and Q5 |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | Why `Sync` on `Producer` would be its V3 by another route |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_split_move_and_drop.md](../lifecycle/001_split_move_and_drop.md) | The move Q3 exercises, as a lifecycle phase |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_proven_by_code_that_must_not_compile.md](001_proven_by_code_that_must_not_compile.md) | The other half of this crate's row; both must be met for it to be Reached |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_handle_ownership.md](../lifecycle/003_handle_ownership.md) | The ownership states Q3 moves through |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_producer.md](../type/001_producer.md) | Q1 and Q5's subject |
| [../type/002_consumer.md](../type/002_consumer.md) | Q2's subject |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's row — the verbatim second clause |

### Tests

| File | Relationship |
|------|--------------|
| `tests/handle_test.rs` | `both_handles_are_send` — Q1 and Q2 as static assertions, plus `Split` |
| `tests/handle_test.rs` | `the_two_ends_travel_to_separate_threads` — Q3 as a spawn-and-move exchange, and Q4 by containing no guard type |
| `tests/ui/producer_shared_across_threads.rs` | Q5 — pins `!Sync`, which no static assertion can express |

### HD35 — The Pair Cannot Move to a `'static` Thread, and Q3 Passes by Using Scoped Ones

The clause is "the pair can be moved to two threads." Look at how the test does
it, and at why it has no choice:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- how every thread in this crates tests is started --'
command grep -E 'thread::scope|thread::spawn|scope\.spawn' \
  ring_handle/tests/handle_test.rs | sed 's|^|    |'
echo '  -- and the lifetimes that force it --'
command grep -E '^pub struct (Producer|Consumer|Split)' ring_handle/src/lib.rs \
  | sed 's|^|    |'
```

Live output:

```
  -- how every thread in this crates tests is started --
      let received = std::thread::scope
        scope.spawn( move ||
        scope.spawn( move ||
  -- and the lifetimes that force it --
    pub struct Split< T >
    pub struct Producer< 'a, T >
    pub struct Consumer< 'a, T >
```

`Producer< 'a, T >` and `Consumer< 'a, T >` both borrow from the `Split`. Neither
is `'static`, so `std::thread::spawn` cannot take them — the test uses
`std::thread::scope`, and it is the only shape available. Zero `thread::spawn`
calls in the crate.

**Q3 is met in the scoped sense and the statement reads in the `'static`
sense.** The difference is real for a caller: a scoped thread must join before
the borrow ends, so a `Producer` cannot be handed to a long-lived worker thread,
stored in a thread pool, or moved into a `'static` closure. Every one of those
is what "moved to another thread" ordinarily means in a system that spawns its
workers once.

`Split< T >` has no lifetime and is the value that does travel — which is why
[`data_structure/001`](../data_structure/001_two_handles_over_one_backend.md)'s
HD12 finds every consumer naming `Split` and none naming a handle. N3 is
satisfied by the pair being borrowed from one owner rather than sharing a `&mut`,
and that is a different and weaker property than the pair being independently
movable, which is what the clause's wording suggests.

### HD36 — The One Two-Thread Test Cannot Fail, Only Hang, and Measurement 5 Is Not Run

Measurement 5 asks for Miri or a sanitizer over the two-thread test, arguing
that a move which compiles proves ownership and not memory safety. Check whether
it runs, and what happens when the property it guards actually breaks:

```sh
cd "$(git rev-parse --show-toplevel)"
# scoped to where a runner would live — a docs-wide grep would match this finding
printf '  miri in this crates src/tests/manifest: %s\n' \
  "$( command grep -rli 'miri' ring_handle/src ring_handle/tests \
       ring_handle/Cargo.toml 2>/dev/null | wc -l )"
# excludes gate/-target_gate (a cargo build dir) and this gate's own hyphen-
# prefixed session logs, both of which can quote 'miri' without being a script
printf '  miri in the gate scripts:               %s\n' \
  "$( command grep -rli 'miri' bench_harness/gate 2>/dev/null \
       | command grep -v '^bench_harness/gate/-' | wc -l )"
echo '  -- the loops in the one two-thread test --'
sed -n '/^fn the_two_ends_travel_to_separate_threads/,/^}/p' \
  ring_handle/tests/handle_test.rs \
  | command grep -E 'while |assert' | sed 's|^|    |'
```

Live output:

```
  miri in this crates src/tests/manifest: 0
  miri in the gate scripts:               0
  -- the loops in the one two-thread test --
          while next < 32
          while got.len() < 32
      assert_eq!( received, ( 0..32 ).collect::< Vec< _ > >(), "in order, and all of it" );
```

Zero mentions of Miri in this crate's source, tests or manifest, and zero in the
family's gate scripts. Both threads spin on unbounded `while` loops —
the producer until it has pushed 32, the consumer until it has received 32 — and
the single assertion runs only after both terminate.

**So the test has two outcomes, not three: pass, or run forever.** If a record
is lost or a store never becomes visible to the draining thread, `got.len()`
stops advancing and the consumer spins until the harness times out. A timeout is
not a red assertion: it names no property, points at no line, and on a busy CI
machine is indistinguishable from a slow one.

Two things make this sharper than the usual objection to a spin loop. The host
is `aarch64` and weakly ordered, which is the configuration where a missing
fence *is* the failure this test is supposed to catch. And this crate's own
[`invariant/002`](../invariant/002_no_parking_operation_is_reachable.md) forbids
parking on the tick path — the busy-wait here is the shape that rule pushes
callers toward, so its cost showing up in the test suite is not incidental.

Measurement 5 is the answer already written down: under Miri the two-thread
interleaving is explored rather than raced, and a lost store is reported as a
data race rather than a hang. It has a measurement, no criterion in Q1–Q5, and
no runner.
