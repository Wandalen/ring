# Integration: The Registry That Declined the Shared Error

### Scope

- **Purpose**: Document `ring_registry`'s exception to "every crate depends on tier 0" — it reaches this crate transitively, declares no edge to it, and defines its own error type for the failure two of this crate's variants were written to carry.
- **Responsibility**: Describe the system, name the integration points, and state the compatibility requirements.
- **In Scope**: The missing manifest edge; what `RegistryError` carries that `RingError` cannot; what the export Contract sees.
- **Out of Scope**: The dead variants as a trap (→ [`pitfall/002`](../pitfall/002_two_name_errors_nothing_constructs.md)); the registry's own API (→ [`ring_registry`](../../../ring_registry/readme.md)).

### System Description

**Thirty crates declare `ring_types`. `ring_registry` is the one whose absence
has a story** — it is not a crate on the family's periphery, it is the storage
behind the named-ring registry, re-exported through `ring_factory`, which is
on the five-crate export Contract.

```sh
cd "$(git rev-parse --show-toplevel)"
for m in ring_*/Cargo.toml; do
  command grep -q '^ring_types' "$m" || basename "$( dirname "$m" )"
done
```

Live output:

```
ring_align
ring_registry
ring_types
```

**Correction (2026-09-28):** the list above previously held two names,
`ring_registry` and `ring_types`. `ring_align` has since dropped `ring_types`
from its own `[dependencies]` — commit `ce60ae6e8` ("Remove unused
dependencies from Cargo.toml files") — for reasons unrelated to this file's
subject: it has no error type of its own and no `Copy`-versus-payload story,
it simply stopped referencing any of the six exported names. The fuller
accounting of who depends on this crate lives in
[`integration/001`](001_the_crate_thirty_one_of_thirty_three_depend_on.md);
this file's scope stays `ring_registry` specifically.

**The absence is a manifest fact, not a reachability one.** `ring_types` is in
the registry's build closure already, arriving through
`ring_handle → ring_core`:

```sh
cd "$(git rev-parse --show-toplevel)"
cargo tree -p ring_registry --prefix none --no-dedupe | sort -u | command grep '^ring_types'
```

Live output:

```
ring_types v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_types)
```

Rust requires a direct dependency before a crate may `use` another, so the
single missing `Cargo.toml` line is the whole barrier — and it is one line. The
registry is not working around an unavailable type; it declined an available
one.

**What it wrote instead differs in exactly one respect, and that respect is the
reason.** Both enums have a `NameTaken` variant; only one carries the name:

| | `ring_types::RingError::NameTaken` | `ring_registry::RegistryError::NameTaken` |
|---|---|---|
| Payload | none | `name : String` |
| Enum size | 24 bytes (set by a sibling variant) | 24 bytes |
| `Copy` | Yes — required | No — a `String` forbids it |
| Allocates | Never | On construction |
| Display | "a ring is already registered under this name" | `a ring is already registered as "telemetry"` |
| Constructed at | nowhere | `ring_registry/src/lib.rs:172` |

**The divergence is forced by a constraint this crate cannot relax.**
`RingError` is `Copy` and allocation-free because an error returned from the
tick path must not allocate — stated in its own doc comment, asserted by
`error_is_copy`. Registration is not on the tick path: it happens once, at
setup, where an allocation is free and a name in the message is worth having.
**The two crates optimise for opposite ends of the same axis, and both are
right for their own end.**

**This is the counterexample to `error.rs`'s opening argument.** That module
opens by ruling out per-crate error types on the grounds that they "would have
to be converted into a shared one at the surface anyway". The prediction was
that conversion is a formality; here it is lossy, because the shared type has no
field to convert *into*.

### Integration Points

| # | Point | Direction | State |
|---|-------|-----------|-------|
| P1 | `ring_registry` → `ring_types` (manifest) | Declared edge | ❌ **Absent.** One line, never added |
| P2 | `ring_registry` → `ring_types` (build closure) | Transitive, via `ring_handle → ring_core` | ✅ Present, and unusable without P1 |
| P3 | `ring_registry` → `ring_handle` | Declared edge | ✅ The registry's only declared dependency |
| P4 | `ring_factory` re-exports `Registry` | Contract-facing | ✅ How the registry reaches an external consumer |
| P5 | `RegistryError` → external consumer | Contract-facing | ⚠️ **Reaches a consumer through a crate that is not on the Contract** |
| P6 | `RingError::NameTaken` / `NameUnknown` | Declared, unconstructed | ❌ Dead in every crate |

**P5 is the seam this instance exists to name.** The export Contract lists five
crates an external consumer may import: `ring_factory`, `ring_handle`,
`ring_tls`, `ring_flush`, `ring_types`. `ring_registry` is not among them —
the registry is reached *through* `ring_factory`, which re-exports `Registry`.
But `Registry::register` returns `Result< (), ( RegistryError, Split< T > ) >`,
and `RegistryError` is declared in `ring_registry`. So a Contract-bound consumer
handling a registration conflict must name a type from a crate the Contract does
not let them import.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep '^pub use' ring_factory/src/lib.rs
command grep 'pub fn register' -A 6 ring_registry/src/lib.rs | command grep 'Result'
```

Live output:

```
pub use ring_config::RingConfig;
pub use ring_registry::Registry;
  -> Result< (), ( RegistryError, Split< T > ) >
```

**Two re-exports, and the error type is not one of them.** The signature a
consumer must handle names `RegistryError`; the door they are allowed through
hands them `Registry` and stops.

**The parallel case was already solved once in this family.** `ring_factory`
re-exports `RingConfig` for exactly this reason — a Contract-bound consumer
could not otherwise name `build`'s only argument. `RegistryError` is the same
shape of problem, one type later, and the same fix applies.

**P6 is what makes P5 avoidable-looking and not actually avoidable.** The
obvious response — "the registry should return `RingError::NameTaken`, which is
already on the Contract" — is P1 plus a payload deletion, and the payload is the
variant's entire value.

### Error Handling

**The family currently has two error types, and the boundary between them is
undocumented.** A consumer sees:

| Operation | Reached via | Error type |
|-----------|-------------|-----------|
| `Factory::build` | `ring_factory` | `BuildError`, wrapping `RingError` |
| `Factory::build_named` | `ring_factory` | `BuildError` |
| `Registry::register` | `ring_factory`'s re-export of `Registry` | `( RegistryError, Split< T > )` |
| `Registry::get_mut` / `remove` | same | `Option` — absence is not an error |
| Every ring operation | `ring_handle` | `RingError` |

**Three error shapes across one Contract, and only one of them is `Copy`.** A
consumer writing a single `From` impl to funnel failures into their own type
needs three, and the registry's carries a tuple whose second element is the ring
they were trying to register — a deliberate design (the ring is handed back
rather than dropped) that no `From` impl can preserve.

**The absent name lookup is the quiet half.** `RingError::NameUnknown` exists;
`Registry::get_mut` returns `Option`. Nothing is wrong with either, but a
consumer reading the error enum for "what can go wrong with a named ring" finds
a variant describing a failure the API models as `None`. The error set
advertises a shape the API does not have.

### Compatibility Requirements

| # | Requirement | Currently | If violated |
|---|-------------|-----------|-------------|
| R1 | A Contract-bound consumer can name every type in a Contract signature | ❌ **Broken by `RegistryError`** | The consumer imports a non-Contract crate, widening the Contract by use rather than by decision |
| R2 | `RingError` stays `Copy` | ✅ Held | The tick path allocates on failure |
| R3 | Adding P1 does not become a silent way to converge the two errors | ⚠️ Unguarded | The registry loses the name from its message, for consistency's sake |
| R4 | The Contract's crate list matches what its signatures actually require | ❌ Broken, same cause as R1 | Gate G5 passes while the real surface is six crates wide |
| R5 | `NameTaken`/`NameUnknown` are either used or marked reserved | ❌ Neither | A consumer writes a dead match arm and gets no warning (`#[ non_exhaustive ]` absorbs it) |

**R1 and R4 are the same defect counted twice — once against the consumer and
once against the gate.** The fix is one line in `ring_factory`:
`pub use ring_registry::RegistryError;`, matching the `RingConfig` and
`Registry` re-exports already there. That is a `ring_factory` change, not a
`ring_types` one, which is why this instance records the requirement and does
not claim the fix.

**R3 is the requirement worth writing down before it is tested.** The natural
reading of this instance is "add the missing edge and unify the errors" — and
that is the one change that makes things worse, because it trades a real
payload for a consistency that no consumer asked for. **The edge and the
unification are separable, and only the second is harmful.**

**R5 is the cheapest of the five and closes the most confusion.** Two rustdoc
clauses saying no crate constructs these, and the registry uses its own type.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | The six names, and the Contract question P5 raises about them |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) | R2 — the `Copy` constraint, and the 24 bytes that make the payload question moot |

### Integrations

| File | Relationship |
|------|--------------|
| [001_the_crate_thirty_one_of_thirty_three_depend_on.md](001_the_crate_thirty_one_of_thirty_three_depend_on.md) | The thirty that did declare the edge |

### Items

| File | Relationship |
|------|--------------|
| [../item/enum/002_ring_error.md](../item/enum/002_ring_error.md) | The nine variants and their constructor counts |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_discriminants_here_handlers_elsewhere.md](../pattern/001_discriminants_here_handlers_elsewhere.md) | The arrangement that worked for the policy enums; the error set is where it did not extend |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_two_name_errors_nothing_constructs.md](../pitfall/002_two_name_errors_nothing_constructs.md) | P6 as a trap — the dead match arm and why nothing warns |

### Sources

| File | Relationship |
|------|--------------|
| [`src/error.rs`](../../src/error.rs) | Lines 3–7, the "one error type" argument this instance is the counterexample to |
| [`ring_registry/src/lib.rs`](../../../ring_registry/src/lib.rs) | Line 56, `RegistryError`; 144–159, `register` and its tuple error |
| [`ring_registry/Cargo.toml`](../../../ring_registry/Cargo.toml) | P1's absence — one declared dependency, and it is `ring_handle` |
| [`ring_factory/src/lib.rs`](../../../ring_factory/src/lib.rs) | The `RingConfig` and `Registry` re-exports R1's fix would join |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ❌ **No test in this crate can see any of this.** R1 and R4 are properties of another crate's manifest and a third crate's re-export list; this suite can only assert that `RingError` has the variants it has. The check that would catch R4 belongs to `bench_harness`'s G5 gate, which reads `gate/declared/ring/export_surface.txt` — and G5 is currently vacuous for the family, so it would not catch it today either |

### TY32 — Three Crates Spell the Same Name Collision

`ring_types` and `ring_factory` render byte-identical text; `ring_registry`
renders a variant that includes the name, which is the whole reason it declined
the shared type.

That is the trade in one line: the two enums that can afford `Copy` produce a
message that cannot say *which* name, and the one that says which name cannot be
`Copy` (→ [`../non_functional_requirement/001`](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md)).

### TY33 — `ring_factory` Depends on Both Error Types and Re-Exports Neither

`ring_factory:214` destructures `RegistryError::NameTaken { .. }` and discards
the payload — including the name — to produce its own payload-free variant. The
conversion is deliberate and it is the only thing standing between a
Contract-bound consumer and a `ring_registry` import.

It also means the one message in the family that names the colliding ring is
built, matched, and thrown away one crate below the surface.
