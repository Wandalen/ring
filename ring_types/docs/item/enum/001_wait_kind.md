# WaitKind

## Representation

What a consumer does when it asks for items and the ring has none — a closed,
four-variant, `Copy` set with no payload. `Spin`, `Yield` and `Park` trade
latency against CPU; `None` returns immediately with whatever is available and is
**the only variant reachable from inside a tick**, which is why this enum
requires it specifically rather than as one of four.

**This crate owns the names and nothing else.** `ring_wait` owns the strategies
that dispatch on these discriminants, and the
rule holds here in a form that is checkable — not by counting `match`
keywords (the crate has six, after every classifier's conversion to an
exhaustive form for exhaustiveness-safety; →
[`../../pattern/001`](../../pattern/001_discriminants_here_handlers_elsewhere.md)),
but by checking what each one returns. Only `Display::fmt`'s on `RingError`
selects among actions; every other `match` in `src/`, including this type's
own [`is_non_blocking`](../associated_function/011_wait_kind_is_non_blocking.md),
is a total function to `bool` — a classifier over the discriminant, not a
dispatch to a strategy.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r '\bmatch\b' ring_types/src/ | command grep -v ': *//'
```

Live output:

```
ring_types/src/error.rs:    match self
ring_types/src/error.rs:    match self
ring_types/src/error.rs:    match self
ring_types/src/policy.rs:    match self
ring_types/src/policy.rs:    match self
ring_types/src/policy.rs:    match self
```

Six lines: three in `error.rs` (`is_configuration`, `is_transient`,
`Display::fmt`), three in `policy.rs` (`is_non_blocking`, `reports_failure`,
`drops_silently`). `is_non_blocking`'s own body used to be the crate's
shortest — a one-line `matches!( self, Self::None )` — until
`Fix(wait_kind_is_non_blocking_classification_not_exhaustive)`
(`policy.rs:67`) gave it the same exhaustive shape as its siblings, so a
fifth `WaitKind` variant now fails to compile here rather than silently
reading `false`.

## Kind

Enum (§ Item Kind Taxonomy : Stable Item Kinds #7)

## Definition

`ring_types/src/policy.rs:22`

```rust
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash, Default ) ]
pub enum WaitKind
{
  #[ default ]
  Spin,
  Yield,
  Park,
  None,
}
```

No `PartialOrd`/`Ord` — unlike `Seq` and `SlotIndex`, and correctly: the four
strategies are not ordered by anything, and deriving an order would invite a
comparison whose meaning is discriminant declaration order.

Not `#[ non_exhaustive ]`. Adding a fifth variant is a breaking change on
purpose, so that every consumer matching on this enum is forced to decide what
the new strategy means to them.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/policy.rs` | 12, 16-19, 22, 37, 55-56, 60-61, 64-65 | Type doc citing the wait-kind requirement (12); doc example (16-19); **definition (22**, whose variant block runs to 35 without respelling the type name**)**; inherent `impl` header (37); `ALL`'s doc example (55-56); `is_non_blocking`'s doc and example (60-61, 64-65) |
| `ring_types/src/lib.rs` | 12, 20, 44 | Discriminant/handler-split prose (12); module responsibility table (20); **re-export (44)** |

Twelve occurrences in `policy.rs` and three in `lib.rs`, from
`grep -n '\bWaitKind\b' src/policy.rs src/lib.rs`. The variant declarations at
24-34 are deliberately outside the range: they are part of the definition but do
not contain the identifier, and widening the citation to cover them would break
OT008's rule against spanning a gap.

Test-only references: `ring_types` (11 in `tests/types_test.rs`, across three
tests — the four-variant closure assertion, the exactly-one-non-blocking
partition, and the `Default` check), plus six consumer crates' suites:
`ring_barrier`, `ring_config`, `ring_factory`, `ring_shutdown`, `ring_spsc`,
`ring_wait`.

**The test set and the source set are not the same six.** `ring_claim` and
`ring_publish` name the type in `src/` but never in `tests/`; `ring_spsc` does
the reverse. Neither asymmetry is wrong, and both are the kind of thing only an
exhaustive per-item count surfaces.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/policy.rs`, `src/lib.rs` | Defining crate — declares the discriminants |
| `ring_barrier` | `src/lib.rs` | Chooses how a barrier waits for a position |
| `ring_claim` | `src/lib.rs` | Chooses how a producer waits for space |
| `ring_config` | `src/lib.rs` | Carries the configured strategy on a `RingConfig` |
| `ring_factory` | `src/lib.rs` | **On the export Contract** — a builder setter takes it |
| `ring_publish` | `src/lib.rs` | Chooses how a publish waits |
| `ring_shutdown` | `src/lib.rs` | Chooses how the drain loop waits at close |
| `ring_wait` | `src/lib.rs` | **The handler** — dispatches on the discriminant to spin, yield, park, or return |

**Seven consumers, one of which is the reason for the other six.** `ring_wait` is
the only crate that turns a `WaitKind` into behaviour; the rest carry it as
configuration from the Contract surface down to the point where `ring_wait` acts
on it. That is the discriminant/handler split as a call graph rather than as a
rule.
