# Workaround: The Trait Must Travel With the Type

### Scope

- **Purpose**: Record the re-export as a workaround for a language rule rather than as a convenience, and measure the second name it creates.
- **Responsibility**: State the rule, show that handing out `PaddedCursor` without `SeqCell` is unusable, count which path each consumer takes, and name the alternative that was not chosen.
- **In Scope**: `pub use ring_atomic::SeqCell` and the two import paths it produces.
- **Out of Scope**: Why `PaddedCursor` must be a local newtype at all, which is [`pattern/001`](../pattern/001_the_forwarding_newtype.md) — a consequence of the *orphan* rule, not this one.

### The Constraint

A trait method is callable only where the trait is in scope. So a crate handed a
`PaddedCursor` and no route to `SeqCell` holds 64 bytes it cannot read:

```rust
// without `use …::SeqCell` in scope:
cursor.load( GATING );
// error[E0599]: no method named `load` found for struct `PaddedCursor`
```

This is separate from the orphan rule. The orphan rule decides *where the impl
may be written*; this decides *where the methods may be called*. Both bite here,
and they have different workarounds.

### The Workaround

```rust
pub use ring_atomic::SeqCell;
```

One line, with the argument at `src/lib.rs:60-68`:

> Making each such crate declare `ring_atomic` itself would put a dependency in
> four manifests to import one trait — and would say, wrongly, that those crates
> have business with the atomic layer beyond the cursor they were handed.

**The second clause is the real argument.** The first is about convenience — six
manifest lines is not a burden. The second is about what a manifest *means*: a
`ring_atomic` edge asserts that the crate works with atomics, and `ring_gating`
does not. It works with cursors, which happen to be atomic inside.

### What It Costs: One Trait, Two Paths

The re-export does not replace the original path; it adds a second one. Both
resolve to the same item, and consumers pick either:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'use ring_cursor::.*SeqCell\|use ring_atomic::.*SeqCell' \
  --include=*.rs */src/ | grep -vE ':[[:space:]]*(///|//!)'
```

Live output:

```
ring_batch/src/lib.rs:use ring_atomic::SeqCell;
ring_claim/src/lib.rs:use ring_cursor::{ PaddedCursor, SeqCell, GATING };
ring_consume/src/lib.rs:use ring_cursor::{ PaddedCursor, SeqCell, GATING };
ring_cursor/src/lib.rs:pub use ring_atomic::SeqCell;
ring_debug/src/lib.rs:use ring_atomic::SeqCell;
ring_mpsc/src/lib.rs:use ring_atomic::{ AtomicSeq, SeqCell };
ring_publish/src/lib.rs:use ring_cursor::{ PaddedCursor, SeqCell, GATING };
ring_spsc/src/lib.rs:use ring_cursor::{ CursorPair, SeqCell, GATING };
ring_tls/src/lib.rs:use ring_atomic::SeqCell;
```

| Path | Crates | Count |
|------|--------|------:|
| `ring_cursor::SeqCell` | `ring_barrier`, `ring_claim`, `ring_consume`, `ring_gating`, `ring_publish`, `ring_shutdown`, `ring_spsc`, `ring_wait` | 8 |
| `ring_atomic::SeqCell` | `ring_debug`, `ring_mpsc` | 2 |

Both are correct. But the split means a reader grepping for `ring_atomic::SeqCell`
to find every user of the trait finds two of ten, and a reader grepping for
`ring_cursor::SeqCell` finds eight of ten. **Neither query answers the question**,
and nothing signals that a second spelling exists.

### `ring_mpsc` Splits Them in One File

```rust
use ring_atomic::{ AtomicSeq, SeqCell };   // :185
use ring_cursor::{ PaddedCursor, GATING }; // :189
```

The type comes from one crate and the trait from another, four lines apart. That
is exactly the pairing the re-export was written to keep together, and it is
undone in the crate that holds both edges.

It is not wrong — `ring_mpsc` uses `AtomicSeq` directly, so it needs the
`ring_atomic` edge regardless, and importing `SeqCell` from the crate it is
already importing from is the natural thing to write. **The workaround does not
fail here; it simply has no effect**, because its whole mechanism is being the
only available path, and for this crate it is not.

### The Alternative That Was Not Chosen

Inherent methods on `PaddedCursor` that forward to the trait:

```rust
impl PaddedCursor
{
  pub fn load( &self, order : Ordering ) -> Seq { SeqCell::load( self, order ) }
  // …three more
}
```

Inherent methods need no import at all, so the re-export could be dropped and all
ten consumers would work with no trait in scope.

| | Re-export (chosen) | Inherent forwarding |
|---|---|---|
| Consumer imports | One `use` line | None |
| Public surface here | 1 item | 4 more methods, each needing docs and a doc example |
| Generic code over `SeqCell` | Works | Still needs the trait imported separately |
| Duplication | None | The four signatures again, a third time after `SeqCell` and the impl |

**The third row decides it.** `ring_batch::claim_gated` is generic over
`P : SeqCell, C : SeqCell` — a real caller that needs the trait itself, not
methods on a concrete type. Inherent methods would not serve it, so the trait
would have to be importable anyway, and the crate would carry both surfaces.

### What Would Notice a Problem

| # | Check | Sees |
|---|-------|------|
| X1 | The compiler | Immediately, if the re-export is removed — eight crates fail |
| X2 | `missing_docs` | The re-export carries its own doc comment, so removing the *docs* would warn |
| X3 | Anything about the two-path split | **Nothing.** Both spellings compile; no lint distinguishes them |
| X4 | `cargo +nightly udeps` | Would not flag `ring_debug`/`ring_mpsc` — both use `ring_atomic` for more than the trait |

**X3 is the open one.** If the family wanted one canonical spelling, the check
would be a grep for `ring_atomic::SeqCell` outside `ring_atomic` and
`ring_cursor` — currently two hits, both defensible. Nothing prevents a third
that is not.

### CU51 — The Re-Export Saves Four Manifests, Not Six

```
who took the re-export, in library code
ring_claim ring_consume ring_publish ring_spsc
control: who took it directly from ring_atomic instead
ring_batch ring_debug ring_mpsc ring_tls
```

The doc comment claims six. The control arm is what makes four mean something:
the same number of crates reach the trait without the re-export at all, so it is
not the family's only route.

**Finding.** Four is still worth the line — a `use` that would otherwise need a
manifest edit in four crates is a real saving. The gap between four and six is
the gap between a measurement and an estimate written once and never re-run,
which is the third instance of that shape in this crate after CU5 and CU13.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'manifests to import one trait' ring_cursor/src/lib.rs
```

Live output:

```
/// four manifests to import one trait — and would say, wrongly, that those
```

**Disposition:** applied — same underlying fix as `api/001`'s CU5: reworded
the doc comment at `src/lib.rs:60-68` from "six manifests" to "four
manifests", matching the measurement this finding itself makes — four crates
via the re-export, four more (`ring_batch`, `ring_debug`, `ring_mpsc`,
`ring_tls`) direct from `ring_atomic`. Verified with `cargo test --release -p
ring_cursor` (all tests and doctests passing).
Now prints: `four manifests to import one trait`

---

### CU52 — The Stated Exception Covers Cases It Was Not Aimed At

`src/lib.rs:60-68` names "independent business with the atomic layer" as the
legitimate reason to import `SeqCell` directly. Four crates do: `ring_batch`,
`ring_debug`, `ring_mpsc`, `ring_tls`.

**Finding.** Two of them — `ring_batch` and `ring_tls` — do not depend on this
crate at all, so "skipping the re-export" was never a choice they made; there was
nothing to skip. The exception as written describes a decision, and half the
crates it covers never faced one. The distinction matters because it is the same
conflation that produced the wrong count at CU51.

---

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_surface_that_forwards.md](../api/001_the_surface_that_forwards.md) | The re-export as a public item, with the same census |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_who_reads_a_cursor.md](../integration/002_who_reads_a_cursor.md) | The ten consumers, and which route each takes |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_forwarding_newtype.md](../pattern/001_the_forwarding_newtype.md) | The orphan-rule half of the same trait-scoping problem |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_padded_cursor.md](../type/002_padded_cursor.md) | The type the trait must travel with |

### Workarounds

| File | Relationship |
|------|--------------|
| [001_the_loom_constructor_cannot_be_const.md](001_the_loom_constructor_cannot_be_const.md) | The crate's other language-imposed workaround |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:60-69` | The re-export and its argument |
| `ring_mpsc/src/lib.rs:201, 205` | The trait and the type imported from different crates |
| `ring_batch/src/lib.rs:306` | `claim_gated`, generic over `SeqCell` — why inherent methods would not have sufficed |
| `ring_atomic/src/lib.rs:108-151` | The trait's definition |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:137-160` | The four methods exercised through the re-exported trait |
| `ring_gating/tests/gating_test.rs:31` | A consumer importing the trait by this crate's path |
| `ring_mpsc/tests/mpsc_test.rs:54` | A consumer importing it by the original path |
