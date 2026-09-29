# Implementation Items

### Scope

- **Purpose**: Catalog the crate's eight `impl` blocks — six inherent, two trait.
- **Responsibility**: Give each block's header, what it contains, and what it costs.
- **In Scope**: The six inherent impls on `Capacity`, `RingError`, `Seq`, `SlotIndex`, `WaitKind`, `OverflowPolicy`; `impl Display for RingError`; `impl Error for RingError`.
- **Out of Scope**: The associated items inside them, which are separate instances (→ [`../associated_function/`](../associated_function/), [`../associated_constant/`](../associated_constant/)); the derived impls, which are attribute expansions rather than items.

### Overview Table

| ID | Name | Kind | Defined In | Status |
|----|------|------|-----------|--------|
| 001 | [impl Capacity](001_impl_capacity.md) | Implementation (#12) | `src/capacity.rs:25` | 🔄 |
| 002 | [impl RingError](002_impl_ring_error.md) | Implementation (#12) | `src/error.rs:83` | 🔄 |
| 003 | [impl Display for RingError](003_impl_display_for_ring_error.md) | Implementation (#12) | `src/error.rs:162` | 🔄 |
| 004 | [impl Error for RingError](004_impl_error_for_ring_error.md) | Implementation (#12) | `src/error.rs:184` | 🔄 |
| 005 | [impl Seq](005_impl_seq.md) | Implementation (#12) | `src/id.rs:27` | 🔄 |
| 006 | [impl SlotIndex](006_impl_slot_index.md) | Implementation (#12) | `src/id.rs:101` | 🔄 |
| 007 | [impl WaitKind](007_impl_wait_kind.md) | Implementation (#12) | `src/policy.rs:37` | 🔄 |
| 008 | [impl OverflowPolicy](008_impl_overflow_policy.md) | Implementation (#12) | `src/policy.rs:116` | 🔄 |

### Six inherent, two trait, and only one has a body

| Block | Contains |
|-------|----------|
| `impl Capacity` | 3 associated functions |
| `impl RingError` | 2 associated functions |
| `impl Seq` | 1 associated constant, 3 associated functions |
| `impl SlotIndex` | 1 associated function |
| `impl WaitKind` | 1 associated constant, 1 associated function |
| `impl OverflowPolicy` | 1 associated constant, 2 associated functions |
| `impl Display for RingError` | 1 associated function — the crate's only `match` |
| `impl Error for RingError` | **nothing — `{}`** |

Sixteen associated items across eight blocks. **`impl core::error::Error for
RingError {}` is empty and is the point**: `Error`'s methods all have defaults,
so the empty block is a pure declaration that `RingError` participates in the
error trait, costing one line and buying `Box< dyn Error >` compatibility for
nineteen consumer crates.

### The two trait impls two lines apart disagree about imports

```rust
impl fmt::Display for RingError   // error.rs:162 — via `use core::fmt;`
impl core::error::Error for RingError {}   // error.rs:184 — fully qualified inline
```

Neither is wrong and the inconsistency is invisible to the compiler. It is worth
recording because `Display` is named three times in the file (header plus two
signature positions) and `Error` once, so the import pays for itself in the first
case and would not in the second
(→ [`../use_declaration/006_use_core_fmt.md`](../use_declaration/006_use_core_fmt.md)).

### Not one impl takes `&self`

```sh
cd "$(git rev-parse --show-toplevel)"
command grep '&self\|&mut self' ring_types/src/*.rs
```

Live output:

```
ring_types/src/error.rs:  fn fmt( &self, f : &mut fmt::Formatter< '_ > ) -> fmt::Result
```

returns exactly one line — `error.rs`, `Display::fmt`'s, whose signature the
trait imposes. Every other associated function in the crate takes `self` by value,
because every type here is `Copy` and at most sixteen bytes
(→ [`../../non_functional_requirement/001`](../../non_functional_requirement/001_errors_and_positions_do_not_allocate.md)).

**Twelve of the sixteen associated items are `const fn`.** The four that are not
are the three `ALL`/`ZERO` constants (already `const` by kind) and `Display::fmt`
(a trait method that cannot be). So the crate's entire inherent surface is
usable in a `const` context — a property nothing in the workspace currently
exploits, since `Capacity::new`'s one production call site doesn't take a
literal (→ [`../../pattern/002`](../../pattern/002_a_newtype_that_makes_a_check_unnecessary.md)).
