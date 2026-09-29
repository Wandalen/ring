# Integration: The Crate the Export Surface Routes Through

### Scope

- **Purpose**: Record what this family's own Contract ruling obliges of this crate specifically — three capabilities reachable only through `build` — and what currently stops that obligation from being enforceable.
- **Responsibility**: State the system description, integration points, error handling, and compatibility requirements.
- **In Scope**: The five-name Contract; gate G5's vacuity; the three routed features; the type-versus-crate gap in the ruling.
- **Out of Scope**: This crate's own dependency anomalies (→ [`integration/001`](001_declared_edges_and_the_reached_closure.md)); the internal construction leaks (→ [`invariant/002`](../invariant/002_construction_is_the_only_path.md)).

### System Description

**Thirty-three crates, five of which a consumer outside the family may name.**

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

The file's own header states the ruling it implements:

> Ruled by docs/decision/121_workstream_008_contract_gaps_ruled.md § 4, which
> upholds the five-crate Contract set by decision/120 and rules that features
> 171 (SPSC), 172 (MPSC) and 181 (registry) are reached *through* this surface
> rather than by importing ring_spsc, ring_mpsc or ring_registry directly:
> ring_factory constructs them and hands back ring_handle values.

**Three features are routed and all three route through this crate.** That is
the whole of this instance's subject: `ring_factory` is not merely one of five
exported names, it is the *only* one that exists to make other crates
unnecessary.

| Capability | Internal crate | How it is reached |
|------------|---------------|-------------------|
| SPSC ring | `ring_spsc` | `build( cfg )` with `producers == 1` |
| MPSC claim | `ring_mpsc` | `build( cfg )` with `producers > 1` |
| Named registry | `ring_registry` | `build_named( cfg, name, registry )` to register; `Registry::get_mut` to look up — **via `pub use ring_registry::Registry;`**, not a method on this crate |

**The other four Contract names route nothing.** `ring_handle` is a thing a
consumer holds; `ring_tls` is a thing they hold; `ring_types` is vocabulary;
`ring_flush` is a decision they make. Only this one is a door.

### Integration Points

| # | Point | What routes through it |
|---|-------|------------------------|
| X1 | `build( cfg )` | The SPSC and MPSC paths, selected by one field |
| X2 | `build_named( cfg, name )` | The named registry's registration half |
| X3 | Lookup by name | The named registry's *other* half — ✅ **placed: the re-exported `Registry`** |
| X4 | Gate G5 | The mechanism that would make X1–X3 the only routes |

**X3 was called the gap in the ruling, and it was a false dilemma.** The
argument ran: this family's own Contract ruling covers registration via X2 but never mentions
lookup, so either lookup comes here as a method or the ruling is incomplete.
Both horns assume that reaching the named registry *through* this crate means
*reimplementing* its surface here. It does not — `pub use ring_registry::Registry;`
routes the name through this crate's door while leaving the one tested
implementation of `get_mut` where it is. **Reached through** is satisfied by a
re-export; the ruling was complete and this instance read it as narrower than it
is. ✅ [`decisions/001`](../decisions/001_the_owner_is_the_return_value.md).

**The generalisation worth keeping: "route X through Y" has two readings, and
the expensive one gets assumed.** Reimplementation (a forwarding method, a
second place to be wrong, a signature to keep in step) versus re-export (one
line, no second implementation). Nothing in § 4's wording chose between them and
this instance did not notice there was a choice — the same shape of miss as
[`data_structure/002`](../data_structure/002_the_handle_pair_as_output.md)'s
two-owners-where-three-exist.

**X4 does not currently work, for a documented reason.** The gate skips
family-internal manifests:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -F 'grep -qx -- "$owner" && continue' bench_harness/gate/g5_export_surface.sh
```

Live output:

```
  printf '%s\n' "${members[@]}" | grep -qx -- "$owner" && continue
```

which is correct — internal crates must depend on each other — and leaves the
gate with nothing to check, because
[`bench_harness`'s own invariant](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md)
records that no crate outside the family depends on any `ring_*` crate yet. **The
gate passes because there are no consumers, not because consumers are
compliant.**

**So the routing obligation is currently unenforced in both directions:**
nothing outside the family is bypassing the surface (there is nothing outside
the family), and nothing inside it is stopped from adding a bypass
(→ [`invariant/002`](../invariant/002_construction_is_the_only_path.md)'s L1 and
L2, which are already public).

#### The gap the ruling did not reach: crates versus types

This family's own Contract ruling rules on which **crates** may be named. It does not rule on
which **types** must be nameable, and those are different questions with a
concrete consequence here:

| Type | Defining crate | On the surface | Needed by a consumer |
|------|---------------|----------------|----------------------|
| `Factory` | `ring_factory` | Yes | To call anything |
| `Producer`, `Consumer` | `ring_handle` | Yes | To use the result |
| `RingConfig` | `ring_config` | ✅ **Re-exported** | **To call `build` at all** |
| `WaitKind`, `OverflowPolicy`, `Capacity` | `ring_types` | Yes | To build a config |
| `BuildError` | `ring_factory` | Yes | To handle X2's failure |

**One row failed, and it was the argument to the crate's only function.** A
consumer could name `ring_factory`, name `ring_handle`, name every policy enum
in `ring_types` — and could not name the type that carries them into `build`.
✅ **Closed by W3.**

Three resolutions, with different blast radii:

| # | Resolution | Cost |
|---|-----------|------|
| W1 | Add `ring_config` to the surface — six names, not five | Widens the Contract, which the surface file's header explicitly warns against without a decision |
| W2 | Move `RingConfig` into `ring_types` | `ring_types` becomes more than vocabulary; `ring_config` is left holding nothing, or is deleted |
| **W3 ✅ taken** | Re-export `RingConfig` from `ring_factory` | No Contract change. Two paths to one type, and the `ring_config` name still appears in `cargo tree` output and error messages |

**W3 is the smallest and W2 is the most honest**, and the smallest was taken.
`RingConfig` is a record of `ring_types` values with no behaviour beyond
validation and clamping — the case for it being vocabulary rather than a crate
of its own is not weak, and W2 remains the better long-term shape. It is not
taken because it changes two shipped, tested crates to buy a property W3 already
delivers. → [`decisions/001`](../decisions/001_the_owner_is_the_return_value.md).

**W3's cost arrived exactly as described and one item was missed.**
`RingConfig::with_overflow` takes an `OverflowPolicy`, which `ring_config` does
not re-export — so a consumer sets one field by naming two crates. `ring_types`
*is* on the Contract, so this is legal rather than a violation, and a third
re-export here would give the family two paths to that type as well. The
table above listed W3's costs as Contract-neutrality and duplicate paths; the
cost that actually bites is that **a re-export covers a type and not its
argument types**, and nothing prompted a check of what the re-exported type's
own methods require.

### Error Handling

| # | Condition | Reported |
|---|-----------|----------|
| X1 | A consumer imports `ring_spsc` directly | **Nothing today.** G5 would catch it once a consumer exists |
| X2 | A consumer calls `Ring::with_config` instead of `build` | **Nothing, ever.** G5 checks manifests, not call sites — and once `ring_spsc` is a legal dependency for anyone, every constructor on it is legal too |
| X3 | ~~A consumer cannot name `RingConfig`~~ | ✅ **Cannot occur** — re-exported. Asserted by `the_contract_surface_is_reachable_without_naming_a_non_contract_crate` |
| X4 | ~~Lookup is needed and lives on `ring_registry`~~ | ✅ **Cannot occur** — `Registry` is re-exported, so lookup is reachable without naming that crate |

**X2 is the limit of what a manifest gate can do, and it is worth stating
plainly.** G5's granularity is the dependency edge. It can say "you may not
depend on `ring_spsc`"; it cannot say "you may depend on `ring_core` but only
call the three functions we meant." Every routing guarantee this instance
describes is therefore crate-grained, and the internal leaks in
[`invariant/002`](../invariant/002_construction_is_the_only_path.md) are outside
its reach by construction rather than by oversight.

**X3's error message was the practical harm and the harm is gone**, but the
prescription that followed it is not discharged. It said the reason belongs in
this crate's own doc comment, "because that is where a confused consumer will
actually look." What the doc comment now carries is the *behaviour* — what
`build` refuses, why there are two doors — and not the fact that two of the
names it exports are re-exports serving the Contract. A consumer reading
`pub use ring_config::RingConfig;` in the rustdoc sees a re-export and no reason
for it. **Fixing the error message removed the prompt to explain the design**,
which is the ordinary way a documentation obligation gets quietly dropped:
nothing fails, and nobody is confused loudly enough to notice.

### Compatibility Requirements

1. ✅ **This crate's public surface names only the five.** Was violated by
   `RingConfig`; closed by W3's re-export — requirement 2 of
   [`integration/001`](001_declared_edges_and_the_reached_closure.md). Asserted
   by `the_contract_surface_is_reachable_without_naming_a_non_contract_crate`,
   which is a compile-time check written as a test.
2. ✅ **Every capability this crate routes — SPSC, MPSC and the named registry —
   is reachable through this crate.** The named registry's lookup half is reachable via the re-exported
   `Registry`. **With one honest exception:** the MPSC backend is
   selected by `producers > 1` and nothing in the returned `Split` reports which
   backend was chosen, so "reachable" here means constructible, not observable
   (→ [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)).
3. **Widening `export_surface.txt` requires a decision**, per the file's own
   header. W1 is therefore not a unilateral fix.
4. **G5's vacuity is a state, not a permission.** The first crate outside the
   family to depend on any `ring_*` crate makes the gate meaningful; nothing
   about this crate should be built on the assumption that it stays vacuous.
5. **Routing is crate-grained and cannot become finer.** Any guarantee that
   requires call-site granularity needs a different mechanism — a lint, a
   `#[ doc( hidden ) ]`, or a sealed trait — and none is currently in the family.

**Requirement 4 is the one that is easy to forget.** It is tempting to treat the
current freedom — any family crate may be depended on, no gate objects — as the
steady state. It is a consequence of the family having no consumers yet, and
the family exists precisely to acquire one.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | X1, and guarantee 3 — the `RingConfig` problem |
| [../api/002_the_named_build_surface.md](../api/002_the_named_build_surface.md) | X2 and X3; R2 is the unplaced lookup |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_selecting_a_backend_from_one_boolean.md](../algorithm/001_selecting_a_backend_from_one_boolean.md) | How X1 serves both 171 and 172 with one call |

### Integrations

| File | Relationship |
|------|--------------|
| [001_declared_edges_and_the_reached_closure.md](001_declared_edges_and_the_reached_closure.md) | The dependency surface beneath this Contract view |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_construction_is_the_only_path.md](../invariant/002_construction_is_the_only_path.md) | M1 and M2 — the same vacuity, seen as a failed enforcement mechanism |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_one_way_in.md](../pattern/002_one_way_in.md) | E1 — the export surface as this pattern's only mechanical half |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_build_error.md](../type/002_build_error.md) | V3 — the Contract rule at payload grain |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/gate/declared/ring/export_surface.txt`](../../../bench_harness/gate/declared/ring/export_surface.txt) | The five names and the § 4 quotation |
| [`bench_harness/gate/g5_export_surface.sh`](../../../bench_harness/gate/g5_export_surface.sh) | Line 22 — X2's granularity limit |
| [`bench_harness/docs/invariant/001_gate_non_vacuity.md`](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md) | X4's vacuity, recorded at gate grain |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | Requirement 1 was described as needing a fixture crate rather than a test function. It did not: `the_contract_surface_is_reachable_without_naming_a_non_contract_crate` imports only Contract names and builds, registers and retrieves a ring — the compiler checks it, and a test file that never mentions `ring_config` or `ring_registry` is evidence of the same kind a fixture crate would give, at none of the cost |

### FC19 — G5's Confinement Half Cannot Fail, and the Half That Can Is Not the One This Instance Is About

G5 got a non-vacuity pairing after plan 009 caught it reporting REACHED against
an unwritten family, and the pairing works — the hollow check grades whether each
declared crate exports anything at all. The half that enforces *confinement*
still scans a set that is empty:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what the confinement half scans: manifests outside the family --'
command grep -rl 'ring_factory\|ring_handle\|ring_tls\|ring_flush\|ring_types\|ring_core\|ring_config\|ring_registry' \
  --include=Cargo.toml . 2>/dev/null | command grep -vE '/ring_|\.claude/worktrees' | sed 's|^\./||' | command grep -v '^ring/Cargo.toml$'
echo '  -- the two halves, in the gate itself --'
command grep 'hollow+=\|violations+=\|declared export surface is empty' \
  bench_harness/gate/g5_export_surface.sh
```

Live output:

```
  -- what the confinement half scans: manifests outside the family --
smoke_ring_write_path/Cargo.toml
  -- the two halves, in the gate itself --
[ ${#allowed[@]} -gt 0 ] || fail "declared export surface is empty"
  [ "${n:-0}" -gt 0 ] || hollow+=( "$c" )
# member name and so violations+= at line 74 never fires — an undeclared
    printf '%s\n' "${allowed[@]}" | grep -qx -- "$dep" || violations+=( "${manifest#"$REPO"/}: $dep" )
```

One file, and it is the workspace root manifest listing members rather than
depending on anything. Every other manifest in the repository that names a
family crate is itself a family crate, and the gate skips those by design — "the
surface bounds what leaves the family, not what moves inside it."

So `violations` is empty by construction, not by compliance. Adding a sixth name
to `export_surface.txt`, or removing four of the five, changes nothing the
confinement half reports. The gate is honest about this in its own comments
(the vacuity it names is a real one it fixed) and the fix addressed substance
rather than reach — a declared surface with no external consumer is exactly as
green as a correct one.

That is not an argument for deleting the gate. It is the reason the surface's
teeth are still theoretical, which is what this instance was written to record,
now measured rather than predicted: the check that would bite arrives with the
first consumer, and there is not one yet
(→ [`api/001`](../api/001_the_build_surface.md) FC5).

**Correction (2026-09-28):** "One file, and it is the workspace root manifest
listing members rather than depending on anything" undercounted, for the same
reason as api/001 FC5 — the recipe's own output, quoted above, already named a
second file, `smoke_ring_write_path/Cargo.toml`. The workspace root
manifest's member list has since moved to `ring/Cargo.toml`, itself a
family-internal workspace file rather than an outside one, so it is now
excluded from this census the same way every individual family crate's
manifest already was — leaving `smoke_ring_write_path/Cargo.toml` as the
one file this half actually scans. Unlike the root manifest, it declares
ordinary path dependencies on family crates (`ring_config`, `ring_core`,
`ring_types`, `ring_tls`, `ring_flush`, `ring_factory`, `ring_handle` — checked
directly), so "lists members rather than depending on anything" described the
wrong file. → api/001 FC5's "there is not one yet" is corrected there for the
same reason: `smoke_ring_write_path` is an existing outside-family
consumer, not a hypothetical future one.

### FC20 — Eight Empty `Error` Impls, Five Wrapping Variants, and No Chain Longer Than One

`BuildError` carries a `RingError` and its `Display` interpolates it. Its
`Error` impl is `{}`, so `source()` takes the default and returns `None`:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every Error impl in the family --'
command grep -r 'impl .*core::error::Error for' --include=*.rs ring_*/src/ | sed 's|ring/||; s|\.rs:|.rs:  |'
printf '  source() implementations, family-wide: %s\n' \
  "$( command grep -rc 'fn source' --include=*.rs ring_*/src/ | awk -F: '{ n += $NF } END { print n + 0 }' )"
```

Live output:

```
  -- every Error impl in the family --
ring_bench/src/lib.rs:  impl core::error::Error for WorkloadError {}
ring_bench/src/lib.rs:  impl core::error::Error for RunError {}
ring_debug/src/lib.rs:  impl core::error::Error for Violation {}
ring_factory/src/lib.rs:  impl core::error::Error for BuildError {}
ring_flush/src/lib.rs:  impl core::error::Error for ConfigError {}
ring_registry/src/lib.rs:  impl core::error::Error for RegistryError {}
ring_testkit/src/lib.rs:  impl core::error::Error for Anomaly {}
ring_types/src/error.rs:  impl core::error::Error for RingError {}
  source() implementations, family-wide: 0
```

Eight impls, every one an empty body, and not a single `source()` anywhere in
the thirty-three crates. Five error variants across the family wrap another
family error — `BuildError::Unsupported` is one of them — so five chains exist
in the data and none of them are walkable through the trait that exists to walk
them.

The practical consequence is specific rather than stylistic. A consumer holding
a `dyn Error` from this crate and iterating `source()` gets one link and stops,
while `to_string()` on the same value renders the wrapped `RingError`'s own
words. So the payload is present in the message and absent from the chain, and a
handler that branches on the root cause — the ordinary reason to call
`source()` — sees `BuildError` as the root of a chain whose real root is one
crate down.

Adding `fn source` to `BuildError` is three lines and would fix this crate's
link. The finding is that it would be the only one in the family, and the
consistency argument then runs the wrong way: eight empty impls look deliberate,
and nothing records whether they are.
