# docs

Design documentation for `ring_types`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The one validation the crate performs, and the classification it performs without ever being asked to |
| `api/` | Five types exported to thirty crates, and five predicates exported to none |
| `data_structure/` | Two position types and the fold between them; an error set closed by `Copy` rather than by ceremony |
| `decisions/` | Open questions, and the fixes this effort found and deliberately did not apply |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Thirty dependents and one that declined the shared error — the crate's position read from both ends |
| `invariant/` | Three properties on one axis: how much of each the compiler actually holds |
| `item/` | All 40 source items, each with its call sites measured rather than asserted |
| `lifecycle/` | Two values traced across crate boundaries, and two traced through the states the design permits — including the fifth it forbids |
| `non_functional_requirement/` | Three thresholds with measurement recipes — two met, one met by a mechanism outside `src/` |
| `pattern/` | Behaviour pushed out, a check pulled in — opposite directions, one protected property |
| `pitfall/` | A doc comment that contradicts its own function, and two errors nothing constructs |
| `type/` | The two types with enough internal structure to be worth a domain reading |
| `workaround/` | The one external constraint a dependency-free crate can still have: the language itself |

**This crate is vocabulary, and every directory above is arranged around what
that costs.** It owns no ring, no buffer, no cursor and no strategy. It owns five
exported names — `Capacity`, `RingError`, `Seq`, `SlotIndex`, and the two policy
enums — and thirty of the family's thirty-three crates depend on it
([`integration/001`](integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md)).
Its `[dependencies]` table is empty — one of only two in the family of which that
is true, the other being `ring_align`.

The instances sit at three grains. `invariant/`,
`non_functional_requirement/` and `workaround/` document the **contract grain** —
what holds, what is measured, and what the language could not give. `algorithm/`,
`data_structure/`, `pattern/` and `pitfall/` document the **mechanism grain**:
how a slot count becomes a mask, how an error set stays `Copy`, and where a doc
comment stopped matching its own code. `api/`, `lifecycle/`,
`type/`, `integration/` and `item/` document the **surface grain** — what a
consumer imports, what it can call, and what it never calls at all.

**Four findings run through the whole set.**

First, **a third of the exported surface has no production consumer.** Five
predicates — `is_configuration`, `is_transient`, `is_non_blocking`,
`reports_failure`, `drops_silently` — and exactly one of them is called by any
sibling's `src/` (`ring_config:240`). The other four exist to be asserted against
(→ [`api/002`](api/002_the_five_classifier_predicates.md)). That is not dead
code: a predicate is how a test states a rule without restating the `match` the
rule already lives in. But it means the crate's export Contract and the crate's
runtime footprint are different shapes, and only one of them is what the family
compiles against.

Second, **the discriminant/handler split works in a direction nobody designed
for.** The split (→ [`pattern/001`](pattern/001_discriminants_here_handlers_elsewhere.md))
put the discriminants here and the strategies in `ring_wait` and `ring_overflow`.
The consequence measured here is that the *handler* crate defends the
*discriminant* roster better than this crate does — `ring_wait` pins
`WaitKind::ALL`'s contents and order with a single `assert_eq!`, stronger than
anything in `ring_types`' own suite, because `ring_wait` is the crate that would
suffer from a reordering
(→ [`item/associated_constant/002`](item/associated_constant/002_wait_kind_all.md)).
The split makes it possible for the two sides to disagree, which is the property
that makes either side's assertion mean anything.

Third, **the three invariants are one axis, not three subjects**: how much of the
property the compiler actually holds.
[`invariant/001`](invariant/001_every_capacity_has_a_valid_mask.md) is held by
construction — the type cannot exist in a violating state.
[`invariant/002`](invariant/002_tier_zero_depends_on_nothing.md) is held by a
manifest a gate reads.
[`invariant/003`](invariant/003_every_error_renders_distinctly.md) is held by a
hand-maintained test roster, and it has already been broken once —
`PolicyUnsupported` was added, the roster was not updated, and gate G1 caught it
as `16/17` line coverage rather than the compiler catching it as an error.

Fourth, **the crate's one workaround exists *because* of the invariant that makes
it dependency-free.** Every sibling could delete the two hand-written `ALL`
arrays by taking `strum::EnumIter`; this crate cannot, because
[`invariant/002`](invariant/002_tier_zero_depends_on_nothing.md) forbids the
dependency (→ [`workaround/001`](workaround/001_hand_written_all_arrays_stand_in_for_variant_enumeration.md)).
The one crate with nothing external to be constrained by is the one whose
constraint has no cheap escape.

**A corollary carried from `ring_flush` and extended here:** a finding resting on
a dependency closure is perishable while the family is being implemented, so
every count in these instances ships with the command that regenerates it. The
extension is that **a finding resting on an unimplemented crate is perishable in
a way no recipe catches** — a count can be regenerated, *"nothing can fail here"*
cannot, because the evidence for it is an absence. Three concrete hazards, each
of which cost a correction in this crate, are recorded in
[`definition/readme.md`](definition/readme.md).

**An `item/` directory exists here and in no other crate of the 33**, which is
the deliberate inverse of `ring_flush`'s decision to omit one. The argument there
was grain: its 27 declarations are five public types already documented by
`type/` and `api/`, so an `item/` instance would be a fifth listing of the same
names. Here the grain is different — the crate is 607 lines and 40 items, its
`type/` definition covers 2 of them, and the per-item measurements
(`Seq::advanced_by` has 14 production callers across 7 crates; `WaitKind::ALL`
has 0) are facts no design-dimension document has a place for
(→ [`item/readme.md`](item/readme.md)).

No `format/` directory exists: this crate defines no byte layout. `Seq` is a
`u64` and `SlotIndex` a `usize` because of what they index, not because any
external format requires it.

No `protocol/` or `operation/` directory exists: nothing here has a wire
representation or a runnable procedure. The single procedure the crate contains —
validating a slot count — is documented as an
[`algorithm/`](algorithm/001_validating_a_slot_count_to_a_power_of_two.md)
instance, which is where a pure computation with no operational surface belongs.

### Related Crates

Thirty dependents, two declared non-dependents, and no dependencies at all.

| Crate | Relationship |
|-------|--------------|
| [`ring_registry/readme.md`](../../ring_registry/readme.md) | **The one sibling that does not depend on this crate.** `NameTaken` and `NameUnknown` live in `RingError` for it, and it declined them — the whole subject of [`integration/002`](integration/002_the_registry_that_declined_the_shared_error.md) |
| [`ring_wait/readme.md`](../../ring_wait/readme.md) | Owns the strategies `WaitKind` names. Defends this crate's roster better than this crate does, and checks the non-blocking property against behaviour rather than against the predicate (the discriminant/handler split) |
| [`ring_overflow/readme.md`](../../ring_overflow/readme.md) | Owns the strategies `OverflowPolicy` names. The other half of the same ruling; its two predicate-vs-outcome assertions are what let the split disagree |
| [`ring_config/readme.md`](../../ring_config/readme.md) | The only sibling calling any classifier predicate in production — `is_non_blocking`, via `is_tick_safe` (`:238`) |
| [`ring_stats/readme.md`](../../ring_stats/readme.md) | The family's only production use of any `ALL` array: `dropped_total` sums over `OverflowPolicy::ALL` (`:378`), so a corrupted roster is a wrong number rather than an error |
| [`ring_cursor/readme.md`](../../ring_cursor/readme.md) | Four of `Seq::ZERO`'s six production references, in two `cfg`-gated constructors — the one place `Default::default()` is not callable and the constant is not redundant |
| [`ring_store/readme.md`](../../ring_store/readme.md) | Both of `SlotIndex::get`'s call sites. Carries two entry points — `get`/`get_mut` take a `SlotIndex`, `at`/`at_mut` take a `Seq` — which is where the fold documented in [`data_structure/001`](data_structure/001_two_position_types_and_the_fold_between_them.md) lands |
| [`ring_index/readme.md`](../../ring_index/readme.md) | One of the two `Capacity::mask` callers. The fold itself, extracted; `ring_mpsc` re-implements it inline and correctly, which is the point of [`pattern/002`](pattern/002_a_newtype_that_makes_a_check_unnecessary.md) |
| [`ring_core/readme.md`](../../ring_core/readme.md) | Used to be one of two `Capacity::new` callers family-wide (`:219`) — refactored to carry the validated `Capacity` it's constructed from instead of re-validating it (→ [`algorithm/001`](algorithm/001_validating_a_slot_count_to_a_power_of_two.md) TY20), so `ring_config` is now the sole production caller |
| [`bench_harness/readme.md`](../../bench_harness/readme.md) | Owns the gates. G1 caught the `16/17` coverage regression that `invariant/003` records; G6 reads the manifest `invariant/002` asserts |
