# Type: One Parameter, Three `impl` Blocks, One Derive

### Scope

**Purpose:** Record that `Buffer< S >` splits its methods across three `impl`
blocks at three different bounds, that the loosest two are what let the rings
store `Buffer< UnsafeCell< S > >` at all, and that its single derive renders
three million characters for a realistic ring — including bytes a `clear` was
believed to have removed.

**Responsibility:** The type as declared: its parameter, its bounds, and the one
trait it derives.

**In Scope:** `ring_store/src/lib.rs:58-63, 65, 96, 144`;
`ring_mpsc/src/lib.rs:643-653`; `ring_spsc/src/lib.rs:470-486`.

**Out of Scope:** Why `Default` is required is
[`data_structure/002`](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md).
The three parameter types are [`type/002`](002_the_types_that_cross_the_boundary.md).

---

### BF46 — The Loosest Bounds Are Not Spare Generality; They Are Load-Bearing

The type and its three blocks:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^#\[ derive( Debug ) ]$/,/^}$/p;/^impl< S : Default > Buffer< S >$/p;/^impl< S : Slot + Default > Buffer< S >$/p;/^impl< S > Buffer< S >$/p' ring_store/src/lib.rs
```

Live output:

```
#[ derive( Debug ) ]
pub struct Buffer< S >
{
  slots : Box< [ S ] >,
  capacity : Capacity,
}
impl< S : Default > Buffer< S >
impl< S : Slot + Default > Buffer< S >
impl< S > Buffer< S >
```

Three bounds, loosening left to right. `new` sits alone under `S : Default`;
`clear` and `all_empty` under `S : Slot + Default`; the other nine functions and
both `IntoIterator` impls under a bare `S`.

**Finding.** This instance previously recorded the split as generality nothing
used — the argument being that `new` was the sole constructor and sat in the
`Slot + Default` block, so every `Buffer` in existence already satisfied the
tightest bound and the looser blocks never saw a type the tight one could not
have handled. **That finding is now falsified, and by exactly the caller it said
did not exist.**

`ring_spsc` and `ring_mpsc` store `Buffer< UnsafeCell< S > >`. `UnsafeCell< S >`
is `Default` when `S` is, and is emphatically not a `Slot` — it has no `clear`
and no `is_empty`, and giving it either would be meaningless. So the rings
construct a `Buffer` whose element type only ever satisfies the *loosest two*
blocks. Splitting `new` out to `S : Default` is what made that legal; before it,
`new`'s `Slot` bound was the sole reason the rings wrapped the whole `Buffer` in
one `UnsafeCell` instead — the arrangement that turned out to be unsound, because
reaching a slot through it materialised `&mut Buffer< S >` over the entire
allocation (→ [`ring_spsc` data_structure/001](../../../ring_spsc/docs/data_structure/001_two_cursor_ring.md)).

The correction to draw is not "the old finding was careless" — it was accurate
when written. It is that **an unused bound relaxation and a bound relaxation
whose absence is silently deforming a caller look identical from inside the
defining crate.** `ring_store` could see that nothing named `Buffer< S >`
generically; it could not see that a downstream crate had contorted its storage
because the bound made the honest shape unconstructible. A finding of the form
"nothing exercises this" is a statement about the callers that exist, and stays
true only until one is written — which is a reason to record such findings with
the search that produced them, as this one did, rather than a reason not to
record them.

What remains true from the original entry: the alternative (one block bounded by
`Slot + Default`) would impose a bound on generic downstream code that has no
reason to need it. That cost is no longer hypothetical — it is two crates in this
family.

---

### BF47 — The Single Derive Renders Three Million Characters, Residue Included

`Buffer` derives one trait:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'derive' ring_store/src/lib.rs
```

Live output:

```
/// # The derived `Debug` renders every slot
/// embeds a `Buffer` and derives `Debug` in turn inherits this cost and this
#[derive(Debug)]
```

That derive is not optional — the workspace sets `missing_debug_implementations`
to warn and every verification level runs under `-D warnings`. What it produces:

```
--- (1) what `Debug` on a Buffer prints ---
  Buffer<TypedSlot<u32>> capacity 4:
    Buffer { slots: [TypedSlot(None), TypedSlot(None), TypedSlot(None), TypedSlot(None)], capacity: Capacity(4) }
  TypedSlot<u32>  capacity  1024: 17452 characters of Debug output
  BytesSlot<4096> capacity   256: 3153718 characters of Debug output
    contains the cleared payload bytes: true
```

Three million characters for a 1 MB ring, and the last line is the sharp part:
the buffer was written, then `clear`ed, and the `Debug` rendering still contains
the payload's bytes — `BytesSlot::clear` moves a length, and the derive prints
the array (`ring_slot`'s
[`pitfall/002`](../../../ring_slot/docs/pitfall/002_clear_forgets_it_does_not_erase.md)
SL44).

Both rings sidestep it with hand-written impls:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A8 -F 'impl< S > core::fmt::Debug for Ring< S >' ring_spsc/src/lib.rs
```

Live output:

```
impl< S > core::fmt::Debug for Ring< S >
{
  /// Cursor positions and capacity — never slot contents.
  ///
  /// Formatting the slots would read every one of them, including the ones the
  /// producer may be writing this instant, which is the data race the whole
  /// crate is arranged to avoid. A `Debug` that is unsound to call from the
  /// consumer's thread would be a trap, so it does not exist.
  fn fmt( &self, f : &mut core::fmt::Formatter< '_ > ) -> core::fmt::Result
```

**Finding.** The mitigation is real and complete: a `Ring` prints three scalars,
so no consumer can reach the dump through the type they actually hold. And the
reason `ring_spsc` gives is the right one and the strongest one — reading every
slot from the consumer's thread is the data race the crate exists to prevent.

Two gaps sit behind that. `ring_mpsc:643` has the identical impl with no doc
comment at all, so half the mitigation is unexplained and the next person
comparing the two crates will find one deliberate omission and one that looks
accidental. And neither impl mentions the other two reasons the derive would be
wrong here: three megabytes of output from a single `{ :? }`, and payload bytes
that survived a `clear`. A future type holding a `Buffer` — a test harness, a
diagnostic wrapper, a new ring — satisfies `missing_debug_implementations` with a
`#[ derive( Debug ) ]` and reopens all three at once, with nothing in
`ring_store` to warn it.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A6 -F '/// # The derived `Debug` renders every slot' ring_store/src/lib.rs
```

Live output:

```
/// # The derived `Debug` renders every slot
///
/// `{ :? }` on a `Buffer` walks the full allocation — megabytes of output for a
/// realistic ring, and for `BytesSlot` it includes bytes a `clear` has already
/// logically discarded (`ring_slot`'s own pitfall/002 SL44). `ring_spsc` and
/// `ring_mpsc` both avoid this by giving their own ring type a hand-written
/// `Debug` that prints cursor positions only, never slot contents — a type that
```

**Disposition:** applied — added a `# The derived `Debug` renders every
slot` section to the doc comment on `Buffer< S >` itself, naming all three
gaps this finding lists: the megabyte-scale output, the `BytesSlot` residue
that survives `clear`, and that both rings' hand-written `Debug` impls exist
specifically to avoid deriving it. Placed on the struct, above the derive,
rather than on either ring's manual impl — this is the one place a future
type embedding a `Buffer` and adding its own `#[ derive( Debug ) ]` would be
reading before doing so. Did not add a doc comment to `ring_mpsc`'s own
undocumented manual impl at line 604 — that file is outside this pass's
scope (`ring_align`, `ring_batch`, `ring_store` only). The crate's 17 unit
tests plus 8 doctests re-verified passing (`cargo test --all-features`,
2026-09-04). Now prints: `renders every slot`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/002`](002_the_types_that_cross_the_boundary.md) | The three types this one is parameterised beside |
| [`data_structure/002`](../data_structure/002_the_default_bound_is_a_bound_on_the_type.md) | The `Default` half of the bounded block's bound |
| [`item/001`](../item/001_the_slot_as_the_buffer_sees_it.md) | The two `Slot` calls the bounded block makes |
| [`lifecycle/002`](../lifecycle/002_the_sweep_nothing_calls.md) | The `clear` whose residue this derive prints |
| `ring_slot` — [`pitfall/002`](../../../ring_slot/docs/pitfall/002_clear_forgets_it_does_not_erase.md) | The residue at the slot level |

### Sources

| Fact | Where |
|------|-------|
| The type and its three blocks | `ring_store/src/lib.rs:58-63, 65, 96, 144` |
| The single derive | `ring_store/src/lib.rs:58` |
| The lint that requires it | `Cargo.toml:222-236` |
| Rendered sizes and the surviving bytes | Release probe, quoted above |
| The two crates storing `Buffer< UnsafeCell< S > >` | `ring_spsc/src/lib.rs`, `ring_mpsc/src/lib.rs` — the `slots` field |
| `ring_spsc`'s documented manual impl | `ring_spsc/src/lib.rs:470-486` |
| `ring_mpsc`'s undocumented one | `ring_mpsc/src/lib.rs:643-653` |

### Tests

| Test | Covers |
|------|--------|
| `a_buffer_is_exactly_its_slots_and_its_capacity` | The two fields, through `Debug` |
| `the_same_buffer_type_serves_both_slot_shapes` | One parameter over two slot types |
| `ring_spsc` / `ring_mpsc` — the `Debug` assertions | That a `Ring` renders scalars only |
| *(to create)* | An assertion that a cleared `Buffer< BytesSlot< N > >` renders no payload bytes — which today would fail |
