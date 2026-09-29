# pub use error::RingError

## Representation

Re-exports [`RingError`](../enum/002_ring_error.md) from the private
[`error`](../module/002_error.md) module to the crate root. Nineteen sibling
crates' `src/` resolve through it — the widest reach of the four re-exports, and
the reason a single shared error type is cheaper than nineteen local ones.

**This line is what the export Contract actually needs.** Five crates are
exported; four of them — `ring_factory`, `ring_handle`,
`ring_flush`, `ring_tls` — return a `Result` whose error is this type. A consumer
bound to the Contract can therefore name their error without importing anything
off it.

That property has one hole. `ring_registry` returns `RegistryError`, and
`ring_factory` re-exports `Registry` without re-exporting that error, so a
Contract-bound caller can receive an `Err` they cannot name
(→ [`../../integration/002`](../../integration/002_the_registry_that_declined_the_shared_error.md),
[`../../decisions/readme.md`](../../decisions/readme.md) P4).

## Kind

Use Declaration (§ Item Kind Taxonomy : Stable Item Kinds #3)

## Definition

`ring_types/src/lib.rs:42`

```rust
pub use error::RingError;
```

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/lib.rs` | 42 | **The declaration.** Its only occurrence |

`capacity.rs:8` resolves through this line too: it writes `use crate::RingError;`,
which reaches the crate-root binding this re-export creates rather than reaching
`error::RingError` directly. So the line serves the crate's own second module as
well as its nineteen consumers — **deleting it breaks both**.

Downgrading it from `pub use` to plain `use` would not, and the distinction is
worth a probe rather than an assertion. A three-line crate with a private
crate-root re-export and a sibling module writing `use crate::E;` compiles with
only dead-code warnings:

```rust
mod inner { #[ derive( Debug ) ] pub enum E { A } }
mod user { use crate::E; pub fn f() -> E { E::A } }
use inner::E;   // private re-export, deliberately not `pub use`
```

So the `pub` keyword on line 42 buys exactly one thing — the nineteen consumers —
and nothing internal (→ [`005_use_crate_ring_error.md`](005_use_crate_ring_error.md)).

Test-only references: 18 consumer test suites plus `ring_types`' own, all
resolving `ring_types::RingError` through this line without naming it.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/lib.rs`, `src/capacity.rs` | Declares the re-export; `capacity.rs` resolves through it via `crate::` |
| *(nineteen consumers)* | `src/lib.rs` each | Resolve `ring_types::RingError` through it — the full list is on [`../enum/002_ring_error.md`](../enum/002_ring_error.md) |

**Of the four re-exports this is the only one the declaring crate resolves
through itself.** The other three exist purely for consumers; this one is also
`capacity.rs`'s route to the error type it returns.
