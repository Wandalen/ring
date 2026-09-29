# api

The crate's public surface — what it exports, and the one inconsistency inside it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Five Functions and No Types](001_five_functions_and_no_types.md) | The whole surface, and what a type-free crate can and cannot promise |
| 002 | [The Argument Order Split](002_the_argument_order_split.md) | `laps_between` takes its arguments in the reverse order of the other three |

### The Surface in One Command

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^pub (const )?fn |^pub (struct|enum|trait|const|type) ' ring_seqno/src/lib.rs
```

Live output:

```
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
pub fn pending( producer : Seq, consumer : Seq ) -> u64
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
```

Five `pub fn`. No `struct`, no `enum`, no `trait`, no `const`, no `type` alias,
no re-export. That is unusually narrow even for a tier-1 crate, and instance 001
argues it is the right shape here while noting the two things it gives up.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ5 | `const fn` | n/a — unadopted | Four of the five functions are const-legal and none is declared `const fn`, while all twelve `pub fn` in `ring_types` — the crate directly below — are |
| SQ6 | `#[ must_use ]` | n/a — observation | All five functions carry `#[ must_use ]`, one of exactly three crates in the family at full coverage, and the crate has no `const fn` at all — the most disciplined about return values and the least about compile-time evaluation |
| SQ7 | Lint attributes | n/a — unenforced | `#![ deny( missing_docs ) ]` is the crate's only lint attribute, so nothing forbids `unsafe` and the crate's zero-`unsafe` property is a fact about the text rather than a guarantee about the future |
| SQ8 | Argument order | **latent hazard** | `laps_between( earlier, later, … )` reverses the order of `may_claim`, `free_slots` and `pending`, all of which take the later position first, and `seq_test.rs:143-145` uses both conventions on three adjacent lines |
