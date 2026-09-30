# API: The Build Surface

### Scope

- **Purpose**: Fix the crate's central call — one argument in, one handle pair out — and record the four constructors it exists to make unnecessary, two of which are already public.
- **Responsibility**: State the operations, their error behaviour, and the compatibility guarantees.
- **In Scope**: `build`; its argument-by-value convention; the one refusal it must relay.
- **Out of Scope**: The naming variant (→ [`api/002`](002_the_named_build_surface.md)); what `build` does internally (→ [`algorithm/002`](../algorithm/002_assembling_a_ring_from_a_validated_record.md)).

### Abstract

**The whole surface is one function, and its shape is an argument about
constructors.**

```rust
impl Factory
{
  pub fn build< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >;
}
```

**The return type is not the one this file specified.** It said
`HandlePair< S >`, and that type **cannot be written** — `ring_handle::Split::ends`
borrows `&mut self` and `Ends::split` borrows `&'a mut self`, so a struct holding
the ring and both handles is self-referential. `build` returns the *owner*; the
caller makes the pair from it. → [`decisions/001`](../decisions/001_the_owner_is_the_return_value.md).

This crate's own acceptance criterion phrases the same thing as `Factory::build(cfg)` returning "a handle
pair whose observable behaviour matches every field." The signature above adds
three commitments to that phrasing, each of which could have gone the other way:

| # | Commitment | Alternative rejected |
|---|-----------|----------------------|
| B1 | `cfg` by value, not `&cfg` | A reference invites the caller to keep and mutate the original, which is a second input by another name (→ [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)). `RingConfig` is `Copy` and 32 bytes (→ [`data_structure/001`](../data_structure/001_the_configuration_record_as_input.md)), so by-value costs nothing |
| B2 | Returns `Result`, not the pair directly | **This reversed.** The surface was specified to return the pair directly, on the finding that nothing in the unnamed path could fail. `ring_core` was then implemented and refused `OverflowPolicy::DropOldest` (→ [`type/002`](../type/002_build_error.md)) — a legal config this crate must hand down and cannot un-refuse. The alternatives are worse: panicking hides a caller error in a library, and silently substituting a different policy returns a ring that does not match the config it was built from, which is exactly what [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md) forbids |
| B3 | `&self`, not `self` or an associated function | The factory is reusable and the registering variant may hold state (→ [`type/001`](../type/001_factory.md)'s N2). `self` would make one factory build one ring |

**B2 is worth reading as a record of how the finding failed rather than only as
a signature.** The original reasoning was a complete trace: every candidate
refusal was located, four were shown to happen in `ring_config` or not at all,
and the conclusion followed. It was falsified by a crate one level down being
written — not by a gap in the trace. **A surface documented against skeletons is
documented against a lower bound on what can fail**, and this family currently
has thirty-one of them.

**The slot type `S` is a parameter and the backend is not.** This is the line
the design walks: the caller chooses what goes *in* the ring, and the config
chooses what *kind* of ring it is. Making the backend a parameter too collapses
the distinction and takes the selection away from `RingConfig`
(→ [`algorithm/001`](../algorithm/001_selecting_a_backend_from_one_boolean.md)'s A3).

### Operations

| # | Operation | Signature shape | Notes |
|---|-----------|-----------------|-------|
| O1 | Build | `fn build< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >` | Fallible in exactly one way, and that way is relayed rather than owned |
| O2 | Build and register | `fn build_named< S : Send >( &self, cfg : RingConfig, name : &str, registry : &mut Registry< S > ) -> Result< (), BuildError >` | → [`api/002`](002_the_named_build_surface.md) |

**Two operations, and the second is not a flag on the first.** `build( cfg,
Some( name ) )` would collapse them and force every unnamed call site to write
`None`. The argument used to be stronger — it also avoided making the error
channel unconditional — and that half has now been overtaken: both operations
return `Result`, so the two signatures differ only by the `name` argument and by
which `BuildError` variants they can actually produce. **The remaining argument
is still sufficient**, because an `Option< &str >` that is `None` at most call
sites is a parameter every caller pays for to serve a minority, and because the
variants genuinely differ: `NameTaken` is unreachable from O1
(→ [`type/002`](../type/002_build_error.md)'s V1).

#### The four constructors this surface exists to prevent

This crate's own If Missing paragraph is the argument for O1 existing at all:

> Constructors sprout everywhere, one per combination someone needed. The set
> of legal configurations is whatever happens to have been written.

The four that a factory-less family would grow, and their status today:

| # | Constructor | Status |
|---|-------------|--------|
| P1 | `Ring::new( capacity )` per backend | **Exists**, twice, and is legitimate as a low-level entry |
| P2 | `Ring::with_config( &cfg )` per backend | **Exists**, twice, and reads one field of five (→ [`invariant/002`](../invariant/002_construction_is_the_only_path.md)'s L2) |
| P3 | `Ring::with_capacity_and_overflow( … )`, and the combinatorial family after it | Does not exist. **This is the one O1 genuinely prevents** |
| P4 | `RingBuilder` with a fluent chain terminating in `.build()` | Does not exist, and is already served — `RingConfig`'s setters *are* the fluent chain, and they return a value rather than a ring |

**P3 is the real target and P4 is the near-miss worth naming.** A builder that
accumulates fields and constructs at the end is the same design as
config-plus-factory with the seam in a different place; the difference is that
`RingConfig` is inspectable, comparable, `Copy`, and storable, and a
half-built builder is none of those. That is what makes the sweep possible: a
sweep enumerates configs, and it cannot enumerate builders.

**P1 and P2 already existing means O1 is not preventing anything yet** — it is
adding a fifth path to four. The prevention is real only once the export
surface has teeth, which is
[`integration/002`](../integration/002_the_crate_the_export_surface_routes_through.md)'s
subject.

### Error Handling

| Condition | Result |
|-----------|--------|
| Capacity invalid | **Cannot reach here.** `RingConfig::new` already returned `Err` |
| Batch or producers out of range | **Cannot reach here.** Clamped at the setter (→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)) |
| Overflow policy is `DropOldest` | **Reported.** `Err( BuildError::Unsupported( RingError::PolicyUnsupported ) )`, relayed from `ring_core::Ring::new` |
| Wait strategy unhonourable | **Not reported.** Accepted silently (→ [`pitfall/002`](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md)) |
| Config not tick-safe | **Not reported.** The check exists on `RingConfig` and nothing calls it (→ [`type/002`](../type/002_build_error.md)) |
| Allocation failure | Abort. Rust's behaviour, not this crate's choice |
| Name already registered | O2 only — `Err( BuildError::NameTaken )` |

**Seven rows, one reported on O1, and the interesting number is the two that are
neither impossible nor reported.** Two rows are genuinely unreachable, one is
Rust's to handle, two belong to O2 or to `ring_core`, and **two are
silent-by-omission** — a wait strategy nothing honours and a tick-safety check
nothing calls. Those two are the honest weakness of this surface: they are not
"cannot fail", they are "fails without saying so".

**The overflow row is the useful contrast.** It was in the silent group by
default — nothing checked it, so nothing reported it — right up until the crate
that would have to honour it was written and declined. **The distinguishing
question is not whether a row is currently reported but whether anything has yet
tried to implement it.**

By that test the two silent rows differ, and the difference is the opposite of
what "silent" suggests. `ring_wait` is implemented — seven public items,
including a `pause( kind, attempt )` that acts on `WaitKind` directly:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -E '^\s*pub (fn|const) ' ring_wait/src/lib.rs
```

Live output:

```
pub const DEFAULT_SPINS : usize = 1024;
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
pub fn pause( kind : WaitKind, attempt : usize ) -> bool
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
```

**So the honouring code exists and this crate does not call it — and the missing
manifest line turns out not to be the reason.** Every one of those seven items
takes the `WaitKind` as an *argument*. There is no waiter to construct, so a
`ring_wait` dependency here would buy nothing: the factory would validate a
discriminant and discard it, and whoever actually blocks would still pass the
kind themselves from the `RingConfig` they still hold, since `RingConfig` is
`Copy`.

The `wait` row is therefore not "unhonourable because one manifest line is
missing" — the sharper edge that reading claimed. It is **honourable by the
caller and not by this crate**, which is a smaller problem still, and it is
ruled rather than merely observed: `docs/decisions` Pending 5. `is_tick_safe`
remains the row with nothing behind it at all.

### Compatibility Guarantees

1. **`build` returns `Result`, and the variant set may only grow.** This
   guarantee previously read "`build` never returns `Result`", on the grounds
   that adding one later would break every call site. That was correct about the
   cost and wrong about the premise, and the cost was paid: the signature
   changed before any call site existed, which is the only cheap moment it could
   have. **The guarantee that replaces it is weaker and enforceable** — new
   `BuildError` variants are additive.

   `#[ non_exhaustive ]` was the proposed enforcement and is **deliberately not
   used**, for `type/001`'s reason plus a sharper one: the suite's
   `the_unnamed_path_can_never_return_name_taken` matches all three arms
   exhaustively, and that exhaustiveness is what makes it a structural claim
   rather than a spot check. `non_exhaustive` would force a wildcard arm and
   silently weaken the one test that enforces V1.
2. **`cfg` stays by value.** Switching to `&RingConfig` reopens the second-input
   question `invariant/001` closes.
3. **No public signature names a crate outside the Contract's five.**
   `RingConfig` is `ring_config` — *not* one of the five — so the argument type
   is re-exported from here: `pub use ring_config::RingConfig;`. ✅ **Resolved**
   as option W3 of three; `Registry` is re-exported for the same reason.
   → [`decisions/001`](../decisions/001_the_owner_is_the_return_value.md).
   Asserted by `the_contract_surface_is_reachable_without_naming_a_non_contract_crate`.
4. **`build` is not `const` and makes no promise about allocation.**
   Construction may allocate freely; what it must not do is leave a cost on the
   tick path (→ [`non_functional_requirement/002`](../non_functional_requirement/002_construction_cost_is_paid_once.md)).
5. **O1 and O2 stay separate**, for the reason under Operations.

**Guarantee 3 is the one to act on.** Verify the problem is real rather than
taking it on trust:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -v '^#' bench_harness/gate/declared/ring/export_surface.txt | command grep -v '^$'
```

Live output:

```
ring_factory
ring_handle
ring_tls
ring_flush
ring_types
```

`ring_config` is not there, and `build`'s only argument is a `RingConfig`. A
consumer outside the family therefore cannot name the type they must construct
to call the crate's only function. Either `ring_config` joins the surface, or
`RingConfig` moves to `ring_types`, or this crate re-exports it — three
different answers with three different blast radii, and this family's own Contract ruling addressed
which crates are reachable *through* the surface without addressing which
*types* must travel with them.

### APIs

| File | Relationship |
|------|--------------|
| [002_the_named_build_surface.md](002_the_named_build_surface.md) | O2, and why it is a second operation |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_selecting_a_backend_from_one_boolean.md](../algorithm/001_selecting_a_backend_from_one_boolean.md) | Why `S` is a parameter and the backend is not |
| [../algorithm/002_assembling_a_ring_from_a_validated_record.md](../algorithm/002_assembling_a_ring_from_a_validated_record.md) | What O1 runs |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_configuration_record_as_input.md](../data_structure/001_the_configuration_record_as_input.md) | B1's argument type, and guarantee 3's subject |
| [../data_structure/002_the_handle_pair_as_output.md](../data_structure/002_the_handle_pair_as_output.md) | O1's return |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_crate_the_export_surface_routes_through.md](../integration/002_the_crate_the_export_surface_routes_through.md) | Guarantee 3 at family grain; why P1 and P2 are not yet prevented |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_configuration_fully_determines_the_ring.md](../invariant/001_configuration_fully_determines_the_ring.md) | B1 and guarantee 2 |
| [../invariant/002_construction_is_the_only_path.md](../invariant/002_construction_is_the_only_path.md) | P1 and P2's existing status |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_way_in.md](../pattern/002_one_way_in.md) | P3 and P4 — the constructors this surface displaces |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_factory.md](../type/001_factory.md) | B3's receiver |
| [../type/002_build_error.md](../type/002_build_error.md) | B2's justification, traced refusal by refusal |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/gate/declared/ring/export_surface.txt`](../../../bench_harness/gate/declared/ring/export_surface.txt) | Guarantee 3's five names, quoted above |
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | The `Copy` record B1 takes by value |

### Tests

| File | Relationship |
|------|--------------|
| `tests/factory_test.rs` | Must name the requirement it verifies in its own header — the family's one crate→feature edge — and assert per-field behaviour rather than per-field round-trip |

### FC5 — What a Consumer Outside the Family May Rely On, and How Many There Are

The export surface is a declared file, not a convention, and it names five
crates. `ring_factory` is one of them, so everything in this instance is a
promise to somebody outside `ring_*`. The measurement that matters is how
many somebodies there are:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the declared surface --'
command grep -vE '^\s*(#|$)' bench_harness/gate/declared/ring/export_surface.txt
echo '  -- manifests naming any of the five, outside ring_* --'
command grep -rl 'ring_factory\|ring_handle\|ring_tls\|ring_flush\|ring_types' \
  --include=Cargo.toml . 2>/dev/null | command grep -vE '/ring_|\.claude/worktrees' | sed 's|^\./||' | command grep -v '^ring/Cargo.toml$'
echo '  -- and inside it --'
command grep -rl 'ring_factory = ' --include=Cargo.toml . | sed -E 's|/Cargo.toml||' | sort
```

Live output:

```
  -- the declared surface --
ring_factory
ring_handle
ring_tls
ring_flush
ring_types
  -- manifests naming any of the five, outside ring_* --
smoke_ring_write_path/Cargo.toml
  -- and inside it --
ring_bench
smoke_ring_write_path
```

The only file outside the family that names one is the workspace root manifest,
which lists members rather than depending on anything. The only crate that
depends on `ring_factory` is `ring_bench`, which is inside the family and
therefore refactorable alongside it.

So every compatibility guarantee this instance states is, today, unexercised.
That is not an argument for weakening them — a contract's value is that it holds
before the first consumer arrives, and the guarantees were written to be checked
against a consumer that does not exist yet. It is an argument about what the
tests can prove: `the_contract_surface_is_reachable_without_naming_a_non_contract_crate`
is the only thing standing in for an outside consumer, it lives inside the
family, and it exercises the surface from a crate that can see every internal it
promises not to need.

The load-bearing consequence is that a breaking change to this surface would
today break exactly one crate, and its author would fix it in the same commit —
which is precisely the condition under which a contract stops being felt as a
contract. The gate file is the only thing that would notice.

**Correction (2026-09-28):** the two sentences above the measurement read "The
only file outside the family that names one is the workspace root manifest,
which lists members rather than depending on anything. The only crate that
depends on `ring_factory` is `ring_bench`, which is inside the family and
therefore refactorable alongside it." Both undercounted, and the recipe's own
output — two blocks, both quoted above — had already said so: the second block
named `smoke_ring_write_path/Cargo.toml` beside the workspace manifest,
and the third named `smoke_ring_write_path` beside `ring_bench`. Its manifest
(checked directly) declares ordinary path dependencies on all five
export-surface crates, not a passing mention of one. So "unexercised… a
consumer that does not exist yet" and "would today break exactly one crate" are
both wrong in their strong form: a breaking change would touch `ring_bench` and
`smoke_ring_write_path` at minimum. Whether `smoke_ring_write_path`'s own
tests would catch such a break is a separate question this pass did not verify.

### FC6 — The Re-export Rule Is Derivable, Correct, and Stated Nowhere

`build` takes a `RingConfig` and returns a `Split`. Only one of those two types
is re-exported here:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what this crate re-exports --'
command grep -E '^pub use ' ring_factory/src/lib.rs
echo '  -- what the surface test file actually imports --'
command grep -E '^use ring_' ring_factory/tests/factory_test.rs
echo '  -- and which of those crates are on the Contract --'
command grep -vE '^\s*(#|$)' bench_harness/gate/declared/ring/export_surface.txt | tr '\n' ' '
echo
```

Live output:

```
  -- what this crate re-exports --
pub use ring_config::RingConfig;
pub use ring_registry::Registry;
  -- what the surface test file actually imports --
use ring_core::{Backend, Ring};
use ring_factory::{BuildError, Factory, Registry, RingConfig};
use ring_handle::Split;
use ring_types::{OverflowPolicy, RingError, WaitKind};
  -- and which of those crates are on the Contract --
ring_factory ring_handle ring_tls ring_flush ring_types 
```

The rule the two `pub use` lines follow is exact: re-export precisely those
crates a caller is forced to name that are **not** on the Contract.
`ring_config` and `ring_registry` are internal, so their types are re-exported;
`ring_handle` owns `Split` and is on the Contract, so it is not re-exported and
a caller names it directly. Applying that rule mechanically reproduces the two
lines and would reproduce them again for any type added to either signature.

It is written down nowhere. The source comments justify each `pub use`
individually, by the pending it closes, and a reader adding a third argument
type has nothing to consult but two precedents that look like special cases.

The same gap shows up one file over. `the_contract_surface_is_reachable_without_naming_a_non_contract_crate`
carries a doc comment claiming its `use` list *is* the assertion, and then
enumerates that list as "`ring_factory` and `ring_types`, both on the Contract,
and `ring_core` only for the backend control test." The file imports four
`ring_*` crates. The omitted one is `ring_handle` — the crate whose absence from
the re-export list is the whole of the rule. The claim survives (`ring_handle`
is on the Contract, so the test still asserts what it says it asserts) and the
enumeration meant to make the claim checkable is the thing that is wrong.
