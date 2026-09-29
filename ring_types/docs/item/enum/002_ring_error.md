# RingError

## Representation

Every way a ring operation can fail — nine variants, `Copy`, allocation-free, and
the only `#[ non_exhaustive ]` item in the crate. One enum rather than one per
crate, because a consumer sits behind the five-crate export surface and never
names the 28 internal crates, so per-crate error types would have to be
converted into a shared one at the surface anyway.

**The "one error type for the family" claim in its module doc is measurably
false.** `ring_registry` declares its own `RegistryError`, and it did so because
this type's `Copy` requirement forbids the `String` payload a name-collision
message needs. Nothing has ruled whether that is a violation, a scoped rule, or
an expired observation (→ [`../../decisions/readme.md`](../../decisions/readme.md), P2).

**Two of the nine variants are constructed by nothing.** `NameTaken` and
`NameUnknown` are declared, classified, formatted and tested, and no code in the
workspace produces either — they exist for the registration failures that
`ring_registry` declined to route through this type
(→ [`../../pitfall/002`](../../pitfall/002_two_name_errors_nothing_constructs.md)).

## Kind

Enum (§ Item Kind Taxonomy : Stable Item Kinds #7)

## Definition

`ring_types/src/error.rs:44`

```rust
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]
#[ non_exhaustive ]
pub enum RingError
{
  CapacityZero,
  CapacityNotPowerOfTwo( usize ),
  Full,
  Empty,
  Closed,
  NameTaken,
  NameUnknown,
  BatchTooLarge { requested : usize, capacity : usize },
  PolicyUnsupported,
}
```

Three shapes in one enum: six unit variants, one tuple variant carrying the
offending capacity, one struct variant carrying both sides of a batch mismatch.
All three stay `Copy` because every payload is a `usize` — the constraint that
made `ring_registry` go its own way.

`#[ non_exhaustive ]` is the crate's only use of the attribute, and it is what
makes this the one enum here whose completeness cannot be asserted from outside:
a foreign `match` needs a wildcard arm, and a wildcard arm absorbs a new variant
silently.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/error.rs` | 20-23, 44, 83, 93-96, 131-134, 162, 184 | Type doc example (20-23); **definition (44**, variants to 80**)**; inherent `impl` header (83); `is_configuration`'s doc example (93-96); `is_transient`'s doc example (131-134); **`impl Display` (162)**; **`impl Error` (184)** |
| `ring_types/src/capacity.rs` | 8, 31-32, 35-37, 40, 44, 48 | **`use crate::RingError;` (8)** — the crate's only cross-module reference; `# Errors` doc (31-32); doc example (35-37); **`new`'s return type (40)**; **the two constructed variants (44, 48)** |
| `ring_types/src/lib.rs` | 21, 42 | Module responsibility table (21, doc); **re-export (42)** |

**`capacity.rs:44` and `capacity.rs:48` are the only two variant constructions in
the entire crate.** Seven of the nine variants are produced elsewhere in the
family or, in two cases, nowhere at all.

Test-only references: `ring_types` (29 in `tests/types_test.rs` — the crate's
most-tested item, across four tests covering the configuration/traffic split, the
transient pair, distinct `Display` output, and the `Error` trait bound), plus 18
consumer crates' suites. The source set is nineteen crates and the test set is
eighteen: `ring_bench` names the type in `src/` and never in `tests/`.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/error.rs`, `src/capacity.rs`, `src/lib.rs` | Defining crate — and the only site of `CapacityZero`/`CapacityNotPowerOfTwo` construction |
| `ring_barrier` | `src/lib.rs` | Reports a barrier wait that could not complete |
| `ring_batch` | `src/lib.rs` | Constructs `BatchTooLarge` |
| `ring_bench` | `src/lib.rs` | Propagates failures out of a benchmark run |
| `ring_claim` | `src/lib.rs` | Reports a claim that found no space |
| `ring_config` | `src/lib.rs` | Propagates `Capacity::new`'s error out of the builder |
| `ring_consume` | `src/lib.rs` | Constructs `Empty` |
| `ring_core` | `src/lib.rs` | Constructs `PolicyUnsupported` (`src/lib.rs:152-155`) |
| `ring_event` | `src/lib.rs` | Reports event-channel failures |
| `ring_factory` | `src/lib.rs` | **On the export Contract** — the error a consumer actually receives |
| `ring_flush` | `src/lib.rs` | **On the export Contract** — reports an incomplete flush |
| `ring_gating` | `src/lib.rs` | Reports a gating check that could not proceed |
| `ring_mpsc` | `src/lib.rs` | Constructs `Full`/`Closed` on the MPSC path |
| `ring_overflow` | `src/lib.rs` | `resolve`'s `Result` arm |
| `ring_publish` | `src/lib.rs` | The publish path's error |
| `ring_shutdown` | `src/lib.rs` | Constructs `Closed` |
| `ring_slot` | `src/lib.rs` | Reports a slot-state violation |
| `ring_spsc` | `src/lib.rs` | Constructs `Full`/`Empty` on the SPSC path |
| `ring_tls` | `src/lib.rs` | **On the export Contract** — propagates per-thread failures |
| `ring_wait` | `src/lib.rs` | Reports a wait that timed out or was cancelled |

**Nineteen consumers — every crate in the family that can fail.** That is the
argument for a single shared error type restated as a measurement: the
alternative is nineteen error types and eighteen conversions at the Contract
boundary. The cost is the one `ring_registry` paid, and it is real
(→ [`../../integration/002`](../../integration/002_the_registry_that_declined_the_shared_error.md)).
