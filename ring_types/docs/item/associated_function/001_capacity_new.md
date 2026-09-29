# Capacity::new

## Representation

The crate's only fallible operation and the only way to obtain a
[`Capacity`](../struct/001_capacity.md). Rejects zero and non-powers of two, so
every value that exists downstream has passed both checks — which is what makes
[`mask`](003_capacity_mask.md) total and removes the power-of-two test from
fifteen consumer crates.

**It is also the crate's only associated function without `#[ must_use ]`, and
that is correct rather than an oversight.** Eleven of the twelve inherent
functions carry the attribute; this one does not, because its return type does:
`core::result::Result` is itself `#[ must_use ]`. Ignoring the result still
warns, and under the workspace's `RUSTFLAGS="-D warnings"` that warning is a
build failure. A probe confirms it:

```rust
#[ derive( Debug ) ] pub enum E { Zero }
pub struct C( usize );
impl C { pub const fn new( s : usize ) -> Result< Self, E > { if s == 0 { return Err( E::Zero ); } Ok( C( s ) ) } }
fn main() { C::new( 8 ); }
```

`rustc` reports `warning: unused Result that must be used` with no `#[ must_use ]`
anywhere in the file.

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/capacity.rs:40`

```rust
pub const fn new( slots : usize ) -> Result< Self, RingError >
```

`const fn`, so a capacity could be validated at compile time. **Nothing in the
workspace does.** Neither production call site passes a literal — `ring_config`
takes the value from a builder argument, `ring_core` from a backend's own
capacity — and

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'const .*Capacity::new' */ --include='*.rs' || true
```

Live output:

```
```

returns nothing. The `--include` matters: without it the same grep hits three
`const CAP : Capacity = match Capacity::new( 1024 )` blocks in this crate's own
`docs/`, which are illustrations of what a caller *could* write, not code.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/capacity.rs` | 27, 31-32, 35-38, 40, 44, 48, 50 | Doc summary (27); `# Errors` section naming both variants (31-32); doc example (35-38); **the definition (40)**; `RingError::CapacityZero` (44); `RingError::CapacityNotPowerOfTwo( slots )` (48); `Ok( Self( slots ) )` (50) |
| `ring_types/src/capacity.rs` | 17, 20, 36-38, 57, 71-72 | Referenced from other doc examples on the same type — the doc-test corpus constructs a `Capacity` eight times |

Test-only references: `ring_types` — 6 in `tests/types_test.rs`, across
`capacity_accepts_powers_of_two` (which loops the powers of two up to 2¹⁶),
`capacity_rejects_zero_and_non_powers_of_two`, and `capacity_compares_by_slot_count`.

**Across the family's source, `Capacity::new` has 113 references and 1 is
production code:**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'Capacity::new' ring_*/src --include='*.rs' | wc -l          # 113
command grep -rn 'Capacity::new' ring_*/src --include='*.rs' \
  | command grep -vc ':[0-9]*: *//'                                                  # 1
```

Live output:

```
113
1
```

The other 112 are documentation and comment references — 111 `///`/`//!`
doctest examples plus one plain `//` comment in `ring_align` that names the
function without calling it. That ratio is why every count in this catalog
separates the two rather than reporting a sum
(→ [`../../pattern/002`](../../pattern/002_a_newtype_that_makes_a_check_unnecessary.md)).

**A neighbouring instance reports 105/104/1 for the same identifier, and both are
right.** [`../../pattern/002`](../../pattern/002_a_newtype_that_makes_a_check_unnecessary.md)
measures *external* references — its recipe adds `grep -v '^ring_types/'`
— so it excludes this crate's own eight doc examples in `capacity.rs`. 113 − 105 = 8,
exactly those.

The production count is 1 under either scope, which is the point: **the number
that matters is scope-invariant and the number that moves is the denominator.**
Any figure in this catalog that changes when you widen the grep is describing the
documentation corpus, not the code.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/capacity.rs` | Defining crate |
| `ring_config` | `src/lib.rs` | `RingConfig::new` validates the caller's slot count and propagates the error with `?` (`:71`) |

**One production call site, not two.** `ring_core::Ring::capacity` used to hold
a second — re-wrapping a number already known valid after it came back out of
crossbeam's `ArrayQueue`, with `.expect( "constructed from a validated Capacity" )`
— until it was refactored to carry the validated `Capacity` inside the storage
variant directly (`ring_core/src/lib.rs:227-240`), so the accessor no longer
re-runs this crate's only fallible check on every call. `ring_config` is the
sole production caller now, and it calls for the original reason: the input
might actually be wrong.

## Caller Tree

- *No caller within `ring_types`* — the crate's four modules never call each other's functions; the only cross-module edge is an import (→ [`../use_declaration/005_use_crate_ring_error.md`](../use_declaration/005_use_crate_ring_error.md))
- *External: `ring_config::RingConfig::new`* (`ring_config/src/lib.rs:71`)

## Callee Tree

- *External: `usize::is_power_of_two`* (`capacity.rs:46`) — the check the whole pattern exists to perform once
- [`RingError::CapacityZero`](../enum/002_ring_error.md) — variant construction (`capacity.rs:44`)
- [`RingError::CapacityNotPowerOfTwo`](../enum/002_ring_error.md) — variant construction (`capacity.rs:48`)

**Three callees and one of them is the entire function.** `is_power_of_two` is a
`core` intrinsic on `usize`; the other two are enum constructions in the same
crate. This is the shallowest callee tree possible for a function that does real
work, and it is why the function is `const`.
