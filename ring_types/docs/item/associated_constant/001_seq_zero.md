# Seq::ZERO

## Representation

The position of a ring that has published nothing. **Matched on 59 `src/` lines
across eleven sibling crates, and executed on six of them.**

That gap is the constant's defining fact and the reason it has its own instance
rather than a footnote on [`Seq`](../struct/002_seq.md). `Seq::ZERO` is what a
doc example starts from — every crate documenting a cursor, a claim, a barrier or
a gate writes it into an illustration — so a raw grep makes it look like the
family's most-used item. Filter the doc comments out and it is used twice, in two
crates, both times to build something empty.

**All six production references are initialisation, and none is a comparison:**

| Crate | Line | Enclosing | Expression |
|-------|-----:|-----------|------------|
| `ring_atomic` | 130 | `fn default() -> Self` | `Self::new( Seq::ZERO )` |
| `ring_atomic` | 252 | `fn default() -> Self` | `Self::new( Seq::ZERO )` |
| `ring_cursor` | 267 | `CursorPair::new`, `#[ cfg( not( loom ) ) ] pub const fn` | `producer : PaddedCursor::new( Seq::ZERO ),` |
| `ring_cursor` | 268 | same | `consumer : PaddedCursor::new( Seq::ZERO ),` |
| `ring_cursor` | 280 | `CursorPair::new`, `#[ cfg( loom ) ] pub fn` | `producer : PaddedCursor::new( Seq::ZERO ),` |
| `ring_cursor` | 281 | same | `consumer : PaddedCursor::new( Seq::ZERO ),` |

**Only four of the six are live in any one build.** `ring_cursor`'s two `new`
functions are `cfg`-gated alternatives of the same `CursorPair::new` — the
default one is `const`, the `--cfg loom` one is not, because loom's instrumented
atomics cannot be constructed in a `const` context. So an ordinary `cargo build`
compiles lines 267-268 and discards 280-281, and a loom build does the reverse.

**Nobody tests against it.** `seq == Seq::ZERO` appears nowhere; "has this ring
published anything" is not a question the family asks. A ring is constructed at
zero and thereafter only compared to other positions
(→ [`../associated_function/009_seq_distance_to.md`](../associated_function/009_seq_distance_to.md)).

## Kind

Associated Constant (§ Item Kind Taxonomy : Associated Item Kinds #2)

## Definition

`ring_types/src/id.rs:30`

```rust
pub const ZERO : Self = Self( 0 );
```

**No `#[ must_use ]`** — the attribute does not apply to constants, so this is
not the deliberate omission [`Capacity::new`](../associated_function/001_capacity_new.md)'s
is. The first item in `impl Seq`, ahead of the block's three functions
(→ [`../implementation/005_impl_seq.md`](../implementation/005_impl_seq.md)).

**It is exactly redundant with two other spellings.** `Seq` derives `Default`,
whose value is `Seq( 0 )`, and its field is `pub`, so `Seq::ZERO`,
`Seq::default()` and `Seq( 0 )` are three names for one value. All three are used
in the family — the constant in `ring_cursor`'s initialisers, the derive wherever
a `#[ derive( Default ) ]` struct contains a `Seq`, and the literal throughout the
tests.

**The constant earns its place on readability rather than capability.**
`PaddedCursor::new( Seq::ZERO )` says what `PaddedCursor::new( Seq( 0 ) )` only
implies, and in a `const fn` context — `ring_cursor:270` is one — `Default::default()`
is not available at all, since `Default` is not `const`. That last point is the
one real capability difference and it is what `ring_cursor` needed.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/id.rs` | 29, 30 | Doc summary — "The position of a ring that has published nothing." (29); **the definition (30)** |

Two lines, the smallest File Usage entry in the catalog. The constant has no
body, no doc example of its own, and no in-crate reference — `id.rs`'s other
items never mention it.

Test-only references: `ring_types` — reached through `Seq::default()`
assertions rather than by name. Plus **84 references across thirteen consumer
suites**: `ring_publish` 20, `ring_barrier` 14, `ring_gating` 10, `ring_consume`
8, `ring_spsc` 7, `ring_atomic` 6, `ring_mpsc` 5, `ring_tls` 5, `ring_claim` 3,
`ring_cursor` 2, `ring_debug` 2, `ring_batch` 1, `ring_seqno` 1.

**The test count and the production count invert the crate ranking.**
`ring_cursor` has four of the six production references and two of the 84 test
references; `ring_publish` has zero production references and twenty test ones.
The crate that *sets* zero and the crates that *start their fixtures at* zero are
different crates.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/id.rs` | Defining crate |
| `ring_cursor` | `src/lib.rs` | Both `Cursors::new` constructors initialise the producer and consumer cursors (`:267`, `:268`, `:280`, `:281`) |
| `ring_atomic` | `src/lib.rs` | Two `Default` impls, one per atomic wrapper type (`:130`, `:252`) |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'Seq::ZERO' ring_*/src | command grep -v '^ring_types/' \
  | command grep -v ':[0-9]*: *//' | command sed -E 's/^([^:]+):[0-9]+:/\1:/'
```

Live output:

```
ring_atomic/src/lib.rs:    Self::new( Seq::ZERO )
ring_atomic/src/lib.rs:    Self::new( Seq::ZERO )
ring_cursor/src/lib.rs:      producer : PaddedCursor::new( Seq::ZERO ),
ring_cursor/src/lib.rs:      consumer : PaddedCursor::new( Seq::ZERO ),
ring_cursor/src/lib.rs:      producer : PaddedCursor::new( Seq::ZERO ),
ring_cursor/src/lib.rs:      consumer : PaddedCursor::new( Seq::ZERO ),
```

returns exactly those six lines. Dropping the second filter returns 59 — the
figure two files in this catalog originally reported as usage before being
corrected (→ [`../readme.md`](../readme.md) § Where the counts come from).

**`ring_cursor`'s two `CursorPair::new` bodies are byte-identical and only one
compiles at a time.** They differ solely in `#[ cfg ]` and in the `const`
keyword — the loom build drops it, because loom's atomics are not
const-constructible. Writing the initialiser twice rather than extracting it is
forced: a shared helper would have to be `const` for one caller and not for the
other.

**That gate is the concrete reason this constant is not redundant with the
`Default` derive.** `Default::default()` is not callable in a `const fn` on
stable Rust, so the default-build constructor could not use it; `Seq::ZERO` can.
The redundancy is real everywhere except the one place that needed it.
