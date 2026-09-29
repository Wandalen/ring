# data_structure

What this crate operates on. It declares no structure of its own, so both
instances concern structure that lives elsewhere.

### Overview Table

| ID | Name | Subject |
|----|------|---------|
| 001 | [The Crate That Declares No Type](001_the_crate_that_declares_no_type.md) | Why `Seq` and `Capacity` live in `ring_types` and what that buys |
| 002 | [The Slice `slowest` Reads](002_the_slice_that_slowest_reads.md) | `&[ Seq ]` — the one aggregate the crate touches, and what its shape costs |

### Both Are About Structure This Crate Refused to Own

001 is the refusal in the large: no newtype for a free-slot count, no wrapper for
a lap count, no `Positions` collection type. Five functions over two borrowed
vocabulary types.

002 is the refusal in the small: `slowest` takes a slice rather than owning or
building a collection — and that single parameter type is what forces a heap
allocation one tier up, in the condition of a lock-free retry loop.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ9 | Declaring no type | n/a — observation | Declaring no type is what keeps the crate free of the fork risk that hit `ring_align`'s constant — there is nothing to copy |
| SQ10 | The crate's whole body | n/a — observation | Five functions reduce to four calls of `Seq::distance_to` and one `.min()`, so the crate borrows all of its arithmetic and contributes only the framing |
| SQ11 | `&[ Seq ]` | n/a — observation | `&[ Seq ]` cannot be produced from `&[ PaddedCursor ]` without materialising the loads — the whole origin of `ring_cursor`'s per-call `Vec`, which the caller resolved by ceasing to call rather than by materialising |
| SQ12 | The slice parameter | n/a — coverage | `slowest` is tested with at most three elements and never with a set larger than a single cache line's worth, while the wrapper that faces real gating sets is one crate up |

### Regenerate

The three counts that make this a crate with no nouns:

```sh
cd "$(git rev-parse --show-toplevel)"
for k in 'pub struct' 'pub enum' 'pub const'; do
  printf '%-12s %s\n' "$k" "$( command grep -c "^$k" ring_seqno/src/lib.rs || true )"
done
```

Live output:

```
pub struct   0
pub enum     0
pub const    0
```
