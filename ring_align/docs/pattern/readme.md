# Pattern Doc Definition

### Scope

- **Purpose**: Name the two reusable shapes this crate is an instance of, so the next crate facing the same problems inherits the reasoning rather than rederiving it.
- **Responsibility**: For each, state the shape, the family evidence for it, and the limit it does not pass.
- **In Scope**: The newtype whose content is its layout; single ownership of a magic number.
- **Out of Scope**: This crate's specific types, which are [`type/`](../type/readme.md); the placement argument, which is [`integration/002`](../integration/002_why_the_constant_lives_here.md).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Newtype as Layout Carrier](001_the_newtype_as_layout_carrier.md) | A wrapper that adds no behaviour, no invariant, and no field — only an attribute — set against the family's three other newtypes | 🔄 |
| 002 | [One Owner for a Magic Number](002_one_owner_for_a_magic_number.md) | Give a platform constant exactly one home, and the limit: an owner makes duplication visible, not impossible | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the family newtypes, and whether the field is public --'
command grep -rhE '^pub struct [A-Z][A-Za-z]*\(' ring_*/src/*.rs | sort -u
echo '  -- the one the pattern is named for, which the shape above does not match --'
command grep -B2 '^pub struct CacheAligned' ring_align/src/lib.rs
echo '  -- declared owners of the line size, family-wide --'
command grep -rc 'pub const CACHE_LINE' ring_*/src/*.rs | command grep -v ':0$'
echo '  -- copies the owner does not prevent --'
command grep -r '>= 64\|% 64' ring_mpsc/src/lib.rs ring_cursor/src/lib.rs
```

Live output:

```
  -- the family newtypes, and whether the field is public --
pub struct AtomicSeq(AtomicU64);
pub struct Budget(usize);
pub struct Capacity(usize);
pub struct PaddedCursor(CacheAligned<AtomicSeq>);
pub struct Seq(pub u64);
pub struct SlotIndex(pub usize);
  -- the one the pattern is named for, which the shape above does not match --
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(align(64))]
pub struct CacheAligned<T>(T);
  -- declared owners of the line size, family-wide --
ring_align/src/lib.rs:1
  -- copies the owner does not prevent --
ring_mpsc/src/lib.rs:        claim.abs_diff(consume) >= 64
ring_cursor/src/lib.rs:    /// assert_eq!(cursor.addr() % 64, 0, "a 64-aligned value starts on a line boundary");
```

The second command is not a convenience: the first pattern is the family's
newtype shape and `CacheAligned` does not match it, so the exemplar has to be
fetched by name. That mismatch is itself the finding recorded as AL38.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL37 | The family newtypes | n/a — inconsistency | Six newtypes, and two of them — `Seq` and `SlotIndex` — expose the field the other four hide. Both exposed ones are plain integers over which no invariant is claimed, so the split is defensible; nothing declares it, and a seventh newtype has no rule to follow |
| AL38 | `CacheAligned< T >` | n/a — observation | The pattern's own exemplar is the one member the family's newtype shape does not match: it is the only generic one, so a grep written for `pub struct Name( … )` finds all six others and misses the type the pattern is named after. A pattern whose canonical instance is invisible to the census of its own kind is worth stating outright |
| AL39 | The owner | n/a — observation | Exactly one declaration of the line size across 33 crates, which is the pattern working — set against two live copies the ownership cannot reach. What single ownership buys is a grep that finds the copies, not their absence, and this block is that grep |
| AL40 | `PaddedCursor` | n/a — observation | The only newtype in the family that wraps another crate's newtype rather than a primitive, and therefore the only place the layout pattern composes: `PaddedCursor( CacheAligned< AtomicSeq > )` stacks three names to say "an atomic sequence number on its own cache line", and the padding survives the stacking only because each layer is transparent |
