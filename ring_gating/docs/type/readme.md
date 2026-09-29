# type

One struct, two fields, and no trait implementations written by hand. Everything
this crate's type system does, it does through types declared somewhere else.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A `usize` Headroom and a `u64` Limit](001_a_usize_headroom_and_a_u64_limit.md) | Why a count and a position are different widths, and the two conversions between them |
| 002 | [`Send` and `Sync` Without `unsafe`](002_send_and_sync_without_unsafe.md) | The four-wrapper chain that makes `GatingSet` shareable, ending in the standard library |

### The Type

```rust
#[ derive( Debug ) ]
pub struct GatingSet
{
  cursors : Vec< PaddedCursor >,
  capacity : Capacity,
}
```

| Element | Declared in | Kind |
|---------|-------------|------|
| `PaddedCursor` | `ring_cursor` | `PaddedCursor( CacheAligned< AtomicSeq > )` |
| `Capacity` | `ring_types` | `Capacity( usize )`, validated on construction |
| `Seq` | `ring_types` | `Seq( pub u64 )` — the only public field in the chain |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c 'impl.*for GatingSet' ring_gating/src/lib.rs   # 0
grep -n 'derive' ring_gating/src/lib.rs                # one: Debug
```

**No trait is implemented for `GatingSet`, by hand or by derive, except `Debug`.**
Not `Default` — there is no sensible capacity. Not `Clone` — cloning a gating set
would silently fork the cursors a producer gates on. Every other property the
type has is inferred.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT51 | The width of a count | n/a — inconsistency | `ring_gating` and `ring_barrier` disagree on it: `GatingSet::admits` takes `usize`, `Barrier::admits` takes `u64`, and neither doc comment mentions the other side |
| GT52 | Where the widths meet | n/a — observation | In `limit`, the crate's only `as` conversion — a widening on every target the workspace builds for, and the single line where the disagreement in GT51 costs anything |
| GT53 | The `Sync` chain | n/a — observation | It crosses four crates and four wrapper types without one line of `unsafe` — and two of those crates document why they no longer need any |
| GT54 | What asserts `Send` and `Sync` | n/a — coverage | Nothing. Neither trait is named anywhere in this crate or its tests, and both are inherited from `AtomicU64` three crates down. A static assertion would need no `unsafe` and no allowlist entry, so the absence is a gap rather than a constraint the crate is under |
