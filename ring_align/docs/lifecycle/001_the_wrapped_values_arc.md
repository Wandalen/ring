# Lifecycle: The Wrapped Value's Arc

### Scope

- **Purpose**: Follow a value from before it is wrapped to after it is unwrapped, marking where the crate's guarantee begins, what preserves it through relocation, and the transition that ends it.
- **Responsibility**: Give the states, the transitions, the compiler-enforced boundaries, and the one exit that discards the property silently.
- **In Scope**: `CacheAligned<T>` from `new` to `into_inner`, including moves.
- **Out of Scope**: The layout at rest, which is [`data_structure/001`](../data_structure/001_the_cache_aligned_wrapper.md); the constant's own lifecycle, which is [`lifecycle/002`](002_the_constant_across_a_platform_port.md).

### States

| State | The value is | Guarantee |
|-------|--------------|-----------|
| S1 | A bare `T` | None |
| S2 | Wrapped, standing alone (a local, a `Box`, a single field) | Alignment and whole-line size hold — and buy **nothing yet**, because there is no neighbour to be separated from |
| S3 | Wrapped, as one of two wrapped fields in a struct | The guarantee is live: this is the only state the crate exists for (→ [`invariant/001`](../invariant/001_two_wrapped_fields_never_share_a_line.md)) |
| S4 | Wrapped, relocated — moved into a `Box`, a `Vec`, another struct, or returned by value | Preserved. See § What a Move Preserves |
| S5 | Unwrapped via `into_inner` | Gone, with no signal |

**S2 is worth naming separately from S3** because it is where most wrapped
values spend most of their time and where the property is inert. A
`CacheAligned` in a local variable is 64 bytes doing the job of 8. That cost is
paid in every state; the benefit exists only in S3.

### Transitions

| From → To | Trigger | Note |
|-----------|---------|------|
| S1 → S2 | `CacheAligned::new( value )` | Infallible, `const`. Nothing is validated because nothing about the *value* matters |
| S1 → S2 | `CacheAligned::default()` | The `Default` derive — creates the wrapper without the caller naming a payload |
| S2 → S3 | The caller declares a second wrapped field beside it | **Not an operation this crate can see.** The transition into the state that matters happens entirely in the consumer's type definition (→ [`data_structure/002`](../data_structure/002_a_padded_pair_in_one_struct.md)) |
| S2/S3 → S4 | Any move | Preserved by the type's alignment travelling with it |
| S2/S3 → S2/S3 | `CacheAligned::clone` / `Copy` | A second wrapped value, independently aligned. Two copies of a cursor is a different bug, but not this crate's |
| S2/S3 → S5 | `into_inner` | The only exit. The padding is dropped with the wrapper |
| S2/S3 → (unchanged) | `get`, `get_mut` | Borrow only. The payload is reached in place, so the wrapper and its alignment persist |

**The interesting transition is the one this crate cannot observe**: S2 → S3.
The wrapper is complete and correct in S2, and whether the family gets any
benefit depends on a declaration in another crate. That asymmetry is why
`on_distinct_lines` is exported rather than kept as a test helper — it is the
only way a consumer can confirm it reached S3
(→ [`api/001`](../api/001_the_reading_surface.md)).

### What a Move Preserves

Moving a wrapped value does not weaken the guarantee: alignment is a property
of the type, so every destination the compiler or an allocator chooses is
64-aligned. `ring_cursor` asserts this on a real heap move rather than
assuming it (`ring_cursor/tests/cursor_test.rs:99`):

```rust
assert!( boxed.on_distinct_lines(), "still separated after a move onto the heap" );
```

**That test is doing more than it looks.** A stack-allocated pair being
separated is unsurprising; a pair that stays separated after `Box::new` moves
it to the heap confirms the allocator honours the type's alignment rather than
the guarantee being an artifact of stack layout.

The same holds for `Vec<CacheAligned<T>>` — the allocation is made with the
element's alignment, and each element is a whole number of lines, so every
element starts a line. That composition is the payoff of whole-line *sizing*
rather than alignment alone (→ [`algorithm/002`](../algorithm/002_rounding_a_payload_up_to_whole_lines.md)).

### What Cannot Strip the Alignment

One route that looks like it should defeat the property is closed by the
compiler. A containing struct cannot pack the wrapper flat. Verified on
`rustc 1.97.1 (8bab26f4f 2026-07-14)`:

```sh
cat > ./-probe.rs <<'EOF'
#[ repr( align( 64 ) ) ]
pub struct CacheAligned< T >( T );

#[ repr( packed ) ]
pub struct Packed { a : CacheAligned< u64 >, b : u8 }

fn main() { let _ = core::mem::size_of::< Packed >(); }
EOF
rustc --crate-name probe --edition 2021 -o /dev/null ./-probe.rs
# error[E0588]: packed type cannot transitively contain a `#[repr(align)]` type
rm -f ./-probe.rs
```

Live output:

```
error[E0588]: packed type cannot transitively contain a `#[repr(align)]` type
 --> ./-probe.rs:5:1
  |
5 | pub struct Packed { a : CacheAligned< u64 >, b : u8 }
  | ^^^^^^^^^^^^^^^^^
  |
note: `CacheAligned` has a `#[repr(align)]` attribute
 --> ./-probe.rs:2:1
  |
2 | pub struct CacheAligned< T >( T );
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: aborting due to 1 previous error

For more information about this error, try `rustc --explain E0588`.
```

**`transitively` is the load-bearing word.** The prohibition follows the
wrapper through any depth of nesting, so `PaddedCursor` and `CursorPair` are
covered by it too without either declaring anything. A consumer cannot
accidentally flatten the padding, and cannot deliberately do it either without
a hard compile error naming the type responsible.

That closes the container route. It leaves S5 as the only way the property
ends.

### The Exit Is Silent

```rust
let raw = padded.into_inner();     // S5
```

`into_inner` compiles, is not `unsafe`, carries no `#[ must_use ]` — and one
would not help anyway, since the failure here is *using* the result, not
discarding it — and produces a `T` with no padding and no complaint. Storing that `T`
beside another hot value reintroduces exactly the contention the wrapper
existed to remove, and every test in this crate still passes because none of
them can see the caller's struct.

| # | Property of the exit | Consequence |
|---|----------------------|-------------|
| L1 | Not `unsafe` | No lint, no review trigger |
| L2 | Not `const` (`E0493` — the destructor cannot be evaluated at compile time, → [`type/002`](../type/002_cache_aligned.md)) | The one friction point, and it only bites in `const` contexts |
| L3 | Returns a plain `T` | The type system stops tracking the property at this line |
| L4 | Named "into_inner", a familiar and unalarming name | Reads as a normal unwrap because it is one |

**L3 is the real statement.** The wrapper is the only carrier of the property,
so discarding the wrapper discards the property — that is not a defect, it is
what a layout carrier *is* (→ [`pattern/001`](../pattern/001_the_newtype_as_layout_carrier.md) K1).
The doc comment says so in three words — *"Unwrap, discarding the padding"* —
which is the correct mitigation available at this level and is not a detector.

### AL35 — The Transition That Ends the Guarantee Is the One Without `#[ must_use ]`

```
99:  pub fn into_inner( self ) -> T
```

Wrap, hold and move all preserve the padding. `into_inner` consumes the wrapper
and hands back a bare payload with no line of its own — the single transition
that ends the crate's guarantee — and it is the one entry point carrying no
annotation
(→ [`api/002`](../api/002_the_wrapper_surface.md) AL5).

**Finding.** Discarding a `CacheAligned` by ignoring the value it unwraps is
therefore both irreversible and silent: the wrapper is gone, the payload is
dropped, and nothing warns. It is the only move in the crate with that
combination, and the annotation that would catch it is on the predicate instead.

---

### AL36 — Nothing at Runtime Can Observe Which Stage a Value Is In

Every transition is a `Copy` move over a type with no state of its own. There is
no flag, no discriminant, and no observable difference between a value that has
been wrapped and one that has been moved twice.

**Finding.** The lifecycle documented here is the reader's model of the
guarantee, not a machine's — worth saying outright, because a lifecycle document
usually implies a state a program can query, and this one is a sequence of
things a *person* has to keep true. That is also why the obligation is
documented at the API boundary
(→ [`api/002`](../api/002_the_wrapper_surface.md) AL8) rather than checked
anywhere.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_rounding_a_payload_up_to_whole_lines.md](../algorithm/002_rounding_a_payload_up_to_whole_lines.md) | Why S4 composes into arrays as well as into single moves |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_wrapper_surface.md](../api/002_the_wrapper_surface.md) | The four operations that drive every transition above |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_a_padded_pair_in_one_struct.md](../data_structure/002_a_padded_pair_in_one_struct.md) | S3, the state the crate exists for and cannot itself reach |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_two_wrapped_fields_never_share_a_line.md](../invariant/001_two_wrapped_fields_never_share_a_line.md) | C5 — the S5 exit stated as a violation consequence |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_the_constant_across_a_platform_port.md](002_the_constant_across_a_platform_port.md) | The other long-lived thing, whose change invalidates the guarantee in every state at once |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_newtype_as_layout_carrier.md](../pattern/001_the_newtype_as_layout_carrier.md) | K1 — the silent exit as a general property of layout-carrying newtypes |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_cache_aligned.md](../type/002_cache_aligned.md) | `E0493`, which is why L2 is the one place the exit meets friction |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/tests/cursor_test.rs:99` | The heap-move assertion behind § What a Move Preserves |
| `ring_align/src/lib.rs:109` | "Unwrap, discarding the padding" — the exit documented at the exit |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | The wrapper's transparency test covers S1 → S2 → S5 for the payload's *value*; nothing covers the property's disappearance, because there is nothing to observe |
| `ring_cursor/tests/cursor_test.rs` | The only S4 assertion in the workspace |
