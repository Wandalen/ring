# Algorithm: Selecting a Backend From One Boolean

### Scope

- **Purpose**: Record that runtime backend selection is a single comparison on a single field, that the comparison is total and irreversible, that the branches return different Rust types — which is the whole difficulty — and that a third backend now arrives by a route the boolean cannot see.
- **Responsibility**: State the abstract and the algorithm.
- **In Scope**: `is_multi_producer()`; the branch; how distinct types are returned from one function; the compile-time third backend and why it is not on this branch.
- **Out of Scope**: What happens after the backend is chosen (→ [`algorithm/002`](002_assembling_a_ring_from_a_validated_record.md)); whether `producers` is the value the caller wrote (→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)).

### Abstract

**One field of five decides which of two implementations exists, and it decides
by a comparison against 1.**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A3 'pub const fn is_multi_producer' ring_config/src/lib.rs
```

Live output:

```
    pub const fn is_multi_producer(&self) -> bool {
        self.producers > 1
    }
```

`ring_config`'s own documentation calls this "the one derived reading in the
record, and the one a factory branches on." It is derived, `const`, total, and
has no failure mode.

**Everything interesting about the branch is what it *cannot* consider.**
Capacity does not affect it. Overflow policy does not. Wait strategy does not.
Batch size does not. A single-producer ring with a batch of 64 is SPSC; a
two-producer ring with a batch of 1 is MPSC. This is not a simplification — it
is the correct decomposition, because the SPSC/MPSC distinction is about how
many threads may claim a slot, and none of the other four fields carries that
information.

**The branch is irreversible and unobservable afterwards.** Nothing downstream
can convert an SPSC ring into an MPSC one, and nothing in the returned handle
pair is required to report which was chosen. A caller who wanted MPSC and got
SPSC finds out when a second producer thread corrupts the ring, not when
`build` returns.

### Algorithm

**Step 1 — read the derived boolean.**

```text
multi ← cfg.is_multi_producer()          // cfg.producers > 1
```

No validation. `producers` cannot be zero by the time it is read: the setter
clamps a zero to one (→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)'s C4),
and the field's default is one. So the branch is total over every constructible
`RingConfig`, which is why it is a `bool` and not a `Result`.

**Step 2 — branch.**

```text
if multi  → ring_mpsc::Ring< S >
else      → ring_spsc::Ring< S >
```

**Step 3 — the return type problem, which is the actual work.**

`ring_spsc::Ring< S >` and `ring_mpsc::Ring< S >` are unrelated types. Every
trait either implements is from `core` — there is no family trait spanning them:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'impl.*for Ring' ring_spsc/src/lib.rs ring_mpsc/src/lib.rs
```

Live output:

```
ring_spsc/src/lib.rs:unsafe impl< S : Send > Sync for Ring< S > {}
ring_spsc/src/lib.rs:impl< S > core::fmt::Debug for Ring< S >
ring_mpsc/src/lib.rs:unsafe impl< S : Send > Sync for Ring< S > {}
ring_mpsc/src/lib.rs:impl< S > core::fmt::Debug for Ring< S >
```

`Sync` and `Debug`, twice each, and nothing else. So a single function cannot
return "one or the other" without a construct that
erases the difference. Four are available and they are not equivalent:

| # | Construct | Cost | Note |
|---|-----------|------|------|
| A1 | An enum with two variants, matched on every operation | One branch per operation, predictable and cheap after the first | The branch is on a field that never changes, so it predicts perfectly |
| A2 | A trait object (`Box< dyn … >`) | An indirect call per operation, no inlining across it | Puts a vtable dispatch on the tick path, which is the path this family's tick-safety design constrains |
| A3 | Generic over the backend, selected at the call site | Zero cost; **the caller chooses the backend, not the config** | Contradicts the premise — the whole point is that `producers` decides |
| A4 | Both, always: construct one and leave the other absent | The memory of the larger, unconditionally | Wasteful and does not remove the branch |

**A3 is the one to reject explicitly**, because it is what a Rust author reaches
for first and it inverts the design. If the backend is a type parameter, the
selection has moved from `RingConfig` to the call site, and
[`invariant/002`](../invariant/002_construction_is_the_only_path.md) is lost —
there is no single enumerable set of legal rings, only whatever combinations
call sites instantiate.

**A2 is the one to reject on measurement grounds.** This family exists to
produce a comparison table between write-path patterns. An indirect call on
every publish is a cost paid inside the thing being measured, and it would be
paid identically by both branches, so it does not even cancel out of the
comparison — it compresses it.

**A1 is the shape this crate should take, and it moves the decision rather than
removing it.** The enum lives somewhere — plausibly `ring_core`, which already
declares both backends — and this crate's job becomes constructing the correct
variant.

**That placement is now settled, and it went where this instance predicted.**
`ring_core` was implemented after the paragraph above was written and put the
enum exactly there, private, with the branch inside it:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^enum Storage/,/^}/p' ring_core/src/lib.rs
command grep -m1 -A5 -F '    let storage = match config.is_multi_producer()' ring_core/src/lib.rs
```

Live output:

```
enum Storage< T >
{
  Spsc( ring_spsc::Ring< TypedSlot< T > > ),
  Mpsc( ring_mpsc::Ring< TypedSlot< T > > ),
  #[ cfg( feature = "crossbeam" ) ]
  Crossbeam( crossbeam_queue::ArrayQueue< T >, Capacity ),
}
    let storage = match config.is_multi_producer()
    {
      true => Storage::Mpsc( ring_mpsc::Ring::with_config( config ) ),
      false => Storage::Spsc( ring_spsc::Ring::with_config( config ) ),
    };
```

**So this crate never writes the branch at all** — it calls
`ring_core::Ring::new( &cfg )` and the comparison happens one crate down. The
algorithm documented here is still the algorithm; the ownership moved. That is
the good outcome for A1 (the enum is private, so no consumer can match on it)
and it removes one entry from [`decisions/`](../decisions/readme.md).

#### The third backend, which this branch does not select

`Storage` has a third variant and `is_multi_producer()` cannot reach it.
`Crossbeam` is behind `#[ cfg( feature = "crossbeam" ) ]` and is chosen by a
second constructor rather than by the config:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'pub fn new\|pub fn new_crossbeam\|feature = "crossbeam"' ring_core/src/lib.rs | head -6
sed -n '/^\[features\]/,/^$/p' ring_core/Cargo.toml
```

Live output:

```
  #[ cfg( feature = "crossbeam" ) ]
  #[ cfg( feature = "crossbeam" ) ]
  pub fn new( config : &RingConfig ) -> Result< Self, RingError >
  #[ cfg( feature = "crossbeam" ) ]
  pub fn new_crossbeam( config : &RingConfig ) -> Result< Self, RingError >
      #[ cfg( feature = "crossbeam" ) ]
[features]
default = []
# Feature 187: an optional third backend, so consumers needing a working
# multi-producer channel are not blocked on the in-house ring being finished.
crossbeam = [ "dep:crossbeam-queue" ]
```

`ring_core`'s own doc comment states the reason: the crossbeam backend "is
selected by [`new_crossbeam`] rather than by a config field, because it is a
build-time opt-in rather than a property of the workload." That reason explains
why it exists at all — an optional third backend so consumers needing a working
multi-producer channel are not blocked on the in-house ring being finished.

**This is A3 in a place A3 was rejected for, and the distinction is real rather
than a loophole.** A3 was rejected because making the backend a type parameter
moves selection from `RingConfig` to the call site, dissolving the enumerable
set of legal rings. A cargo feature moves it to the *build*, which is different
in the way that matters here: within one compiled binary the set of legal rings
is still enumerable, and the sweep still enumerates it. What changes is that the
set is now a function of the feature flags, so **two builds of the same
workspace can disagree about what `build` does with the same `RingConfig`**.

| | Selected by | Visible to the sweep | Enumerable within one binary |
|---|---|---|---|
| SPSC | `producers == 1` | Yes | Yes |
| MPSC | `producers > 1` | Yes | Yes |
| Crossbeam | `--features crossbeam` **and** calling `new_crossbeam` | **No** — no config value reaches it | Yes, but only if the feature is on |

**The consequence for this crate is a question it cannot answer alone.** `build`
takes a `RingConfig` and nothing else, so it can never produce a crossbeam ring;
either the third backend is unreachable through the factory — making the
Contract's "reached *through* this surface" ruling incomplete for the optional crossbeam backend —
or `build` gains an input that is not a config field, which is what
[`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)
exists to forbid. Recorded in [`decisions/`](../decisions/readme.md).

**Step 4 — there is no step 4.** The backend does not need to be told how many
producers there are. `ring_mpsc`'s own doc comment is explicit that its
claim path "is safe for many producers because it is a compare-exchange, not
because it was told one." The count selects the implementation and is not an
argument to it, which is why `Ring::with_config` discarding `producers` is
harmless for MPSC specifically and catastrophic as a general pattern
(→ [`invariant/002`](../invariant/002_construction_is_the_only_path.md)'s L2).

### Algorithms

| File | Relationship |
|------|--------------|
| [002_assembling_a_ring_from_a_validated_record.md](002_assembling_a_ring_from_a_validated_record.md) | What runs once the branch is taken; the other four fields' fates |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | Where the return type from step 3 becomes a signature |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_configuration_record_as_input.md](../data_structure/001_the_configuration_record_as_input.md) | `producers`, and the four fields that do not reach this branch |
| [../data_structure/002_the_handle_pair_as_output.md](../data_structure/002_the_handle_pair_as_output.md) | What A1's enum has to be wrapped in before it leaves |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_construction_is_the_only_path.md](../invariant/002_construction_is_the_only_path.md) | What A3 would cost — the enumerability this branch exists to preserve |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_configuration_as_data.md](../pattern/001_configuration_as_data.md) | Why the selection is a value's consequence rather than a call-site choice |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_criterion_grades_the_clamped_value.md](../pitfall/001_the_criterion_grades_the_clamped_value.md) | C4 — a clamp on `producers` changes this branch, not a parameter |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_config_state_through_a_build.md](../lifecycle/003_config_state_through_a_build.md) | The branch as a transition, and why it has no failing edge |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | `is_multi_producer`, quoted above |
| [`ring_spsc/src/lib.rs`](../../../ring_spsc/src/lib.rs) | One branch's `Ring< S >` |
| [`ring_mpsc/src/lib.rs`](../../../ring_mpsc/src/lib.rs) | The other's, and the compare-exchange note behind step 4 |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | ⚠️ **The prescribed test could not be written as prescribed.** `build` returns a `Split`, which exposes no backend discriminant, so the two rings do *not* differ observably through this crate's own output — `only_two_of_five_config_fields_are_observable_through_the_factory` asserts exactly that. The branch is instead exercised one level down by `the_backend_does_change_with_producer_count_where_it_can_still_be_seen`, which calls `Ring::new` directly and reads `Backend::Spsc` / `Backend::Mpsc`. **The warning against a `cfg.producers()` round-trip test was right and did not go far enough:** the honest alternative it implied is unavailable at this crate's grain |

### FC1 — Three Backends, Two Selection Mechanisms, and Neither Can See the Other

The boolean above chooses between two implementations. There is a third, and it
is not on this branch — it is reached by calling a different function, chosen at
compile time. So backend selection in this crate is two mechanisms operating at
two different times, and each is blind to the other's input:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two constructions, which differ in exactly one call --'
command grep 'Ring::new( &cfg )\|Ring::new_crossbeam( &cfg )' ring_factory/src/lib.rs
echo '  -- every config field the crossbeam constructor reads --'
command grep -A9 'pub fn new_crossbeam' ring_core/src/lib.rs | command grep -E 'config\.'
printf '  mentions of the branch field on this crate side: %s\n' \
  "$( command grep -c 'is_multi_producer' ring_factory/src/lib.rs )"
```

Live output:

```
  -- the two constructions, which differ in exactly one call --
    let ring = Ring::new( &cfg ).map_err( BuildError::Unsupported )?;
    let ring = Ring::new_crossbeam( &cfg ).map_err( BuildError::Unsupported )?;
  -- every config field the crossbeam constructor reads --
          crossbeam_queue::ArrayQueue::new( config.capacity().get() ),
          config.capacity(),
  mentions of the branch field on this crate side: 0
```

`new_crossbeam` reads `capacity()` and `overflow()` and stops. `producers` — the
one field this entire algorithm turns on — is not read on that path by anybody,
and `is_multi_producer` is never named in this crate at all.

The consequence is a silent config downgrade with no diagnostic anywhere. A
caller who builds a `RingConfig` with `producers = 4` and hands it to
`build_crossbeam` gets a ring, and `crossbeam_queue::ArrayQueue` is in fact
multi-producer safe, so the ring is not *wrong* — but nothing in the call
checked, and the same call with `producers = 1` returns something
indistinguishable. The field that decides everything on one door is inert on the
other, and the two doors have the same argument type.

`#[ cfg ]` cannot read a runtime value and `is_multi_producer` cannot read a
build flag, so this is not a bug that could be fixed by moving code. It is the
cost of the ruling in [`decisions/002`](../decisions/002_two_doors_not_one_that_routes.md),
stated here as what the algorithm does not cover rather than as what the ruling
gave up.

### FC2 — The Crate's Only Algorithm Has No Test at This Crate's Grain

`build` returns a `Split`, which exposes no backend discriminant, so the branch
has no observable consequence through this crate's own output. The test that
proves the branch works reaches around the crate to do it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the API the backend test actually calls --'
command grep 'Ring::new( &base\|\.backend()' ring_factory/tests/factory_test.rs
printf '  tests reaching a backend through a Factory return value: %s\n' \
  "$( command grep -c 'Factory.*backend()' ring_factory/tests/factory_test.rs )"
```

Live output:

```
  -- the API the backend test actually calls --
  let single : Ring< u32 > = Ring::new( &base.with_producers( 1 ) ).expect( "a ring" );
  let multi : Ring< u32 > = Ring::new( &base.with_producers( 4 ) ).expect( "a ring" );
  assert_eq!( single.backend(), Backend::Spsc );
  assert_eq!( multi.backend(), Backend::Mpsc );
  tests reaching a backend through a Factory return value: 0
```

`Ring::new` and `Backend::Spsc` are `ring_core`'s API, called from
`ring_factory`'s test file. The test is correct, its doc comment argues honestly
for why it exists — it is the control against "the factory ignores the field
entirely" — and it establishes a property of `ring_core`, not of `build`.

So the crate's central algorithm is verified by a test that does not call it.
That is defensible and it is also the strongest available argument that this
algorithm belongs one crate down: everything `build` contributes to backend
selection is passing `&cfg` along, and the selection itself, the branch, the two
types, and the only assertion that can see them all live in `ring_core`. The
instance is filed here because the *decision* to have one door is this crate's;
the mechanism it documents is not.
