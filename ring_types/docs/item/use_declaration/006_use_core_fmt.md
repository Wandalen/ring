# use core::fmt

## Representation

**The crate's only reference to anything outside itself.** `error.rs` needs
`core::fmt` for the two names its hand-written
[`Display` impl](../implementation/003_impl_display_for_ring_error.md) uses in a
signature: `fmt::Formatter` and `fmt::Result`.

`core`, not `std` — and that choice makes the whole crate `no_std`-clean, which
`lib.rs` declares outright rather than leaving to inference:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep '#!\[ no_std \]' ring_types/src/lib.rs
```

Live output:

```
#![ no_std ]
```

A probe against a copy of the five source files confirms it compiles clean, no
source change required:

```sh
cd "$(git rev-parse --show-toplevel)"
S=$(mktemp -d) && cp ring_types/src/*.rs "$S"/ && rustc --edition 2021 --crate-name probe --crate-type lib --out-dir "$S/out" "$S/lib.rs" && ls "$S/out"
```

Live output:

```
libprobe.rlib
```

Exit 0, no warnings, `libprobe.rlib` produced. **The crate needs no change to be
`no_std`** — `grep -rn '\bstd::' src/` returns one line, and it is a comment
explaining why a `use std::` would be wrong here, not an actual reference —
and `core::error::Error` covers the one trait that would otherwise have forced
`std`.

`lib.rs:27` is what makes this enforced rather than incidental. `Cargo.toml` has
no feature for it and none of the six gate scripts checks it, but the attribute
itself is the mechanism: a stray `std::` reference now fails to compile rather
than passing silently, which is a stronger guarantee than a property nothing
declares would give.

Its module doc names the dependencies it is *not* taking and why: no
`error_tools`, no `thiserror`, no `anyhow`, deliberately, so that tier 0 compiles
in isolation. **That makes this line the exact point where the empty
`[dependencies]` table is bought** — a `thiserror` derive would be shorter, and
`error.rs`'s nineteen-line `match` is what the family pays instead
(→ [`../../invariant/002`](../../invariant/002_tier_zero_depends_on_nothing.md)).

The module-form import (`use core::fmt;`, then `fmt::Formatter`) rather than
name-form (`use core::fmt::{ Formatter, Result };`) is load-bearing here: a bare
`Result` in scope would shadow the prelude's, and `capacity.rs` in the same crate
returns `Result< Self, RingError >` meaning the prelude one. Keeping the `fmt::`
qualifier makes the two unmistakable.

## Kind

Use Declaration (§ Item Kind Taxonomy : Stable Item Kinds #3)

## Definition

`ring_types/src/error.rs:13`

```rust
use core::fmt;
```

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/error.rs` | 13, 162, 164 | **The declaration (13)**; **`impl fmt::Display for RingError` (162)**; **`fmt`'s signature — `&mut fmt::Formatter< '_ >` and `fmt::Result` (164)** |

Three lines, all executable. `error.rs:184`'s
[`impl core::error::Error`](../implementation/004_impl_error_for_ring_error.md)
does *not* resolve through this line — it fully-qualifies `core::error::Error`
inline rather than importing it, so the two trait impls two lines apart use
opposite conventions.

Test-only references: none. `tests/types_test.rs` asserts the `Error` bound and
the `Display` output, but through `ring_types::RingError`'s behaviour rather than
by naming `core::fmt`.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/error.rs` | Local alias so the `Display` impl's header and its one method signature can name `fmt::Display`, `fmt::Formatter` and `fmt::Result` without repeating `core::fmt::` three times |

**One row, and it is the only item in the catalog that points outward.** Every
other one of the forty is entirely internal to `ring_types` or to the family. If
a future gate asserts the empty dependency table
(→ [`../../decisions/readme.md`](../../decisions/readme.md), P5), this line is
what it has to permit and `error_tools` is what it has to refuse — which is why
the exemption it would need is awkward enough to have stopped the gate being
written.
