# API: The Named Build Surface

### Scope

- **Purpose**: Fix the operation that builds a ring and registers it under a name, and record why it is a second function rather than an optional argument — and what it costs this crate's otherwise-stateless design.
- **Responsibility**: State the operations, their error behaviour, and the compatibility guarantees.
- **In Scope**: `build_named`; lookup; the one real error edge this crate owns.
- **Out of Scope**: The registry's own storage and concurrency (→ [`ring_registry`](../../../ring_registry/readme.md)); the unnamed path (→ [`api/001`](001_the_build_surface.md)).

### Abstract

**This operation puts a name on a ring so that something other than the
constructing code can find it.** That is the entire purpose, and it is why this
operation exists at all: the unnamed `build` hands a pair to its caller and the
ring is thereafter reachable only through that pair. A named build additionally
leaves a route in.

```rust
impl Factory
{
  pub fn build_named< S : Send >
  ( &self, cfg : RingConfig, name : &str, registry : &mut Registry< S > )
  -> Result< (), BuildError >;
}
```

**Two things this file predicted about the signature are wrong, and they are
wrong in opposite directions.**

It called this "the crate's only fallible operation". `build` is fallible too —
`ring_core` refuses `OverflowPolicy::DropOldest`
(→ [`api/001`](001_the_build_surface.md)'s B2). Fallibility turned out not to
distinguish the two paths at all; only the *variant set* does.

It called this "its only stateful one", and the state is now **the caller's**.
The registry is an argument, not a field: `Factory` stays fieldless, and a ring's
lifetime is the registry's rather than the factory's
(→ [`decisions/001`](../decisions/001_the_owner_is_the_return_value.md)).

**The return type is `()`**, not a handle pair. The ring went into the registry,
which owns it; `Registry::get_mut` lends it back. Returning a pair as well would
mean handing out a producer for a ring the registry also holds — breaking the
producer/consumer partition this family's split exists to create.

`ring_registry` and `ring_handle` are both among this crate's five declared
dependencies, so the crate defining `build`'s return type is now named directly
rather than reached through the crate that names rings:

```sh
cd "$(git rev-parse --show-toplevel)"
cargo tree -p ring_factory --no-dedupe | command grep -c 'ring_handle'
for m in ring_*/Cargo.toml; do
  command grep -q '^ring_handle = ' "$m" && basename "$( dirname "$m" )"
done
```

Live output:

```
2
ring_factory
ring_registry
```

**Expected: `2`, then `ring_factory` and `ring_registry`.** The `2` is two
*paths*, not two crates — `--no-dedupe` reprints every route, and `ring_handle`
is reachable both directly and beneath `ring_registry`, which also depends on it.
Two dependents in the family is the number that carries the meaning.

**Both numbers were `1` before the fix**, and that was the anomaly: the crate
defining this crate's return type was reachable *only* through the crate that
names rings — an accident of the manifests, not a design, so removing
`ring_registry` in a plausible refactor would have broken `build`'s signature for
a reason recorded in neither manifest. ✅ **Fixed** by declaring `ring_handle`
directly. Re-run the recipe rather than trusting either number
(→ [`integration/001`](../integration/001_declared_edges_and_the_reached_closure.md)'s
requirement 1).

### Operations

| # | Operation | Signature shape | Notes |
|---|-----------|-----------------|-------|
| R1 | Build and register | `fn build_named< S : Send >( &self, cfg : RingConfig, name : &str, registry : &mut Registry< S > ) -> Result< (), BuildError >` | Fails on a taken name **or** an unsupported policy; must not leave a ring behind when it does — the observable half asserted by `a_refusal_drops_nothing_that_was_already_registered`, the unobservable half held structurally (→ [`type/002`](../type/002_build_error.md)'s V2) |
| R2 | Look up by name | ~~`fn get< S >( &self, name : &str ) -> Option< HandlePair< S > >`~~ | ✅ **Settled: it belongs on the registry**, which is re-exported here. Neither a facade nor a Contract breach — `pub use ring_registry::Registry;` → [`decisions/001`](../decisions/001_the_owner_is_the_return_value.md) |

**R2 is the operation that decides what this crate is.** If lookup lives here,
`Factory` is a facade over the registry and the export surface needs nothing
more. If lookup lives on `ring_registry`, then a consumer who needs it must
name `ring_registry` — which this family's own Contract ruling explicitly rules out, saying
the named registry is reached *through* the surface. Recorded in
[`decisions/`](../decisions/readme.md).

#### Why not `build( cfg, Some( name ) )`

| # | Reason | Weight |
|---|--------|--------|
| S1 | It forces `None` onto every unnamed call site, which is the majority path | Ergonomic, minor |
| S2 | It forces `Result` onto the unnamed path, which cannot fail — undoing [`api/001`](001_the_build_surface.md)'s B2 | **Decisive** |
| S3 | `build_named` is greppable; `Some( … )` at a call site is not. Auditing which rings are registered becomes a search for a name rather than a pattern | Practical |
| S4 | The two operations have different invariant obligations — R1 must satisfy the refuse-never-alter proviso (→ [`lifecycle/004`](../lifecycle/004_name_state_through_a_registration.md)) and `build` has nothing to satisfy | Structural |

**S2 is the one that settles it.** Every unnamed caller would pay an error
channel for a failure that cannot occur on their path, and the habitual
`.unwrap()` that follows would then silently cover the one case where the error
is real.

### Error Handling

| Condition | Result |
|-----------|--------|
| Name is already registered | `Err( BuildError::NameTaken )`, **and no ring is reachable afterwards** |
| Name is empty | Open. Nothing forbids `""` today and nothing gives it meaning |
| Two threads register the same name concurrently | Exactly one succeeds. The registry owns this; this crate must not add a check-then-insert of its own |
| The config is invalid | Cannot occur — same as [`api/001`](001_the_build_surface.md)'s table |
| Lookup finds nothing | `None`, not an error — an absent name is an ordinary answer |

**The concurrent row is the one with a wrong implementation available.** A
`if registry.contains( name ) { return Err( … ) }` followed by an insert is
the natural spelling and is a time-of-check-to-time-of-use race: two threads
both see the name free, both insert, and one silently replaces the other's
ring. The correct shape is a single atomic insert-if-absent on the registry,
which means this crate must not make the decision at all — it must ask once and
act on the answer.

**The empty-name row is a real gap, not padding.** A registry keyed by `&str`
accepts `""` as readily as any other key, and a config-driven harness that
leaves a name field blank registers under it. That is the same shape as
[`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)'s C4
— an unset field becoming a valid value — and it is unresolved for the same
reason: nothing distinguishes absent from empty.

### Compatibility Guarantees

1. **A failed `build_named` leaves nothing registered and nothing reachable.**
   The strongest guarantee here, and the one a naive implementation breaks by
   constructing first and registering second without a cleanup path.
2. **`build_named` never replaces an existing registration.** Silent
   replacement is the failure mode a name exists to prevent; if replacement is
   ever wanted it is a differently-named operation.
3. **The error type stays one variant wide, or grows by addition only.**
   `BuildError` is not `#[ non_exhaustive ]` (→ [`type/001`](../type/001_factory.md)),
   so a new variant is a breaking change inside this workspace — acceptable,
   and worth noticing rather than discovering.
4. **R1 and `build` produce indistinguishable rings for equal configs.**
   Registration may refuse the whole call; it may not shape the ring
   (→ [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)'s V3).
5. **No public signature names a crate outside the Contract's five** — which
   `RingConfig` already violates, per [`api/001`](001_the_build_surface.md)'s
   guarantee 3, and `HandlePair` does not, since `ring_handle` is on the
   surface.

**Guarantee 4 is the subtle one and it is what keeps this operation from
becoming a second input.** The registry is consulted, so information flows from
shared state into the call — and the guarantee restricts that flow to a single
bit that either aborts the call or does nothing. Any wider flow makes
`invariant/001` false.

### APIs

| File | Relationship |
|------|--------------|
| [001_the_build_surface.md](001_the_build_surface.md) | The unnamed path S2 protects |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_handle_pair_as_output.md](../data_structure/002_the_handle_pair_as_output.md) | What R1 returns and R2 would return |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_declared_edges_and_the_reached_closure.md](../integration/001_declared_edges_and_the_reached_closure.md) | `ring_registry` as the sole route to `ring_handle` |
| [../integration/002_the_crate_the_export_surface_routes_through.md](../integration/002_the_crate_the_export_surface_routes_through.md) | R2's placement question, at family grain |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_configuration_fully_determines_the_ring.md](../invariant/001_configuration_fully_determines_the_ring.md) | Guarantee 4, and V3 — the registry as a hidden second input |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_factory_outlives_nothing.md](../lifecycle/002_the_factory_outlives_nothing.md) | What a registered ring's lifetime becomes, which is the one case the factory does outlive something |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_name_state_through_a_registration.md](../lifecycle/004_name_state_through_a_registration.md) | Guarantee 1's ordering, as transitions |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_factory.md](../type/001_factory.md) | N2 — the operation that might give the factory a field |
| [../type/002_build_error.md](../type/002_build_error.md) | The one variant, and guarantee 3 |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/gate/declared/ring/export_surface.txt`](../../../bench_harness/gate/declared/ring/export_surface.txt) | Guarantee 5's five names |

### Tests

| File | Relationship |
|------|--------------|
| `tests/factory_test.rs` | Register twice; assert `Err( NameTaken )` and that the first ring is still the one the name resolves to. Guarantee 1 needs its own assertion — the error alone does not prove nothing was left behind |

### FC7 — The Contract Is Five Crates Wide by Declaration and Six Wide in Use

`build_named` returns `()`. The ring it built is reachable only through the
registry the caller passed, so using this operation at all means calling
`Registry`'s methods — and `Registry` belongs to a crate the export surface
explicitly classifies as internal and freely refactorable:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- Registry public API --'
command grep -E '^  pub (const )?fn ' ring_registry/src/lib.rs | sed 's/(.*//'
echo '  -- which of them this crate own tests must call --'
command grep -oE 'registry\.[a-z_]+\(' ring_factory/tests/factory_test.rs | sort -u
printf '  ring_registry on the export surface: %s\n' \
  "$( command grep -c '^ring_registry$' bench_harness/gate/declared/ring/export_surface.txt )"
```

Live output:

```
  -- Registry public API --
  pub fn new
  pub fn register
  pub fn get_mut
  pub fn remove
  pub fn contains
  pub fn len
  pub fn is_empty
  pub fn names
  -- which of them this crate own tests must call --
registry.contains(
registry.get_mut(
registry.is_empty(
registry.len(
registry.names(
registry.remove(
  ring_registry on the export surface: 0
```

Eight public methods, six of them exercised here, and the crate that owns all
eight is one of the twenty-eight the surface file says are "internal to the
family and freely refactorable."

The re-export does not fix this. `pub use ring_registry::Registry` makes the
*name* reachable through a Contract crate; it does not move the *methods* onto
the Contract, and an outside consumer of `build_named` is bound to their
signatures exactly as if they had written the dependency themselves. Renaming
`get_mut` would be a legal internal refactor by the surface file's own words and
a breaking change for every caller of this operation.

This is the sharpest reading of this family's own Contract ruling that the named
registry is "reached *through* this surface": reaching a registry through the surface
puts the registry's API on the surface, whatever the file lists. The narrow fix
is a line in `export_surface.txt` — the wide one is that `build_named` should
return something this crate owns. Neither is a decision this instance can make,
and the gap is currently invisible to the gate, because the gate checks the
declared names and has nothing to say about what a declared name transitively
requires.

### FC8 — The One Operation With Its Own Error Is the One That Cannot Report Which Half Failed

`build_named` is `self.build( cfg )?` followed by a registration. Both halves
are fallible, and the two failures collapse into one two-variant enum whose
variants do not partition by half — `Unsupported` can only come from the build,
but a caller matching on it learns that from the documentation, not the type:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two exits --'
command grep 'self.build( cfg )?\|Err( BuildError::NameTaken )' ring_factory/src/lib.rs
echo '  -- what the caller can distinguish --'
# anchored on the enum body, not on a fixed -A window: three doc-comment lines
# sit above each variant, so any window wide enough for the first is a guess
sed -n '/^pub enum BuildError/,/^}/p' ring_factory/src/lib.rs | command grep -vE '^\s*(///|$)'
```

Live output:

```
  -- the two exits --
    let split = self.build( cfg )?;
      Err( ( RegistryError::NameTaken { .. }, _refused ) ) => Err( BuildError::NameTaken ),
  -- what the caller can distinguish --
pub enum BuildError
{
  NameTaken,
  Unsupported( RingError ),
}
```

The ordering guarantee — nothing is registered when `Unsupported` is returned,
because the ring is never built — holds and is worth having. What it costs is
that the `?` discards the only place the two phases are separable. A caller who
wants to retry under a different name on `NameTaken` and give up on
`Unsupported` can do it; a caller who wants to know whether the registry was
touched has to infer it from which variant arrived, and that inference is a
documented property of the current implementation rather than of the type.

`the_unnamed_path_can_never_return_name_taken` asserts the other direction —
that `build` never produces `NameTaken` — which is the half the compiler could
have enforced with two error types. That test exists because the type does not
say it. One enum for both doors is the right call for a two-function crate; the
finding is that it makes the crate's only genuinely sequenced operation report
its failures as though they were unordered.

Each variant's own doc comment now states which phase produces it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -B1 -A10 '^pub enum BuildError' ring_factory/src/lib.rs
```

Live output:

```
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub enum BuildError
{
  /// A ring is already registered under this name.
  ///
  /// Reachable only from [`Factory::build_named`], and only after the build
  /// half of that call already succeeded: the two variants partition by
  /// phase, so this one means a ring was built and then dropped because
  /// registration refused it, never that the build itself failed.
  NameTaken,
  /// The backend cannot honour the configured overflow policy.
  ///
```

**Disposition:** applied — `BuildError::NameTaken` and
`BuildError::Unsupported`'s own doc comments in `ring_factory/src/lib.rs`
now state the phase-partition explicitly: `NameTaken` means the build half of
`build_named` already succeeded and only registration refused, `Unsupported`
means the build half itself failed and the registry was never reached. What
was previously "a documented property of the current implementation" (a test
and a code comment) is now stated on the type the caller actually matches on;
the crate's test suite re-verified passing (`cargo test -p ring_factory
--all-features`, 2026-09-04). Now prints: `and only after the build`
