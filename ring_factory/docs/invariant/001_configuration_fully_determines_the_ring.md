# Invariant: The Configuration Fully Determines the Ring

### Scope

- **Purpose**: State the property the benchmark sweep rests on — two rings built from equal `RingConfig` values must be indistinguishable — and name every input that could break it.
- **Responsibility**: State the invariant, its enforcement, and what breaks when it does not hold.
- **In Scope**: What `build` may read; determinism of the resulting ring's observable behaviour.
- **Out of Scope**: Whether the config matches what the caller *requested*, which is a separate and weaker claim (→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)); the ring's runtime behaviour once records flow.

### Invariant Statement

**`build` is a function of its argument and nothing else.**

> For any two `RingConfig` values `a` and `b`, if `a == b` then the rings
> produced by `build( a )` and `build( b )` are observably indistinguishable.

`RingConfig` derives `PartialEq`, so `a == b` is a real, checkable relation and
not an informal one:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'derive' ring_config/src/lib.rs | head -1
```

Live output:

```
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
```

**This is stronger than "the fields are honoured" and weaker than
"construction is pure."** It permits `build` to allocate, to consult the
allocator, to produce rings at different addresses — those are not observable
through the handle pair. It forbids `build` from reading anything the caller
did not put in the record.

**The banned inputs, stated positively, are the point:**

| Input | Why it would break the invariant |
|-------|----------------------------------|
| An environment variable | Two identical sweep points on two machines produce different rings; the comparison is between machines, not configurations |
| A global or `static mut` default | The Nth build differs from the first. Test order becomes load-bearing |
| A compile-time `cfg` this crate reads itself | The invariant holds per-binary and the benchmark compares binaries |
| Wall-clock time, a random seed, thread identity | The same sweep point is not reproducible across runs |
| A previously registered name (→ [`api/002`](../api/002_the_named_build_surface.md)) | Registration may *refuse*; it must not silently produce a different ring |

**The last row is the one this crate can actually get wrong**, because
`ring_registry` is a declared dependency and the named build path genuinely
does consult shared state. The invariant survives only if that consultation is
allowed to fail the whole call and never to alter the ring it returns.

#### The compile-time row arrived, one crate down

The third row was written as a hypothetical. It is now real, in `ring_core`
rather than here:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^\[features\]/,/^$/p' ring_core/Cargo.toml
```

Live output:

```
[features]
default = []
# Feature 187: an optional third backend, so consumers needing a working
# multi-producer channel are not blocked on the in-house ring being finished.
crossbeam = [ "dep:crossbeam-queue" ]
```

**This sharpens the invariant rather than breaking it, and the distinction is
worth being exact about.** The row bans a `cfg` that changes *what `build`
returns for a given record*. The optional crossbeam backend does not do that: it
is reached by a separate constructor, `ring_core::Ring::new_crossbeam`, never by
`Ring::new` inspecting a feature flag (→ [`algorithm/001`](../algorithm/001_selecting_a_backend_from_one_boolean.md)).
Enabling the feature adds a constructor; it does not redirect the existing one.

| Shape | Breaks the invariant | Present here |
|-------|---------------------|--------------|
| `#[cfg(feature)]` inside `new`, choosing a different backend for the same config | **Yes** — same record, different ring, per build | No |
| `#[cfg(feature)]` gating an additional constructor | No — the record's meaning is unchanged | Yes |

**The hazard is that the second shape becomes the first by a small edit.** A
single `#[ cfg( feature = "crossbeam" ) ]` arm added to `new`'s `match` would
satisfy every test in the family and violate this invariant silently, because
the tests all run under one feature set. **That is the concrete thing to watch
for, and it is not detectable from this crate** — the guard has to live where
the `match` is.

### Enforcement Mechanism

**The strongest layer is structural, not discipline — the crate has nowhere
to read a second input from** (§ FC21 below: zero occurrences of `static`,
`thread_local!`, `Atomic`, `Mutex`, `RefCell`, `Cell<`, `unsafe`, or `lazy` in
the source, and a fieldless `Factory` receiver). What discipline still has to
cover is only the part that absence of storage cannot reach:

| Layer | What it enforces | What it misses |
|-------|------------------|----------------|
| No `static`/interior mutability/`unsafe` anywhere in the crate, and a fieldless `Factory` | There is nowhere in this crate to store or read a second input from, structurally | A global one crate down, in `Ring::new` itself |
| `build( cfg )` taking `cfg` by value | Nothing may be read from a caller-held mutable location during the call | Anything read from a `static`, the environment, or the clock |
| `RingConfig: Copy` | The record cannot be mutated behind the call's back mid-build | Everything else |
| This crate's own stated design intent — "the only constructor input" | States the intent | Is a documented intent, not a check |
| The test named below | Builds twice from one config and compares behaviour | Two builds in one process, one after the other — catches a counter, misses an environment variable |

**The honest summary is that the strongest available check is a same-process
double build**, and it is worth writing anyway because it catches the one
failure a factory is actually likely to grow: an incrementing identity, a
one-time initialisation, a cached first result. It does not catch a `cfg!` or
an env read, and no test in this crate can.

**Determinism across processes is checkable only where the ring's behaviour is
being measured**, which is `ring_bench`'s ground. Two runs of the same sweep
point producing materially different numbers is the observable form of a
violation here, and it will be read as measurement noise unless somebody has
this invariant in mind.

### Violation Consequences

| # | Violation | Consequence |
|---|-----------|-------------|
| V1 | `build` consults an environment variable for a default | The sweep's numbers are machine-specific and nothing says so. Two engineers compare results and disagree with no visible cause |
| V2 | A `static` counter influences the ring | The Nth build differs from the first. A test that passes alone fails in a suite, and the suspicion falls on the test |
| V3 | The named path returns a ring shaped by what was already registered | Two identical configs registered under different names build differently. **The registry becomes a hidden second input** |
| V4 | `build` caches and returns a previously built ring for an equal config | The invariant holds *too well* — the rings are not merely indistinguishable, they are the same object, and two sweep points share a buffer |
| V5 | A future field is added to `RingConfig` and `build` ignores it | The invariant still holds; the *criterion* (→ [`non_functional_requirement/001`](../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md)) is what fails, and only if somebody adds the assertion |

**V3 is the realistic one.** `ring_registry` is declared, the named build path
is specified, and the natural implementation — look up, then construct — is one
refactor away from the unnatural one where the lookup result reaches the
construction. The [`lifecycle/004`](../lifecycle/004_name_state_through_a_registration.md)
instance is where that ordering is pinned.

**V4 is worth stating because it looks like an optimisation.** Caching by
config is a plausible thing to want in a factory and is catastrophic here: the
benchmark's entire purpose is that each sweep point exercises its own ring.
Two points sharing a ring produce contention that belongs to neither.

**V1 and V2 are cheap to avoid and expensive to diagnose.** Neither produces
an error; both produce numbers that are wrong in a direction nobody predicts.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | The signature that makes the invariant statable — one argument, by value |
| [../api/002_the_named_build_surface.md](../api/002_the_named_build_surface.md) | V3's surface; the second input that must not become one |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_configuration_record_as_input.md](../data_structure/001_the_configuration_record_as_input.md) | The `Copy`, `PartialEq` record the invariant quantifies over |

### Invariants

| File | Relationship |
|------|--------------|
| [002_construction_is_the_only_path.md](002_construction_is_the_only_path.md) | The companion — this one says the config determines the ring, that one says nothing else builds one |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md](../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md) | V5's distinction — the invariant and the criterion fail separately |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_configuration_as_data.md](../pattern/001_configuration_as_data.md) | Why the input is a value at all, which is what makes `a == b` meaningful |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_criterion_grades_the_clamped_value.md](../pitfall/001_the_criterion_grades_the_clamped_value.md) | Holds regardless of clamping — equal configs build equal rings whether or not either was corrected |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_name_state_through_a_registration.md](../lifecycle/004_name_state_through_a_registration.md) | V3's ordering constraint, stated as transitions |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | `RingConfig`'s derives — `PartialEq` is what makes the statement checkable |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | ✅ `two_rings_from_one_config_behave_identically` and `two_factories_build_identically` do exactly this. **What it does not catch is wider than V1 alone:** two rings built from configs differing only in `producers` also behave identically through this crate's surface, because `Split` exposes no backend discriminant — so the determinism assertion passes for a reason stronger than the invariant claims. `build_crossbeam` is the one construction path the invariant does not cover, and it says so rather than being excepted (→ [`decisions/002`](../decisions/002_two_doors_not_one_that_routes.md)) |

### FC21 — The Enforcement Mechanism Is Structural, and the One This Instance Names Is a Grep

A second input would have to be stored somewhere. There is nowhere:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every place a second input could hide --'
for k in 'static ' 'thread_local!' 'Atomic' 'Mutex' 'RefCell' 'Cell<' 'unsafe' 'lazy'; do
  printf '  %-14s %s\n' "$k" \
    "$( command grep -vE '^\s*(///|//!)' ring_factory/src/lib.rs | command grep -c "$k" )"
done
echo '  -- and the receiver --'
command grep -B1 'pub struct Factory;' ring_factory/src/lib.rs
```

Live output:

```
  -- every place a second input could hide --
  static         0
  thread_local!  0
  Atomic         0
  Mutex          0
  RefCell        0
  Cell<          0
  unsafe         0
  lazy           0
  -- and the receiver --
#[ derive( Debug, Clone ) ]
pub struct Factory;
```

Zero of each, and a fieldless receiver. `build` is a function of `( &Factory,
RingConfig )` where the first argument is zero-sized and carries no state, so it
is a function of the config alone — not by discipline but because the crate has
no storage, no interior mutability, no globals and no `unsafe` with which to
acquire any.

That is a strictly stronger enforcement than the grep this instance names, and
it fails differently: a grep over reads goes stale silently when a new read is
added, whereas adding a field to `Factory` or a `static` to the crate is a
structural change visible in any diff and in this block's own zeros. Stating the
mechanism as "nothing else is read" describes the current body; stating it as
"there is nowhere to read from" describes the shape, and the shape is what a
reviewer can check without re-reading the function.

The invariant is not thereby proved — `Ring::new` could consult a global one
crate down and this crate would not know — but the boundary is now exact: the
part of the invariant this crate can guarantee, it guarantees by construction,
and the part it cannot is entirely `ring_core`'s.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F 'The strongest layer is structural, not discipline' \
  ring_factory/docs/invariant/001_configuration_fully_determines_the_ring.md
```

Live output:

```
**The strongest layer is structural, not discipline — the crate has nowhere
to read a second input from** (§ FC21 below: zero occurrences of `static`,
`thread_local!`, `Atomic`, `Mutex`, `RefCell`, `Cell<`, `unsafe`, or `lazy` in
```

**Disposition:** applied — the "Enforcement Mechanism" section above no
longer opens with "There is no mechanism"; it now states the structural
guarantee this finding demonstrates — a fieldless `Factory` and zero
occurrences of any interior-mutability/global/`unsafe` construct in the
source — as its own strongest layer, with a new table row for it, and keeps
the four prior layers as the narrower discipline covering what the structural
layer cannot reach (a global inside `ring_core::Ring::new`). Now prints: `The
strongest layer is structural, not discipline`

### FC22 — The Determinism Test Passes for a Reason That Would Survive the Invariant Being False

`two_rings_from_one_config_behave_identically` builds twice from one config and
compares observable behaviour. It cannot fail while `Split` exposes what it
exposes:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what the surface can observe, per the crate own test --'
command grep -A4 'fn only_two_of_five_config_fields_are_observable_through_the_factory' \
  ring_factory/tests/factory_test.rs | command grep -vE '^\s*$'
echo '  -- and the determinism assertions --'
command grep 'fn two_rings_from_one_config_behave_identically\|fn two_factories_build_identically' \
  ring_factory/tests/factory_test.rs
```

Live output:

```
  -- what the surface can observe, per the crate own test --
fn only_two_of_five_config_fields_are_observable_through_the_factory()
{
  // Four slots. `observable_profile` offers eight records, and the window in
  // which that discriminates anything is narrower on **both** sides —
  // measured, not reasoned (→ `tests/manual/readme.md` F2):
  -- and the determinism assertions --
fn two_rings_from_one_config_behave_identically()
fn two_factories_build_identically()
```

Two of five fields reach anything the surface can see. So two rings built from
configs differing in `producers`, `wait` and `batch` are also indistinguishable
through this crate — meaning the determinism assertion holds over a set far
wider than "equal configs", and would keep holding if `build` started reading a
sixth input, provided that input touched none of capacity or overflow.

The test is not wrong and it is not worthless: it catches the case where `build`
becomes nondeterministic in the two fields that *are* observable, which is the
only failure a caller of this crate could see. What it cannot do is what its name
suggests — establish the invariant. The invariant's real content is about fields
this crate's output does not expose, and a test written at this crate's grain
has no access to them, exactly as
[`algorithm/001`](../algorithm/001_selecting_a_backend_from_one_boolean.md) FC2
found for the branch itself.

The narrow consequence: a green run of this crate's suite is evidence about two
fields and silence about three, and nothing in the suite's output distinguishes
the two cases.

`two_rings_from_one_config_behave_identically`'s own doc comment now says so,
at the point a reader of the test — not only of this instance — would meet it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -B9 '^fn two_rings_from_one_config_behave_identically' ring_factory/tests/factory_test.rs
```

Live output:

```
/// This is the invariant's positive half: nothing outside the config — no
/// factory state, no call order, no ambient default — reached the ring.
///
/// **Only as strong as what `observable_profile` can see.** `Split` exposes
/// no backend discriminant for `wait`, `batch` or `producers`, so this would
/// pass just as cleanly if `build` silently ignored those three fields — it
/// proves determinism in `capacity` and `overflow` only, the two fields this
/// crate's surface can actually observe (`docs/invariant/001` FC22).
#[ test ]
fn two_rings_from_one_config_behave_identically()
```

**Disposition:** applied — `two_rings_from_one_config_behave_identically`'s
doc comment in `ring_factory/tests/factory_test.rs` no longer states
only the invariant's positive half; it now states this test's own coverage
limit — that it proves determinism in `capacity` and `overflow` alone, since
`Split` exposes no discriminant for the other three fields — closing the
"nothing in the suite's output distinguishes the two cases" gap at the test
site itself. The crate's test suite re-verified passing (`cargo test -p
ring_factory --all-features`, 2026-09-04). Now prints: `silently ignored
those three fields — it`
