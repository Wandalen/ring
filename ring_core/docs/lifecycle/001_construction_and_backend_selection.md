# Lifecycle: Construction and Backend Selection

### Scope

- **Purpose**: Specify how a backend is chosen, when the choice becomes irrevocable, and why one backend is selected by a config field while another is selected by a different constructor entirely.
- **Responsibility**: The two constructors, the discriminator each uses, the one construction-time rejection, and the consequences of the choice being permanent.
- **In Scope**: `Ring::new`, `Ring::new_crossbeam`, `Ring::overflow`.
- **Out of Scope**: What happens after construction (→ [`lifecycle/002`](002_ends_split_and_handle_lifetimes.md)); the enum being constructed (→ [`data_structure/001`](../data_structure/001_three_way_storage_enum.md)).

### Lifecycle Phases

Construction is three phases deep and all of them complete before the ring is
handed back — there is no partially-built state a caller can observe:

| # | Phase | What happens | Can refuse |
|---|-------|--------------|:---:|
| 1 | **Discrimination** | The entry point and the config decide which `Storage` variant will exist | no |
| 2 | **Validation** | The requested `OverflowPolicy` is checked against what that variant can honour | ✅ `RingError::PolicyUnsupported` |
| 3 | **Construction** | The backend is allocated at its fixed capacity and the policy is stored beside it | no |

#### Two constructors, two kinds of discriminator

| Constructor | Selects | Discriminated by | Returns |
|---|---|---|---|
| `new( &config )` | `Spsc` or `Mpsc` | `RingConfig::is_multi_producer()` — a **workload** property | `Result`, and it can fail |
| `new_crossbeam( &config )` | `Crossbeam` | the call itself — a **build** decision | `Result`, but never `Err` today |

**`new_crossbeam` returns a `Result` it never uses**, and that is on purpose: the
two constructors have identical signatures, so swapping one call for the other
is a one-word edit at the call site rather than a change to the surrounding
error handling. Narrowing it to `Self` would make the interchangeability the
feature exists for cost more than it saves.

**The asymmetry is deliberate and is the design decision of this instance.**
Producer cardinality is a property of the caller's workload, so it belongs in
the config a caller already fills in. Whether the crossbeam backend exists at
all is a property of the *build* — it is behind a cargo feature — and a config
field cannot express that: setting `backend = Crossbeam` in a build without the
feature would have to fail at runtime, for a mistake the compiler could catch.

A separate constructor makes the mistake a compile error instead. In a build
without `crossbeam`, `new_crossbeam` does not exist, and code calling it does
not build.

#### The one rejection

`OverflowPolicy::DropOldest` is refused by `new` with
`RingError::PolicyUnsupported`.

Eviction contradicts exactly-once delivery, which both in-house rings guarantee
— dropping an unread record to make room means a record was accepted and never
delivered. Rather than silently degrading to `DropNewest`, the constructor
fails: **a caller who asked for eviction and got refusal-of-new-records instead
would be running a different program than the one they wrote**, with no signal.

`new_crossbeam` accepts it, because `ArrayQueue::force_push` implements exactly
that semantics. So the same policy value is valid or invalid depending on which
constructor is called — which is another reason the two are separate entry
points rather than one with a backend field.

### Phase Transitions

**The transition out of phase 3 is one-way.** There is no `set_backend`, no
rebuild, no runtime swap. Once `Storage` holds a
variant it holds it for the ring's life.

| Consequence | |
|---|---|
| Dispatch can be a `match` on a fixed variant | no vtable on the path (→ [`algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md)) |
| The capacity is the backend's, fixed at construction | no growth, no reallocation |
| The overflow policy is stored once beside the storage | never re-read from a config |
| Swapping backends means constructing a new ring | which is what this crate means by "a build flag rather than a rewrite" — the *program* is unchanged, the construction line is not |

`overflow()` reports the policy the ring was built with, so a caller who was
handed a ring rather than building one can still discover what a refusal will
do. Asserted by `a_ring_reports_the_overflow_policy_it_was_built_with`, because
a reporting method that reports a default rather than the stored value is a
plausible bug that nothing else would catch.

### Dependencies

Construction is the one place this crate reads another crate's *decision* rather
than calling its behaviour:

| Needed from | For which phase | What would happen without it |
|---|---|---|
| `ring_config::RingConfig` | 1 — `is_multi_producer()` is the SPSC/MPSC discriminator | The cardinality would have to be a constructor argument, duplicating a field the caller already fills in |
| `ring_types::RingError` | 2 — the refusal's vocabulary | A second family error type for one condition |
| `ring_spsc`, `ring_mpsc`, `crossbeam-queue` | 3 — the thing actually allocated | — |

Nothing else is consulted. In particular the overflow resolution
(`ring_overflow`) is not involved at construction — it runs per push, long after
the policy has been stored.

### Cleanup Requirements

**Construction has no teardown of its own.** A refused `new` allocates nothing:
phase 2 runs before phase 3, so a `PolicyUnsupported` costs a comparison and
returns. There is no partially-constructed ring to unwind and no drop order to
get right.

The ring's own drop is the only cleanup obligation, and it is the backend's:
records still in storage are dropped exactly once — the in-house rings' `Drop`
for two arms, upstream's for the third
(→ [`002_ends_split_and_handle_lifetimes.md`](002_ends_split_and_handle_lifetimes.md)'s
drop obligation, where the composition risk actually lives).

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_three_way_storage_enum.md](../data_structure/001_three_way_storage_enum.md) | The variant this procedure selects |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_ends_split_and_handle_lifetimes.md](002_ends_split_and_handle_lifetimes.md) | What the constructed ring is then used for |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_backend.md](../type/001_backend.md) | The public name for the choice made here |
| [../type/002_producer_cardinality.md](../type/002_producer_cardinality.md) | `is_multi_producer()`, the discriminator |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_backend_swap_is_a_build_flag.md](../non_functional_requirement/001_backend_swap_is_a_build_flag.md) | The requirement the permanence table's last row is measured against |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `the_producer_count_selects_between_spsc_and_mpsc` — `new`'s discriminator |
| `tests/core_test.rs` | `the_crossbeam_backend_ignores_the_producer_count` — that `new_crossbeam` does not consult it |
| `tests/core_test.rs` | `drop_oldest_is_rejected_by_the_in_house_backends`, `crossbeam_honours_drop_oldest_by_evicting` — both sides of the rejection |
| `tests/core_test.rs` | `a_ring_reports_the_overflow_policy_it_was_built_with` — `overflow()` |

### CO33 — One Policy Is Rejected at Construction and Two Backends Support It

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
grep 'PolicyUnsupported' src/lib.rs
grep -oE '^fn [a-z_]+' tests/core_test.rs | sed 's/^fn //' | grep 'drop_oldest\|crossbeam'
```

Live output:

```
    /// [`RingError::PolicyUnsupported`] for `OverflowPolicy::DropOldest`, which neither
    /// assert_eq!(Ring::<u8>::new(&evicting).unwrap_err(), RingError::PolicyUnsupported);
            return Err(RingError::PolicyUnsupported);
the_crossbeam_backend_ignores_the_producer_count
drop_oldest_is_rejected_by_the_in_house_backends
crossbeam_honours_drop_oldest_by_evicting
```

`drop_oldest_is_rejected_by_the_in_house_backends` and
`crossbeam_honours_drop_oldest_by_evicting` are the two halves, and together
they say something the lifecycle prose does not: **a `RingConfig` is not
portable across backends.** A config carrying `DropOldest` constructs a ring
under `--features crossbeam` and returns `Err( PolicyUnsupported )` without it.

`non_functional_requirement/001` states that swapping backends is a build flag
and no code change. That is true for every configuration except this one.

### CO34 — Selection Uses `match` on a Boolean for Coverage Reasons

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
command grep -m1 -B2 -A8 -F '    // `match` rather than `if`/`else`: under the family'"'"'s brace style the `else`' src/lib.rs
```

Live output:

```
        }

        // `match` rather than `if`/`else`: under the family's brace style the `else`
        // keyword lands on a line of its own, where `llvm-cov` opens a region that
        // nothing can ever execute — so the crate reads 120/121 with both arms
        // demonstrably covered. `tests/manual/readme.md` C4 records the measurement.
        let storage = match config.is_multi_producer() {
            true => Storage::Mpsc(ring_mpsc::Ring::with_config(config)),
            false => Storage::Spsc(ring_spsc::Ring::with_config(config)),
        };
```

The comment names the reason: `llvm-cov` attributes an `if`/`else` on a boolean
differently from a two-arm `match`, and the `match` form produces the region
counts the family's coverage gate expects.

**A measurement tool shaping source form is worth recording as such.** It is a
defensible trade — the two forms are semantically identical — but it means this
line's shape is owned by the coverage configuration, and a future reader
simplifying it to an `if` would silently change a coverage reading rather than
break a test.
