# Lifecycle: From a Record to a Handle Pair

### Scope

- **Purpose**: Trace one construction from the caller's first `RingConfig::new` to the moment ownership of the pair leaves this crate, and identify the single phase at which a refusal is still possible.
- **Responsibility**: State the phases, transitions, dependencies, and cleanup requirements.
- **In Scope**: The arc of one build; where validation, clamping, branching and registration each fall.
- **Out of Scope**: The ring's own operating life after handover (→ [`ring_handle`](../../../ring_handle/readme.md)); the factory's lifetime (→ [`lifecycle/002`](002_the_factory_outlives_nothing.md)).

### Lifecycle Phases

**Six phases, of which this crate owns four and can refuse in two.**

| # | Phase | Owner | Can refuse | Note |
|---|-------|-------|-----------|------|
| L1 | Config construction — `RingConfig::new( slots )` | `ring_config` | **Yes** — `Err` on zero or non-power-of-two | The only refusal that precedes this crate |
| L2 | Config refinement — `with_wait`, `with_overflow`, `with_producers`, `with_batch` | `ring_config` | **No, by design.** Clamps instead | Where the requested value is lost (→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)) |
| L3 | Backend selection | `ring_core`, called by **this crate** | No | One comparison, total (→ [`algorithm/001`](../algorithm/001_selecting_a_backend_from_one_boolean.md)) |
| L4 | Assembly — capacity, overflow | `ring_core`, called by **this crate** | **Yes** — `PolicyUnsupported` on `DropOldest` | Allocation happens here; `wait` and `batch` pass through unread |
| L5 | Registration | **This crate**, naming path only | **Yes** — `NameTaken` | The one refusal this crate raises itself |
| L6 | Handover — the pair leaves | **This crate** | No | Ownership transfers wholly; nothing is retained |

**The distribution is the finding, and it changed twice.** This table was written
with L4 marked "No" and the conclusion that on the ordinary unnamed path no
phase from L3 to L6 could say no — true while `ring_core` had no implementation,
and the reason [`api/001`](../api/001_the_build_surface.md) once specified
`build` as returning the pair directly.

**The second change is at L6 and is larger.** The pair does not leave; its
*owner* does. `Split` owns the ring, `ends()` borrows it, `split()` divides the
borrow — so L6 hands over one value and the caller performs the split whenever
they are ready, as many times as the borrows allow
(→ [`decisions/001`](../decisions/001_the_owner_is_the_return_value.md)). This
instance's own title says "to a handle pair", and the arc now ends one step
earlier than that.

**L4 now refuses, and it refuses on both paths.** `ring_core::Ring::new` rejects
`OverflowPolicy::DropOldest`, a value `ring_config` accepts at L2 without
clamping or complaint. So the arc has two refusal points inside this crate's
span rather than one, and the unnamed path passes through the new one:

| Path | Refusals it can meet |
|------|----------------------|
| `build( cfg )` | L4 only |
| `build_named( cfg, name )` | L4, then L5 |

**Ordering matters here and the table above fixes it.** L4 precedes L5, so a
config that cannot build never reaches the registry and cannot leave a name
half-claimed. The reverse order would make `Unsupported` capable of stranding a
registration, which is exactly the hazard
[`lifecycle/004`](../lifecycle/004_name_state_through_a_registration.md)
pins for `NameTaken`.

**The two silent gaps are unchanged and are now the outliers.** `wait`
unhonoured and tick-safety unchecked still have nowhere to surface — they are
the only things on this arc that fail without a phase to fail in.

**L2 is where the arc loses information and it is not recoverable downstream.**
Everything after L2 sees a legal record and cannot distinguish a value that was
asked for from one that was corrected. That is a property of the arc, not of any
single crate, and it is why the mitigation in
[`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md) is
"record the config, not the request" at the far end rather than a check here.

### Phase Transitions

| # | Transition | Trigger | Failure edge |
|---|-----------|---------|--------------|
| T1 | L1 → L2 | The caller holds a `RingConfig` | — |
| T2 | L1 → ✗ | `new` rejected the capacity | **`Err( RingError )`.** The arc never starts |
| T3 | L2 → L2 | Each setter returns a new `Self` | None. A setter cannot fail |
| T4 | L2 → L3 | `build( cfg )` or `build_named( cfg, name )` is called | — |
| T5 | L3 → L4 | `is_multi_producer()` answered; a backend is chosen | None. Total |
| T6 | L4 → L6 | Assembly complete, unnamed path | — |
| T7 | L4 → L5 | Assembly complete, naming path | — |
| T8 | L5 → L6 | The name was free; the registry holds it | — |
| T9 | L5 → ✗ | The name was taken | **`Err( NameTaken )`**, and the ring built in L4 must not survive |
| T10 | L6 → (out of scope) | The caller holds both handles | — |
| T11 | L4 → ✗ | `overflow` is `DropOldest` | **`Err( Unsupported )`**, raised before anything is allocated |

**T9 is the transition with an obligation attached; T11 is the one without,
and the contrast is the useful part.** By T9 the ring exists — L4 allocated it —
so a refusal at L5 has something to dispose of, and the obligation is that
nothing built in L4 is reachable after T9 returns. T11 fires at the *top* of L4,
before `ring_core` constructs any storage:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A17 -F '  pub fn new( config : &RingConfig ) -> Result< Self, RingError >' ring_core/src/lib.rs
```

Live output:

```
  pub fn new( config : &RingConfig ) -> Result< Self, RingError >
  {
    if config.overflow() == OverflowPolicy::DropOldest
    {
      return Err( RingError::PolicyUnsupported );
    }

    // `match` rather than `if`/`else`: under the family's brace style the `else`
    // keyword lands on a line of its own, where `llvm-cov` opens a region that
    // nothing can ever execute — so the crate reads 120/121 with both arms
    // demonstrably covered. `tests/manual/readme.md` C4 records the measurement.
    let storage = match config.is_multi_producer()
    {
      true => Storage::Mpsc( ring_mpsc::Ring::with_config( config ) ),
      false => Storage::Spsc( ring_spsc::Ring::with_config( config ) ),
    };

    Ok( Self { storage, overflow : config.overflow() } )
```

The policy check precedes the `match` that builds `Storage`, so T11 has nothing
to clean up and needs no counterpart to T9's ordering rule. **Two failure edges
in one phase with opposite obligations is worth stating explicitly**, because a
reader who generalises T9's disposal rule to "L4 and L5 must clean up on
failure" will add a drop path that can never run.

**T3's self-loop is where the arc's one asymmetry lives.** Each setter consumes
and returns a `RingConfig`, so a chain is a sequence of distinct values, and
only the last one reaches T4. A caller who writes `cfg.with_batch( 64 );` on its
own line — statement, not assignment — has built a config and discarded it.
`RingConfig`'s setters are `#[ must_use ]`-shaped by convention rather than by
attribute; whether they carry the attribute is `ring_config`'s to say.

**There is no transition from L6 back to anything.** Once the pair leaves, this
crate has no reference to it, no list it was added to, and no way to reach the
ring again — except through the registry on the naming path, which is the sole
exception and is what makes [`lifecycle/002`](002_the_factory_outlives_nothing.md)'s
claim conditional.

### Dependencies

| # | Dependency | Needed at | Reachable |
|---|-----------|-----------|-----------|
| D1 | `ring_config` | L2's product arrives at T4 | Declared. Implemented — 13 items |
| D2 | `ring_core` | L3 and L4 | Declared. **Implemented — 26 items.** Owns both phases and T11 |
| D3 | `ring_registry` | L5 | Declared. Implemented — 12 items. **Also re-exported**, which is how L5's lookup half reaches a Contract-bound consumer |
| D4 | `ring_handle` | L6 | ✅ **Now declared directly.** It was reachable only through `ring_registry`, which is not enough to *name* a type, and L6's product is a `Split` (→ [`integration/001`](../integration/001_declared_edges_and_the_reached_closure.md)'s requirement 1). Implemented — 20 items |
| D5 | `ring_stats` | Nowhere in this arc | ❌ **No longer declared.** The manifest line went rather than the arc growing a phase for it — `RingConfig` has no field that would configure counters. Implemented — 21 items, none reachable from here |
| D6 | `ring_tls` | Nowhere in this arc | ❌ **No longer declared**, same reason — and its removal took `ring_batch` out of the closure too, since this was its only route in. Implemented — 15 items |
| D7 | `ring_wait` | L4, if `wait` were honoured | **Not reachable, now for a positive reason.** Its 7 items are free functions taking a `WaitKind` per call — there is no waiter to construct at L4, so there is nothing to depend on it *for* (→ [`pitfall/002`](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md)) |

Confirm the last column rather than trusting it — the concurrent implementation
effort has moved it twice during this crate's documentation:

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_config ring_core ring_registry ring_handle ring_stats ring_tls ring_wait; do
  printf '%-14s %s\n' "$c" \
    "$( cat $c/src/*.rs | command grep -cE '^\s*(pub )?(fn|struct|enum|trait|type|const) ' )"
done
```

Live output:

```
ring_config    13
ring_core      26
ring_registry  12
ring_handle    20
ring_stats     21
ring_tls       15
ring_wait      7
```

Count across `src/*.rs`, not `src/lib.rs` alone: `ring_types` reads as empty on
the narrower recipe and is a 22-item crate spread over four files.

**D2's implementation is what reshaped this instance.** L3 and L4 were written as
phases *this crate* owns; `ring_core` now owns both, and this crate calls one
function. The phase list is unchanged in substance — the same work happens in
the same order — but the Owner column moved, and with it the crate that decides
what L4 refuses.

**Every row in this table changed while the crate was being implemented, and two
changed direction.** D3 and D4 went from skeletons to the two crates whose
shipped shapes ruled four of this crate's ten pending decisions; D5 and D6 went
from *declared but unused* to *not declared*, which is the branch that removes
rather than adds. The table is worth re-running rather than reading — the recipe
below is why it is here at all.

**D5 and D6 are declared and this arc never touches them**, which is either two
dependencies the manifest should not carry or two phases this lifecycle is
missing. Both are now implemented, so the "not yet written" reading is closed:
whatever they are for, this arc is not using it. `ring_stats`
plausibly belongs at L4 — a ring built with counters attached — and `ring_tls`
plausibly belongs nowhere near construction at all, since staging
is per-thread and created long after a ring is. Neither is settled; recorded in
[`decisions/`](../decisions/readme.md).

**D7's absence is the arc's one hard blocker, and it hardened.** `ring_wait` is
implemented — `pause( kind, attempt )`, `wait_until`, `for_space`, `for_data` —
and remains outside this crate's closure, so L4 still cannot construct a waiter.
The gap is no longer "the strategies are unwritten"; it is one missing manifest
edge between working code and a field that names it.

### Cleanup Requirements

| # | Requirement | Enforced by |
|---|------------|-------------|
| C1 | After T9, nothing built in L4 is reachable | Ordering — build, then register, then hand back; never hand back before registering |
| C2 | The factory retains nothing after L6 | The type has no fields (→ [`type/001`](../type/001_factory.md)) |
| C3 | Dropping the factory destroys nothing | Same — it owns nothing to destroy |
| C4 | A registered ring outlives the handles that were returned, or does not | **Unresolved.** Depends on what the registry holds (→ [`data_structure/002`](../data_structure/002_the_handle_pair_as_output.md)'s H3) |

**C1 is satisfiable for free and is easy to lose.** The natural implementation —
build, then register — satisfies it because the L4 product is still a local when
L5 runs. An implementation that registers a *clone* and returns the original
satisfies it too. What breaks it is an implementation that inserts into the
registry, hands the pair to the caller, and then discovers the name conflict; no
sane author writes that deliberately, and a `?` in the wrong position produces
it.

**C4 is the real cleanup question and this crate cannot answer it.** If the
registry retains a route to the ring, then dropping both handles does not
destroy the ring, and "who closes a named ring" becomes a live question that
`ring_shutdown`'s close/reset/drain-all path owns. If the registry retains only a
name, lookup is nearly useless. Both readings are consistent with this crate's
own registration surface as designed.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | T4's unnamed entry, and why L3–L6 having no refusal means no `Result` |
| [../api/002_the_named_build_surface.md](../api/002_the_named_build_surface.md) | L5, T9, and C1 |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_selecting_a_backend_from_one_boolean.md](../algorithm/001_selecting_a_backend_from_one_boolean.md) | L3 |
| [../algorithm/002_assembling_a_ring_from_a_validated_record.md](../algorithm/002_assembling_a_ring_from_a_validated_record.md) | L4, phase by field |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_configuration_record_as_input.md](../data_structure/001_the_configuration_record_as_input.md) | L1 and L2's product |
| [../data_structure/002_the_handle_pair_as_output.md](../data_structure/002_the_handle_pair_as_output.md) | L6's product, and C4's dependence on H3 |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_declared_edges_and_the_reached_closure.md](../integration/001_declared_edges_and_the_reached_closure.md) | D4 and D7 — the dependency table's two anomalies |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_the_factory_outlives_nothing.md](002_the_factory_outlives_nothing.md) | C2 and C3, and the naming path's exception to them |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_criterion_grades_the_clamped_value.md](../pitfall/001_the_criterion_grades_the_clamped_value.md) | L2 — the phase that loses the request |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_config_state_through_a_build.md](../lifecycle/003_config_state_through_a_build.md) | L1–L4 as config states |
| [../lifecycle/004_name_state_through_a_registration.md](../lifecycle/004_name_state_through_a_registration.md) | T9 and C1's ordering |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | L1's refusal and L2's clamps |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | ⚠️ `a_refusal_drops_nothing_that_was_already_registered` and `a_refused_registration_leaves_the_original_ring_intact` cover C1 from the two sides that can be observed — the incumbent ring's records neither drop early nor leak, and its contents drain back in order. **C1's own wording asks for something no test here can see**: "the second call's ring is not reachable by any route" is trivially true (it was never returned) and its *destruction* is invisible, because a freshly built ring holds no records to count. Structural guarantee, not a checked one (→ `tests/manual/readme.md` F1) |

### FC29 — The Arc Crosses Four Crates and This Crate Holds Two Statements of It

The traced construction runs `RingConfig::new` → setters → `Factory::build` →
`ring_core::Ring::new` → the backend → `Split::new`. Four crates, and the
portion inside the crate the arc is filed under is this:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the whole of this crate contribution to the arc --'
sed -n '/pub fn build< S : Send >/,/^  }/p' ring_factory/src/lib.rs
echo '  -- and the crates the rest of it is in --'
command grep 'ring_config::\|ring_core::\|ring_handle::\|use ring_' ring_factory/src/lib.rs \
  | command grep -v '///'
```

Live output:

```
  -- the whole of this crate contribution to the arc --
  pub fn build< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
  {
    let ring = Ring::new( &cfg ).map_err( BuildError::Unsupported )?;
    Ok( Split::new( ring ) )
  }
  -- and the crates the rest of it is in --
//! use ring_factory::{ Factory, RingConfig };
//! cannot be written.** `ring_handle::Split::ends` borrows `&mut self` and
//! `OverflowPolicy::DropOldest` without complaint and `ring_core::Ring::new`
use ring_core::Ring;
use ring_handle::Split;
use ring_registry::RegistryError;
use ring_types::RingError;
pub use ring_config::RingConfig;
pub use ring_registry::Registry;
```

Two statements. The refusal phase the instance set out to locate is inside
`Ring::new`, one crate down; the clamping phase is inside `ring_config`, one
crate sideways; the split the caller performs afterwards is `ring_handle`'s.

That is the honest shape of a factory and it changes what this instance is for.
It is not a description of code in this crate — it is the only place the four
crates' phases are laid end to end in one order, and each of the four documents
its own segment without reference to where in the arc that segment falls. The
value is the ordering, and the ordering is exactly what no single crate's
`src/` can state.

The cost is that three quarters of it can go stale without any file in this
crate changing, which is the same exposure
[`algorithm/002`](../algorithm/002_assembling_a_ring_from_a_validated_record.md)
FC3 records for the field walk, arrived at from the other direction.
