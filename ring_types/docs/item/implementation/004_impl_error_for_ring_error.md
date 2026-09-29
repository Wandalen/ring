# impl Error for RingError

## Representation

**The crate's only empty item, and its shortest line of consequence.** Every
method on `core::error::Error` has a default implementation — `source()` returns
`None`, `description()` is deprecated — so the empty block is a pure declaration
that `RingError` participates in the error trait, and nothing more.

What it buys is disproportionate to its size: `Box< dyn Error >` compatibility,
`?`-conversion into any error type that implements `From< Box< dyn Error > >`,
and interoperation with every error-handling crate in the ecosystem — for
nineteen consumer crates, at the cost of one line and zero dependencies.

`core::error::Error`, not `std::error::Error`. That choice is why the crate
compiles unchanged under `#![ no_std ]`
(→ [`../use_declaration/006_use_core_fmt.md`](../use_declaration/006_use_core_fmt.md),
where the probe is).

## Kind

Implementation (§ Item Kind Taxonomy : Stable Item Kinds #12)

## Definition

`ring_types/src/error.rs:184`

```rust
impl core::error::Error for RingError {}
```

Fully qualified inline rather than imported — the opposite convention to
[`impl fmt::Display`](003_impl_display_for_ring_error.md) twenty-two lines above.
Here it is the cheaper choice: the trait path appears once, so an import would
add a line rather than save two.

`Display` is a supertrait of `Error`, so this block does not compile without
`error.rs:162`. The two are ordered accordingly and the dependency is invisible
in the source — deleting the `Display` impl produces an error reported against
*this* line.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/error.rs` | 184 | **The whole block** — header, body and closing brace on one line, the file's last |

Test-only references: `ring_types` — `tests/types_test.rs`'s
`error_implements_the_error_trait` asserts the bound structurally rather than
behaviourally:

```rust
fn accepts< E : core::error::Error >( _ : E ) {}
```

A nested helper generic over the bound, called with a `RingError`. If this impl
were deleted the test would fail to compile rather than fail an assertion, which
is the strongest form the check can take — and the only one available, since the
trait adds no observable behaviour to assert on.

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/error.rs` | Declares the impl |
| *(nineteen consumers)* | `src/lib.rs` each | Any that boxes, converts, or `?`-propagates a `RingError` into a foreign error type relies on it |

**Nothing in the family currently needs it.** No crate in `ring_*/`
boxes a `RingError` or converts it through `From< Box< dyn Error > >`:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'dyn Error\|Box< dyn\|Box<dyn' ring_*/src
```

Live output:

```
ring_flush/src/lib.rs:// binding refusal or fold it into a `Box< dyn Error >` alongside the other two.
```

returns exactly one line, and it is a comment: `ring_flush/src/lib.rs`
weighing whether to "fold it into a `Box< dyn Error >` alongside the other two"
and deciding against. So the family has considered the capability once and used
it zero times.

That makes this line one written entirely for consumers outside the
family — the Contract's actual audience, who are not in this workspace and cannot
be measured from it. That makes it the one item in this catalog whose
justification is not falsifiable by grep, and it is worth being explicit about
that rather than reporting the zero as though it meant the impl were unused.
